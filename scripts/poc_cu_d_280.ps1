# CU-D-280: MCP tools/list exposes desktop hover, wait value, session abort.
# ASCII-only. Binary stdin file avoids PowerShell StreamWriter UTF-16/BOM.
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-280: not Windows_NT"
  exit 0
}

$root = (Get-Location).Path
$bin = Join-Path $root "target\debug"
$mcp = Join-Path $bin "vcu-mcp.exe"
$vcu = Join-Path $bin "vcu.exe"
if (-not (Test-Path $mcp)) { throw "missing vcu-mcp.exe under target/debug" }
if (-not (Test-Path $vcu)) { throw "missing vcu.exe under target/debug" }
$ud = Join-Path $env:TEMP ("vcu-280-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $ud | Out-Null
& $vcu --user-dir $ud init --json | Out-Null

$msg = '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
$enc = New-Object System.Text.UTF8Encoding $false
$body = $enc.GetBytes($msg)
$prefix = $enc.GetBytes("Content-Length: $($body.Length)")
$crlf = [byte[]](13, 10)
$ms = New-Object System.IO.MemoryStream
$ms.Write($prefix, 0, $prefix.Length)
$ms.Write($crlf, 0, $crlf.Length)
$ms.Write($crlf, 0, $crlf.Length)
$ms.Write($body, 0, $body.Length)
$frame = $ms.ToArray()

$inPath = Join-Path $ud "in.bin"
$outPath = Join-Path $ud "out.bin"
$errPath = Join-Path $ud "err.txt"
[System.IO.File]::WriteAllBytes($inPath, $frame)

$p = Start-Process -FilePath $mcp -ArgumentList @("--user-dir", $ud) -WorkingDirectory $root -RedirectStandardInput $inPath -RedirectStandardOutput $outPath -RedirectStandardError $errPath -Wait -PassThru -WindowStyle Hidden
$outBytes = @(0)
if (Test-Path $outPath) { $outBytes = [System.IO.File]::ReadAllBytes($outPath) }
$out = $enc.GetString($outBytes)
$err = ""
if (Test-Path $errPath) { $err = [System.IO.File]::ReadAllText($errPath) }
if ($p.ExitCode -ne 0) {
  throw "mcp tools/list exit=$($p.ExitCode) stdout=[$out] stderr=[$err]"
}
if ($out -notlike "*Content-Length:*") {
  throw "mcp tools/list missing Content-Length stdout=[$out] stderr=[$err]"
}
$nl = $out.IndexOf("`r`n`r`n")
if ($nl -lt 0) { $nl = $out.IndexOf("`n`n") }
if ($nl -lt 0) { throw "mcp tools/list missing header break stdout=[$out]" }
$bodyText = $out.Substring($nl).Trim()
$parsed = $bodyText | ConvertFrom-Json
$tools = @($parsed.result.tools)
$names = @()
foreach ($t in $tools) { $names += [string]$t.name }
if ($names -notcontains "vcu_hover") { throw "vcu_hover missing from MCP tools names=$($names -join ',')" }
if ($names -notcontains "vcu_session_abort") { throw "vcu_session_abort missing from MCP tools names=$($names -join ',')" }
if ($names -notcontains "vcu_wait") { throw "vcu_wait missing from MCP tools names=$($names -join ',')" }
$wait = $null
foreach ($t in $tools) {
  if ([string]$t.name -eq "vcu_wait") { $wait = $t; break }
}
$props = $wait.inputSchema.properties
if ($null -eq $props.value) { throw "vcu_wait schema missing value" }
Write-Host "TOOLS_OK hover=vcu_hover abort=vcu_session_abort wait.value=true"
Write-Host "CU-D-280 OK"
