# CU-D-360: MCP tools/call live vcu_screenshot PrintWindow PNG on Notepad.
# ASCII-only. File stdin for MCP frames. No OS cursor warp or HID inject.
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-360: not Windows_NT"
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
$ud = Join-Path $env:TEMP ("vcu-360-" + [guid]::NewGuid().ToString())
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

function Clip([string]$s) {
  if ($s.Length -gt 400) { return $s.Substring(0, 400) }
  return $s
}

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

  $initMsg = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"vcu-360","version":"0"}}}'
  $shotMsg = '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"vcu_screenshot","arguments":{"session":"' + (Esc $sid) + '","tab_id":"' + (Esc $tab) + '"}}}'
  $abortMsg = '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"vcu_session_abort","arguments":{"session":"' + (Esc $sid) + '"}}}'

  $ms = New-Object System.IO.MemoryStream
  Add-McpFrame $ms $initMsg
  Add-McpFrame $ms $shotMsg
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
    throw "mcp exit=$($p.ExitCode) stdout=[$(Clip $out)] stderr=[$(Clip $err)]"
  }
  if ($out -notlike "*vcu-mcp*") { throw "initialize missing vcu-mcp stdout=[$(Clip $out)] stderr=[$(Clip $err)]" }
  if ($out -notlike "*image/png*") { throw "screenshot missing image/png stdout=[$(Clip $out)] stderr=[$(Clip $err)]" }
  if ($out -notlike "*width*") { throw "screenshot missing width stdout=[$(Clip $out)]" }
  if ($out -notlike "*height*") { throw "screenshot missing height stdout=[$(Clip $out)]" }
  if ($out -notlike "*bytes*") { throw "screenshot missing bytes stdout=[$(Clip $out)]" }
  Write-Host "SHOT_OK mime=image/png via=mcp_tools_call"
  Write-Host "CU-D-360 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  if ($notepad -and -not $notepad.HasExited) {
    Stop-Process -Id $notepad.Id -Force -ErrorAction SilentlyContinue
  }
}
