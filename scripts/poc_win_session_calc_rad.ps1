# CU-WIN-SESSION-017: sin(pi) differs in degree and radian mode, then restore both modes.
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
function Open-Nav($sid) {
  $refs = Get-Refs $sid
  if (@($refs | Where-Object { $_.name -eq "关闭导航" }).Count -eq 1) { return "open" }
  $nav = Press-Named $sid "打开导航"
  if ($nav -ne "uia_invoke") { throw "nav path $nav" }
  return "opened"
}
function Close-Nav($sid) {
  $refs = Get-Refs $sid
  if (@($refs | Where-Object { $_.name -eq "关闭导航" }).Count -eq 1) {
    Press-Named $sid "关闭导航" | Out-Null
  }
}
function Select-Mode($sid, $item) {
  Open-Nav $sid | Out-Null
  $refs = Get-Refs $sid
  $btn = @($refs | Where-Object { $_.name -eq $item } | Select-Object -First 1)
  if ($btn.Count -ne 1) {
    Close-Nav $sid
    return "already"
  }
  $click = (& $Vcu click --session $sid --ref $btn[0].ref | Out-String) | ConvertFrom-Json
  if ($click.ok -ne $true) { throw "click failed: $item" }
  $detail = $click.data.detail
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used on $item" }
  if ($detail.input_path -ne "selection_item") { throw "mode path $($detail.input_path) for $item" }
  Start-Sleep -Milliseconds 350
  Close-Nav $sid
  return "switched"
}
function Invoke-SinPi($sid) {
  $pi = Press-Named $sid "π"
  if ($pi -ne "uia_invoke") { throw "pi path $pi" }
  $trig = Press-Named $sid "三角学"
  if ($trig -ne "toggle") { throw "trig path $trig" }
  $sin = Press-Named $sid "正弦"
  if ($sin -ne "uia_invoke") { throw "sin path $sin" }
  $shown = Get-Display $sid
  $refs = Get-Refs $sid
  if (@($refs | Where-Object { $_.name -eq "三角学" }).Count -eq 1) {
    Press-Named $sid "三角学" | Out-Null
  }
  return $shown
}
function Clear-Display($sid) {
  foreach ($name in @("清除","清除条目")) {
    $refs = Get-Refs $sid
    if (@($refs | Where-Object { $_.name -eq $name }).Count -eq 1) {
      Press-Named $sid $name | Out-Null
      return
    }
  }
  throw "clear button missing"
}
function Test-RadianLike($text) {
  if ($text -eq "显示为 0") { return $true }
  if ($text -match "e-1[0-9]") { return $true }
  if ($text -like "显示为 0.000*") { return $true }
  return $false
}
function Test-DegreeLike($text) {
  return $text -like "显示为 0.0548*"
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
  Start-Sleep -Milliseconds 500
  Select-Mode $sid "科学 计算器" | Out-Null
  $first = Invoke-SinPi $sid
  Clear-Display $sid
  $toggle = $null
  for ($n = 0; $n -lt 8 -and -not $toggle; $n++) {
    $refs = Get-Refs $sid
    $btn = @($refs | Where-Object { $_.name -match "度" -and $_.role -like "*Button*" } | Select-Object -First 1)
    if ($btn.Count -eq 1) {
      $click = (& $Vcu click --session $sid --ref $btn[0].ref | Out-String) | ConvertFrom-Json
      if ($click.ok -ne $true) { throw "degree toggle click failed: $($btn[0].name)" }
      $detail = $click.data.detail
      if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
      $toggle = "$($btn[0].name):$($detail.input_path)"
      break
    }
    Start-Sleep -Milliseconds 200
  }
  if (-not $toggle) {
    $names = @((Get-Refs $sid) | ForEach-Object { $_.name }) -join " | "
    throw "degree toggle missing: $names"
  }
  $second = Invoke-SinPi $sid
  if ($first -eq $second) { throw "angle mode did not change: $first" }
  $rad = $null
  $deg = $null
  if (Test-RadianLike $first) { $rad = $first } elseif (Test-DegreeLike $first) { $deg = $first }
  if (Test-RadianLike $second) { $rad = $second } elseif (Test-DegreeLike $second) { $deg = $second }
  if (-not $rad -or -not $deg) { throw "expected radian and degree results, got $first | $second" }
  Clear-Display $sid
  $refs = Get-Refs $sid
  $btn = @($refs | Where-Object { $_.name -match "度" -and $_.role -like "*Button*" } | Select-Object -First 1)
  if ($btn.Count -eq 1) { Press-Named $sid $btn[0].name | Out-Null }
  Select-Mode $sid "标准 计算器" | Out-Null
  $trigBtn = @((Get-Refs $sid) | Where-Object { $_.name -eq "三角学" })
  if ($trigBtn.Count -ne 0) { throw "failed to restore standard mode" }
  $hostTry = & $Vcu app snapshot "win:ApplicationFrameHost:$($cid.Split(':')[-1])" --json
  if ($LASTEXITCODE -eq 0) { throw "host process was allowlisted" }
  Write-Host "CU-WIN-SESSION-017 OK $cid $sid degree=$deg radian=$rad restored"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  Stop-Process -Name CalculatorApp -Force -ErrorAction SilentlyContinue
}