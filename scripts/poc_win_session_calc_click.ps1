
# CU-WIN-SESSION-002: session click presses Calculator seven via UIA, not the OS cursor.
# Does not allowlist ApplicationFrameHost. Does not click Edge Allow.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "debug vcu missing: $Vcu" }
Start-Process calc.exe | Out-Null
$cid = $null
for ($i = 0; $i -lt 15; $i++) {
  Start-Sleep -Milliseconds 400
  $raw = & $Vcu app windows --json | Out-String
  $obj = $raw | ConvertFrom-Json
  $hit = @($obj.data.windows | Where-Object { $_.bundle_or_exe -eq "Calculator" } | Select-Object -First 1)
  if ($hit.Count -eq 1 -and $hit[0].id) { $cid = $hit[0].id; break }
}
if (-not $cid) { throw "calculator window not listed" }
$sid = $null
try {
  $start = (& $Vcu session start --surface desktop --app-id $cid | Out-String) | ConvertFrom-Json
  if ($start.data.stage_presenter -ne "winforms") { throw "presenter" }
  $sid = $start.data.session_id
  $snap = (& $Vcu snapshot --session $sid | Out-String) | ConvertFrom-Json
  $refs = @($snap.data.dom_refs)
  $seven = @($refs | Where-Object { $_.name -eq "七" -and $_.role -like "*Button*" } | Select-Object -First 1)
  if ($seven.Count -ne 1) { throw "digit seven not found" }
  $before = @($refs | Where-Object { $_.name -like "显示为*" } | Select-Object -First 1)
  if ($before.Count -ne 1 -or $before[0].name -ne "显示为 0") { throw "display not zero: $($before[0].name)" }
  $click = (& $Vcu click --session $sid --ref $seven[0].ref | Out-String) | ConvertFrom-Json
  if ($click.ok -ne $true) { throw "click failed" }
  $detail = $click.data.detail
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  if ($detail.input_path -ne "uia_invoke") { throw "path $($detail.input_path)" }
  Start-Sleep -Milliseconds 400
  $snap2 = (& $Vcu snapshot --session $sid | Out-String) | ConvertFrom-Json
  $after = @($snap2.data.dom_refs | Where-Object { $_.name -like "显示为*" } | Select-Object -First 1)
  if ($after.Count -ne 1 -or $after[0].name -ne "显示为 7") { throw "display $($after[0].name)" }
  $hostTry = & $Vcu app snapshot "win:ApplicationFrameHost:$($cid.Split(':')[-1])" --json
  if ($LASTEXITCODE -eq 0) { throw "host process was allowlisted" }
  Write-Host "CU-WIN-SESSION-002 OK $cid $sid $($seven[0].ref) 0->7"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  Stop-Process -Name CalculatorApp -Force -ErrorAction SilentlyContinue
}
