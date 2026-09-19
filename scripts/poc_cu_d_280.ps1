# CU-D-280: MCP tools/list exposes desktop hover, wait value, session abort.
# ASCII-only for Windows PowerShell 5.1 -File.
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
$psi.Arguments = "--user-dir $ud"
$psi.WorkingDirectory = $root
$psi.UseShellExecute = $false
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.CreateNoWindow = $true
$psi.StandardOutputEncoding = [System.Text.UTF8Encoding]::new($false)
$psi.StandardErrorEncoding = [System.Text.UTF8Encoding]::new($false)
$proc = New-Object System.Diagnostics.Process
$proc.StartInfo = $psi
[void]$proc.Start()

$msg = '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
$enc = New-Object System.Text.UTF8Encoding $false
$body = $enc.GetBytes($msg)
$header = $enc.GetBytes(("Content-Length: {0}`r`n`r`n" -f $body.Length))
# Write the whole LSP/MCP frame to the raw pipe. Do not mix StreamWriter with BaseStream:
# StreamWriter buffers the header and can emit it AFTER the JSON body.
$stdin = $proc.StandardInput.BaseStream
$stdin.Write($header, 0, $header.Length)
$stdin.Write($body, 0, $body.Length)
$stdin.Flush()
$proc.StandardInput.Close()

$errTask = $proc.StandardError.ReadToEndAsync()
$out = $proc.StandardOutput.ReadToEnd()
$err = $errTask.Result
if (-not $proc.WaitForExit(15000)) {
  try { $proc.Kill() } catch {}
  throw "mcp tools/list timeout stderr=[$err]"
}
if ($proc.ExitCode -ne 0) {
  throw "mcp tools/list exit=$($proc.ExitCode) stdout=[$out] stderr=[$err]"
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
