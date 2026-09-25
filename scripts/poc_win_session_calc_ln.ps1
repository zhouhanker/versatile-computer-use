# CU-WIN-SESSION-013: scientific ln(e) = 1, then restore standard mode.
# No OS cursor. Does not allowlist ApplicationFrameHost.
# Refuses if Calculator is already open.
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
  for ($n = 0; $n -lt 8; $n++) {
    $hit = @((Get-Refs $sid) | Where-Object { $_.name -like "显示为*" } | Select-Object -First 1)
    if ($hit.Count -eq 1) { return $hit[0].name }
    Start-Sleep -Milliseconds 200
  }
  throw "display missing"
}
function Press-Named($sid, $name) {
  for ($n = 0; $n -lt 8; $n++) {
    $refs = Get-Refs $sid
    $btn = @($refs | Where-Object { $_.name -eq $name } | Select-Object -First 1)
    if ($btn.Count -eq 1) {
      $click = (& $Vcu click --session $sid --ref $btn[0].ref | Out-String) | ConvertFrom-Json
      if ($click.ok -ne $true) { throw "click failed: $name" }
      $detail = $click.data.detail
      if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used on $name" }
      Start-Sleep -Milliseconds 280
      return $detail.input_path
    }
    Start-Sleep -Milliseconds 200
  }
  throw "not found: $name"
}
function Select-Mode($sid, $item) {
  $nav = Press-Named $sid "打开导航"
  if ($nav -ne "uia_invoke") { throw "nav path $nav" }
  $path = Press-Named $sid $item
  if ($path -ne "selection_item") { throw "mode path $path for $item" }
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
  Select-Mode $sid "科学 计算器"
  if ((Get-Display $sid) -ne "显示为 0") { throw "display not zero" }
  $ePath = Press-Named $sid "Euler 数字"
  if ($ePath -ne "uia_invoke") { throw "euler path $ePath" }
  $lnPath = Press-Named $sid "自然对数"
  if ($lnPath -ne "uia_invoke") { throw "ln path $lnPath" }
  $shown = Get-Display $sid
  if ($shown -ne "显示为 1") { throw "ln(e) display $shown" }
  Select-Mode $sid "标准 计算器"
  $lnBtn = @((Get-Refs $sid) | Where-Object { $_.name -eq "自然对数" })
  if ($lnBtn.Count -ne 0) { throw "failed to restore standard mode" }
  $hostTry = & $Vcu app snapshot "win:ApplicationFrameHost:$($cid.Split(':')[-1])" --json
  if ($LASTEXITCODE -eq 0) { throw "host process was allowlisted" }
  Write-Host "CU-WIN-SESSION-013 OK $cid $sid ln(e)=1 restored"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  Stop-Process -Name CalculatorApp -Force -ErrorAction SilentlyContinue
}