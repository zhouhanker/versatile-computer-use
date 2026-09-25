
# CU-WIN-SESSION-001: Windows product session binds Calculator and raises the Stage HUD.
# Does not click calculator buttons. Does not move the OS cursor. Does not click Edge Allow.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { $Vcu = "vcu" }
function Invoke-Vcu([string[]]$Args) {
  $p = & $Vcu @Args
  return ($p | Out-String)
}
Start-Process calc.exe | Out-Null
$cid = $null
for ($i = 0; $i -lt 15; $i++) {
  Start-Sleep -Milliseconds 400
  $raw = & $Vcu app windows --json | Out-String
  if ($raw -match '"bundle_or_exe": "Calculator"') {
    $obj = $raw | ConvertFrom-Json
    $hit = @($obj.data.windows | Where-Object { $_.bundle_or_exe -eq "Calculator" } | Select-Object -First 1)
    if ($hit.Count -eq 1) { $cid = $hit[0].id; break }
  }
}
if (-not $cid) { throw "calculator window not listed" }
try {
  $start = & $Vcu session start --surface desktop --app-id $cid | Out-String
  $s = $start | ConvertFrom-Json
  if (-not $s.ok) { throw "session start failed" }
  $d = $s.data
  if ($d.stage_hud -ne $true) { throw "stage_hud not true" }
  if ($d.stage_presenter -ne "winforms") { throw "presenter $($d.stage_presenter)" }
  if ($d.active_app_id -ne $cid) { throw "bound $($d.active_app_id)" }
  if ($d.policy.os_cursor -ne "deny") { throw "os cursor not denied" }
  $sid = $d.session_id
  $abort = & $Vcu session abort $sid | Out-String
  $a = $abort | ConvertFrom-Json
  if ($a.data.aborted -ne $true -or $a.data.hud -ne $false) { throw "abort did not clear hud" }
  Write-Host "CU-WIN-SESSION-001 OK $cid $sid"
} finally {
  Stop-Process -Name CalculatorApp -Force -ErrorAction SilentlyContinue
}
