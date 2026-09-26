# CU-WIN-SESSION-091: button named fraction computes 5 factorial, then restore standard mode.
# No OS cursor. Does not allowlist ApplicationFrameHost.
# Refuses if Calculator is already open.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor091 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor091]::Pos()

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
  $names = @((Get-Refs $sid) | ForEach-Object { $_.name }) -join " | "
  throw "not found: $name :: $names"
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
  if ((Get-Display $sid) -ne "显示为 0") { throw "display not zero" }
  $nine = Press-Named $sid "五"
  if ($nine -ne "uia_invoke") { throw "digit path $nine" }
  if ((Get-Display $sid) -ne "显示为 5") { throw "not 5: $(Get-Display $sid)" }
  $root = Press-Named $sid "分数"
  if ($root -ne "uia_invoke") { throw "fact path $root" }
  $shown = Get-Display $sid
  if ($shown -ne "显示为 120") { throw "fact display $shown" }
  $restored = Select-Mode $sid "标准 计算器"
  if ($restored -ne "switched") { throw "standard mode was not switched: $restored" }
  $gone = 1
  for ($n = 0; $n -lt 8; $n++) {
    Start-Sleep -Milliseconds 250
    $refs = Get-Refs $sid
    $gone = @($refs | Where-Object { $_.name -eq "π" }).Count
    if ($gone -eq 0) { break }
  }
  if ($gone -ne 0) {
    $names = @($refs | ForEach-Object { $_.name }) -join " | "
    throw "failed to restore standard mode :: $names"
  }
  $hostTry = & $Vcu app snapshot "win:ApplicationFrameHost:$($cid.Split(':')[-1])" --json
  if ($LASTEXITCODE -eq 0) { throw "host process was allowlisted" }
  $cursor1 = [VcuCursor091]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-091 OK $cid $sid 5!=120 restored cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  Stop-Process -Name CalculatorApp -Force -ErrorAction SilentlyContinue
}
