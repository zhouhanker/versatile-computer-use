# CU-D-260: Guide hover then click still bm_click (no SendInput).
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-260: not Windows_NT"
  exit 0
}

$root = (Get-Location).Path
$bin = Join-Path $root "target\debug"
$vcu = Join-Path $bin "vcu.exe"
$daemonBin = Join-Path $bin "vcu-daemon.exe"
if (-not (Test-Path $vcu)) {
  throw "missing vcu.exe under target/debug; build vcu-cli and vcu-daemon first"
}
$env:Path = "$bin;$env:Path"
$ud = Join-Path $env:TEMP ("vcu-260-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $ud | Out-Null

& $vcu --user-dir $ud init --json | Out-Null
$cfgPath = Join-Path $ud "config.json"
$cfg = Get-Content $cfgPath | ConvertFrom-Json
$cfg.daemon_port = 19000 + (Get-Random -Maximum 1000)
$cfg | ConvertTo-Json | Set-Content $cfgPath

$marker = Join-Path $ud "invoke-ok.txt"
$helperPs1 = Join-Path $ud "helper.ps1"
@(
  'param([string]$Marker)'
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null'
  'Add-Type -AssemblyName System.Drawing | Out-Null'
  '$form = New-Object System.Windows.Forms.Form'
  '$form.Text = "VCU-D-260"'
  '$form.Width = 280'
  '$form.Height = 140'
  '$form.StartPosition = "Manual"'
  '$form.Left = 80'
  '$form.Top = 80'
  '$btn = New-Object System.Windows.Forms.Button'
  '$btn.Name = "VcuCount"'
  '$btn.Text = "VcuCount"'
  '$btn.Width = 120'
  '$btn.Height = 32'
  '$btn.Left = 70'
  '$btn.Top = 40'
  '$script:VcuMarker = $Marker'
  '$btn.Add_Click({ Set-Content -LiteralPath $script:VcuMarker -Value "INVOKE_OK" })'
  '$form.Controls.Add($btn)'
  '[System.Windows.Forms.Application]::Run($form)'
) | Set-Content -LiteralPath $helperPs1 -Encoding ASCII

$sys = Join-Path $env:SystemRoot "System32\WindowsPowerShell\v1.0\powershell.exe"
$helper = Start-Process -FilePath $sys -ArgumentList @("-NoProfile", "-STA", "-File", $helperPs1, "-Marker", $marker) -PassThru
Start-Sleep -Seconds 1
for ($i = 0; $i -lt 20; $i++) {
  $helper.Refresh()
  if ($helper.MainWindowHandle -ne [IntPtr]::Zero) { break }
  Start-Sleep -Milliseconds 200
}

$daemon = $null
try {
  $daemon = Start-Process -FilePath $daemonBin -ArgumentList @("--user-dir", $ud) -PassThru -WindowStyle Hidden
  $ok = $false
  for ($i = 0; $i -lt 30; $i++) {
    Start-Sleep -Milliseconds 200
    & $vcu --user-dir $ud daemon status | Out-Null
    if ($LASTEXITCODE -eq 0) { $ok = $true; break }
  }
  if (-not $ok) { throw "daemon health failed" }

  $tab = "win:powershell:$($helper.Id)"
  $startRaw = & $vcu --user-dir $ud session start --surface desktop --app-id $tab --json
  Write-Host $startRaw
  $start = $startRaw | ConvertFrom-Json
  if (-not $start.ok) { throw "session start failed" }
  $sid = [string]$start.data.session_id
  Write-Host ("STAGE_OK session={0} tab={1}" -f $sid, $tab)

  $snapRaw = & $vcu --user-dir $ud snapshot --session $sid --tab $tab --json
  Write-Host $snapRaw
  $snap = $snapRaw | ConvertFrom-Json
  if (-not $snap.ok) { throw "snapshot failed" }
  $ref = $null
  foreach ($r in @($snap.data.dom_refs)) {
    if ([string]$r.name -eq "VcuCount") { $ref = [string]$r.ref; break }
  }
  if (-not $ref) { throw "no VcuCount button in snapshot" }
  Write-Host ("SNAP_OK ref={0}" -f $ref)

  $hoverPath = Join-Path $ud "hover.json"
  $hover = @{
    type = "hover"
    target = @{ tab_id = $tab; ref = $ref }
    args = @{ tab_id = $tab }
  } | ConvertTo-Json -Compress
  Set-Content -LiteralPath $hoverPath -Value $hover -Encoding ASCII
  $hoverRaw = & $vcu --user-dir $ud act --session $sid --action-json $hoverPath --json
  Write-Host $hoverRaw
  $hovered = $hoverRaw | ConvertFrom-Json
  if (-not $hovered.ok) { throw "hover failed" }
  $hpath = [string]$hovered.data.detail.input_path
  $hcursor = [bool]$hovered.data.detail.os_cursor_used
  if ($hpath -ne "guide_hover") { throw "hover expected guide_hover" }
  if ($hcursor) { throw "hover os_cursor_used true" }
  Write-Host ("HOVER_OK path={0} os_cursor_used={1}" -f $hpath, $hcursor)

  $clickRaw = & $vcu --user-dir $ud click --session $sid --tab $tab --ref $ref --json
  Write-Host $clickRaw
  $clicked = $clickRaw | ConvertFrom-Json
  if (-not $clicked.ok) { throw "click failed" }
  $path = [string]$clicked.data.detail.input_path
  $cursor = [bool]$clicked.data.detail.os_cursor_used
  if ($cursor) { throw "click os_cursor_used true" }
  if ($path -ne "bm_click") { throw "click expected bm_click after hover" }
  $got = $false
  for ($i = 0; $i -lt 20; $i++) {
    if (Test-Path -LiteralPath $marker) {
      $txt = (Get-Content -LiteralPath $marker -Raw).Trim()
      if ($txt -eq "INVOKE_OK") { $got = $true; break }
    }
    Start-Sleep -Milliseconds 150
  }
  if (-not $got) { throw "marker file missing after click" }
  Write-Host ("INVOKE_OK path={0} os_cursor_used={1}" -f $path, $cursor)

  & $vcu --user-dir $ud session abort $sid | Out-Null
  Write-Host "CU-D-260 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  if ($helper -and -not $helper.HasExited) {
    Stop-Process -Id $helper.Id -Force -ErrorAction SilentlyContinue
  }
}
