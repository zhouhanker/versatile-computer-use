# CU-D-300: MCP tools/call live vcu_click still bm_click on a throwaway button.
# ASCII-only. File stdin for MCP frames. No OS cursor warp or HID inject.
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-300: not Windows_NT"
  exit 0
}

$root = (Get-Location).Path
$bin = Join-Path $root "target\debug"
$vcu = Join-Path $bin "vcu.exe"
$daemonBin = Join-Path $bin "vcu-daemon.exe"
$mcp = Join-Path $bin "vcu-mcp.exe"
if (-not (Test-Path $vcu)) { throw "missing vcu.exe under target/debug" }
if (-not (Test-Path $daemonBin)) { throw "missing vcu-daemon.exe under target/debug" }
if (-not (Test-Path $mcp)) { throw "missing vcu-mcp.exe under target/debug" }
$env:Path = "$bin;$env:Path"
$ud = Join-Path $env:TEMP ("vcu-300-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $ud | Out-Null

function Esc([string]$s) {
  return $s.Replace('\', '\\').Replace('"', '\"')
}

function Add-McpFrame([System.IO.MemoryStream]$ms, [string]$json) {
  $enc = New-Object System.Text.UTF8Encoding $false
  $body = $enc.GetBytes($json)
  $prefix = $enc.GetBytes("Content-Length: $($body.Length)")
  $crlf = [byte[]](13, 10)
  $ms.Write($prefix, 0, $prefix.Length)
  $ms.Write($crlf, 0, $crlf.Length)
  $ms.Write($crlf, 0, $crlf.Length)
  $ms.Write($body, 0, $body.Length)
}

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
  '$form.Text = "VCU-D-300"'
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

  $initMsg = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"vcu-300","version":"0"}}}'
  $clickMsg = '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"vcu_click","arguments":{"session":"' + (Esc $sid) + '","ref":"' + (Esc $ref) + '","tab_id":"' + (Esc $tab) + '"}}}'
  $abortMsg = '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"vcu_session_abort","arguments":{"session":"' + (Esc $sid) + '"}}}'

  $ms = New-Object System.IO.MemoryStream
  Add-McpFrame $ms $initMsg
  Add-McpFrame $ms $clickMsg
  Add-McpFrame $ms $abortMsg
  $frame = $ms.ToArray()
  $inPath = Join-Path $ud "in.bin"
  $outPath = Join-Path $ud "out.bin"
  $errPath = Join-Path $ud "err.txt"
  [System.IO.File]::WriteAllBytes($inPath, $frame)

  $p = Start-Process -FilePath $mcp -ArgumentList @("--user-dir", $ud) -WorkingDirectory $root -RedirectStandardInput $inPath -RedirectStandardOutput $outPath -RedirectStandardError $errPath -Wait -PassThru -WindowStyle Hidden
  $enc = New-Object System.Text.UTF8Encoding $false
  $outBytes = [byte[]]@()
  if (Test-Path $outPath) { $outBytes = [System.IO.File]::ReadAllBytes($outPath) }
  $out = $enc.GetString($outBytes)
  $err = ""
  if (Test-Path $errPath) { $err = [System.IO.File]::ReadAllText($errPath) }
  if ($p.ExitCode -ne 0) {
    throw "mcp exit=$($p.ExitCode) stdout=[$out] stderr=[$err]"
  }
  if ($out -notlike "*vcu-mcp*") { throw "initialize missing vcu-mcp stdout=[$out] stderr=[$err]" }
  if ($out -notlike "*bm_click*") { throw "vcu_click missing bm_click stdout=[$out] stderr=[$err]" }
  if ($out -notlike "*os_cursor_used*") { throw "vcu_click missing os_cursor_used stdout=[$out]" }
  $got = $false
  for ($i = 0; $i -lt 20; $i++) {
    if (Test-Path -LiteralPath $marker) {
      $txt = (Get-Content -LiteralPath $marker -Raw).Trim()
      if ($txt -eq "INVOKE_OK") { $got = $true; break }
    }
    Start-Sleep -Milliseconds 150
  }
  if (-not $got) { throw "marker file missing after mcp click stdout=[$out] stderr=[$err]" }
  Write-Host "INVOKE_OK path=bm_click os_cursor_used=False via=mcp_tools_call"
  Write-Host "CU-D-300 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  if ($helper -and -not $helper.HasExited) {
    Stop-Process -Id $helper.Id -Force -ErrorAction SilentlyContinue
  }
}
