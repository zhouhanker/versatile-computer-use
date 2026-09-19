# CU-D-250: Windows Guide hover on Notepad without OS cursor / SendInput.
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-250: not Windows_NT"
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
$ud = Join-Path $env:TEMP ("vcu-250-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $ud | Out-Null

& $vcu --user-dir $ud init --json | Out-Null
$cfgPath = Join-Path $ud "config.json"
$cfg = Get-Content $cfgPath | ConvertFrom-Json
$cfg.daemon_port = 19000 + (Get-Random -Maximum 1000)
$cfg | ConvertTo-Json | Set-Content $cfgPath

$notepad = Start-Process -FilePath "notepad.exe" -PassThru
Start-Sleep -Seconds 1
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

  $tab = "win:notepad:$($notepad.Id)"
  $startRaw = & $vcu --user-dir $ud session start --surface desktop --app-id $tab --json
  Write-Host $startRaw
  $start = $startRaw | ConvertFrom-Json
  if (-not $start.ok) { throw "session start failed" }
  $sid = [string]$start.data.session_id
  Write-Host ("STAGE_OK session={0}" -f $sid)

  $snapRaw = & $vcu --user-dir $ud snapshot --session $sid --tab $tab --json
  Write-Host $snapRaw
  $snap = $snapRaw | ConvertFrom-Json
  if (-not $snap.ok) { throw "snapshot failed" }
  $ref = $null
  foreach ($r in @($snap.data.dom_refs)) {
    if ([string]$r.role -like "*Edit*") { $ref = [string]$r.ref; break }
  }
  if (-not $ref) {
    $first = @($snap.data.dom_refs)[0]
    if ($null -ne $first) { $ref = [string]$first.ref }
  }
  if (-not $ref) { throw "empty notepad scene" }
  Write-Host ("SNAP_OK ref={0}" -f $ref)

  $actionPath = Join-Path $ud "hover.json"
  $action = @{
    type = "hover"
    target = @{ tab_id = $tab; ref = $ref }
    args = @{ tab_id = $tab }
  } | ConvertTo-Json -Compress
  Set-Content -LiteralPath $actionPath -Value $action -Encoding ASCII
  $hoverAt = Get-Date
  $hoverRaw = & $vcu --user-dir $ud act --session $sid --action-json $actionPath --json
  Write-Host $hoverRaw
  $hovered = $hoverRaw | ConvertFrom-Json
  if (-not $hovered.ok) { throw "hover failed" }
  $path = [string]$hovered.data.detail.input_path
  $cursor = [bool]$hovered.data.detail.os_cursor_used
  $hid = [bool]$hovered.data.detail.hid_injected
  $overlay = [bool]$hovered.data.detail.guide.overlay
  if ($path -ne "guide_hover") { throw "unexpected input_path" }
  if ($cursor) { throw "os_cursor_used true" }
  if ($hid) { throw "hid_injected true" }
  if (-not $overlay) { throw "guide overlay not true" }
  Write-Host ("HOVER_OK path={0} overlay={1} os_cursor_used={2}" -f $path, $overlay, $cursor)

  $guideFile = $null
  for ($i = 0; $i -lt 20; $i++) {
    $cands = @(Get-ChildItem -LiteralPath $env:TEMP -Filter "vcu-stage-*.json" -ErrorAction SilentlyContinue |
      Where-Object { $_.Name -notlike "*.tmp" -and $_.LastWriteTime -ge $hoverAt.AddSeconds(-2) })
    foreach ($f in $cands) {
      try {
        $ctl = Get-Content -LiteralPath $f.FullName -Raw | ConvertFrom-Json
      } catch { continue }
      if ($null -eq $ctl.guide) { continue }
      $gx = [double]$ctl.guide.x
      $gy = [double]$ctl.guide.y
      if ($ctl.guide.visible -eq $true) {
        $guideFile = $f
        Write-Host ("GUIDE_FILE_OK x={0} y={1}" -f $gx, $gy)
        break
      }
    }
    if ($guideFile) { break }
    Start-Sleep -Milliseconds 100
  }
  if (-not $guideFile) { throw "stage control json missing visible guide" }

  & $vcu --user-dir $ud session abort $sid | Out-Null
  Write-Host "CU-D-250 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  if ($notepad -and -not $notepad.HasExited) {
    Stop-Process -Id $notepad.Id -Force -ErrorAction SilentlyContinue
  }
}
