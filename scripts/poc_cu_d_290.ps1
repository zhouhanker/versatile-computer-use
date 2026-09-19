# CU-D-290: MCP tools/call live vcu_hover + vcu_session_abort on Windows Notepad.
# ASCII-only. File stdin for MCP frames. No OS cursor warp or HID inject.
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-290: not Windows_NT"
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
$ud = Join-Path $env:TEMP ("vcu-290-" + [guid]::NewGuid().ToString())
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

function Get-McpBodies([string]$out) {
  $bodies = New-Object System.Collections.ArrayList
  $parts = $out -split "Content-Length:"
  foreach ($part in $parts) {
    $p = [string]$part
    $brace = $p.IndexOf("{")
    if ($brace -lt 0) { continue }
    [void]$bodies.Add($p.Substring($brace).Trim())
  }
  return ,$bodies.ToArray()
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

  $initMsg = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"vcu-290","version":"0"}}}'
  $hoverMsg = '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"vcu_hover","arguments":{"session":"' + (Esc $sid) + '","ref":"' + (Esc $ref) + '","tab_id":"' + (Esc $tab) + '"}}}'
  $abortMsg = '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"vcu_session_abort","arguments":{"session":"' + (Esc $sid) + '"}}}'

  $ms = New-Object System.IO.MemoryStream
  Add-McpFrame $ms $initMsg
  Add-McpFrame $ms $hoverMsg
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
  $bodies = @(Get-McpBodies $out)
  if ($bodies.Count -lt 3) {
    throw "mcp expected 3 framed replies got $($bodies.Count) stdout=[$out] stderr=[$err]"
  }

  $initRpc = $bodies[0] | ConvertFrom-Json
  if ([string]$initRpc.result.serverInfo.name -ne "vcu-mcp") {
    throw "initialize missing vcu-mcp"
  }

  $hoverRpc = $bodies[1] | ConvertFrom-Json
  if ([bool]$hoverRpc.result.isError) {
    throw "vcu_hover isError text=$([string]$hoverRpc.result.content[0].text)"
  }
  $hoverText = [string](@($hoverRpc.result.content)[0].text)
  $hovered = $hoverText | ConvertFrom-Json
  if (-not $hovered.ok) { throw "vcu_hover ok=false $hoverText" }
  $path = [string]$hovered.data.detail.input_path
  $cursor = [bool]$hovered.data.detail.os_cursor_used
  if ($path -ne "guide_hover") { throw "unexpected input_path=$path $hoverText" }
  if ($cursor) { throw "os_cursor_used true $hoverText" }
  Write-Host ("HOVER_OK path={0} os_cursor_used={1} via=mcp_tools_call" -f $path, $cursor)

  $abortRpc = $bodies[2] | ConvertFrom-Json
  if ([bool]$abortRpc.result.isError) {
    throw "vcu_session_abort isError text=$([string]$abortRpc.result.content[0].text)"
  }
  $abortText = [string](@($abortRpc.result.content)[0].text)
  $aborted = $abortText | ConvertFrom-Json
  if (-not $aborted.ok) { throw "vcu_session_abort ok=false $abortText" }
  Write-Host "ABORT_OK via=mcp_tools_call"
  Write-Host "CU-D-290 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  if ($notepad -and -not $notepad.HasExited) {
    Stop-Process -Id $notepad.Id -Force -ErrorAction SilentlyContinue
  }
}
