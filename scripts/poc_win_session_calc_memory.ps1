# CU-WIN-SESSION-007: store 5 in calculator memory, clear the display, recall 5.
# No OS cursor. Does not allowlist ApplicationFrameHost.
# Refuses to start if a Calculator window is already open.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "debug vcu missing: $Vcu" }

function Get-CalcId {
  $raw = & $Vcu app windows --json | Out-String
  $obj = $raw | ConvertFrom-Json
  $hit = @($obj.data.windows | Where-Object { $_.bundle_or_exe -eq "Calculator" } | Select-Object -First 1)
  if ($hit.Count -eq 1) { return $hit[0].id }
  return $null
}
function Get-Refs($sid) {
  $snap = (& $Vcu snapshot --session $sid | Out-String) | ConvertFrom-Json
  return @($snap.data.dom_refs)
}
function Get-Display($sid) {
  $hit = @((Get-Refs $sid) | Where-Object { $_.name -like "显示为*" } | Select-Object -First 1)
  if ($hit.Count -ne 1) { throw "display missing" }
  return $hit[0].name
}
function Press-Name($sid, $name) {
  $refs = Get-Refs $sid
  $btn = @($refs | Where-Object { $_.name -eq $name -and $_.role -like "*Button*" } | Select-Object -First 1)
  if ($btn.Count -ne 1) { throw "button not found: $name" }
  $click = (& $Vcu click --session $sid --ref $btn[0].ref | Out-String) | ConvertFrom-Json
  if ($click.ok -ne $true) { throw "click failed: $name" }
  $detail = $click.data.detail
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used on $name" }
  if ($detail.input_path -ne "uia_invoke") { throw "path $($detail.input_path) on $name" }
  Start-Sleep -Milliseconds 280
}

if (Get-CalcId) { throw "a Calculator window is already open; refusing to touch it" }
Start-Process calc.exe | Out-Null
$cid = $null
for ($i = 0; $i -lt 15; $i++) {
  Start-Sleep -Milliseconds 400
  $cid = Get-CalcId
  if ($cid) { break }
}
if (-not $cid) { throw "calculator window not listed" }
$sid = $null
try {
  $start = (& $Vcu session start --surface desktop --app-id $cid | Out-String) | ConvertFrom-Json
  if ($start.data.stage_hud -ne $true -or $start.data.stage_presenter -ne "winforms") { throw "stage not raised" }
  $sid = $start.data.session_id
  if ((Get-Display $sid) -ne "显示为 0") { throw "display not zero" }
  Press-Name $sid "五"
  if ((Get-Display $sid) -ne "显示为 5") { throw "digit not shown" }
  Press-Name $sid "记忆存储"
  Press-Name $sid "清除"
  $cleared = Get-Display $sid
  if ($cleared -ne "显示为 0") { throw "clear left $cleared" }
  Press-Name $sid "记忆调用"
  $recalled = Get-Display $sid
  if ($recalled -ne "显示为 5") { throw "recall $recalled" }
  $hostTry = & $Vcu app snapshot "win:ApplicationFrameHost:$($cid.Split(':')[-1])" --json
  if ($LASTEXITCODE -eq 0) { throw "host process was allowlisted" }
  Write-Host "CU-WIN-SESSION-007 OK $cid $sid MS 5 -> clear -> MR 5"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  Stop-Process -Name CalculatorApp -Force -ErrorAction SilentlyContinue
}
