# CU-D-280: MCP tools/list exposes desktop hover, wait value, session abort.
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

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $mcp
$psi.Arguments = "--user-dir `"$ud`""
$psi.UseShellExecute = $false
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.CreateNoWindow = $true
$proc = New-Object System.Diagnostics.Process
$proc.StartInfo = $psi
[void]$proc.Start()
$msg = '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
$enc = New-Object System.Text.UTF8Encoding $false
$bytes = $enc.GetBytes($msg)
$header = "Content-Length: $($bytes.Length)`r`n`r`n"
$proc.StandardInput.Write($header)
$proc.StandardInput.BaseStream.Write($bytes, 0, $bytes.Length)
$proc.StandardInput.Flush()
$proc.StandardInput.Close()
$out = $proc.StandardOutput.ReadToEnd()
[void]$proc.WaitForExit(15000)
Write-Host $out
if ($out -notlike "*Content-Length:*") { throw "mcp tools/list missing Content-Length" }
$nl = $out.IndexOf("`r`n`r`n")
if ($nl -lt 0) { $nl = $out.IndexOf("`n`n") }
if ($nl -lt 0) { throw "mcp tools/list missing header break" }
$body = $out.Substring($nl).Trim()
$parsed = $body | ConvertFrom-Json
$tools = @($parsed.result.tools)
$names = @()
foreach ($t in $tools) { $names += [string]$t.name }
if ($names -notcontains "vcu_hover") { throw "vcu_hover missing from MCP tools" }
if ($names -notcontains "vcu_session_abort") { throw "vcu_session_abort missing from MCP tools" }
if ($names -notcontains "vcu_wait") { throw "vcu_wait missing from MCP tools" }
$wait = $null
foreach ($t in $tools) {
  if ([string]$t.name -eq "vcu_wait") { $wait = $t; break }
}
$props = $wait.inputSchema.properties
if ($null -eq $props.value) { throw "vcu_wait schema missing value" }
Write-Host "TOOLS_OK hover=vcu_hover abort=vcu_session_abort wait.value=true"
Write-Host "CU-D-280 OK"
