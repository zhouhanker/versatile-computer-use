# CU-WIN-SESSION-027: session click flips an owned checkbox, and the scene reads the state.
# A push button is not labeled as a checkbox. No SendInput. Does not move the OS cursor.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor027 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor027]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-check-027-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-CHECK-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 200, 180, 460, 200',
  '$btn = New-Object System.Windows.Forms.Button',
  '$btn.Text = "VCU-CHECK-BTN"',
  '$btn.Bounds = New-Object System.Drawing.Rectangle 24, 16, 160, 28',
  '$box = New-Object System.Windows.Forms.CheckBox',
  '$box.Text = "VCU-CHECK-027"',
  '$box.Checked = $false',
  '$box.Bounds = New-Object System.Drawing.Rectangle 24, 60, 220, 28',
  '$form.Controls.Add($btn)',
  '$form.Controls.Add($box)',
  '[System.Windows.Forms.Application]::Run($form)'
) | Set-Content -Encoding ASCII -Path $formScript
$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru
$owned = $proc.Id
$sid = $null
try {
  $cid = $null
  for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Milliseconds 200
    $raw = & $Vcu app windows --json | Out-String
    $hit = @((($raw | ConvertFrom-Json).data.windows) | Where-Object { $_.id -eq "win:powershell:$owned" })
    if ($hit.Count -eq 1) { $cid = $hit[0].id; break }
  }
  if (-not $cid) { throw "owned window not listed" }
  $start = (& $Vcu session start --surface desktop --app-id $cid | Out-String) | ConvertFrom-Json
  if ($start.ok -ne $true -or $start.data.stage_hud -ne $true) { throw "stage not raised" }
  $sid = $start.data.session_id
  $btn = (& $Vcu extract --session $sid --selector "VCU-CHECK-BTN" | Out-String) | ConvertFrom-Json
  $btnVal = [string]@($btn.data.matches | Select-Object -First 1).value
  if ($btnVal -match "toggle-") { throw "button was labeled as a checkbox: $btnVal" }
  $off = (& $Vcu wait --session $sid --name "VCU-CHECK-027" --value "toggle-off" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($off.ok -ne $true) { throw "unchecked state not visible" }
  $ref = [string]$off.data.detail.found_ref
  $click = (& $Vcu click --session $sid --ref $ref | Out-String) | ConvertFrom-Json
  if ($click.ok -ne $true) { throw "click failed" }
  $detail = $click.data.detail
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $on = (& $Vcu wait --session $sid --name "VCU-CHECK-027" --value "toggle-on" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($on.ok -ne $true) { throw "checked state not visible path=$($detail.input_path)" }
  $back = (& $Vcu click --session $sid --ref $ref | Out-String) | ConvertFrom-Json
  if ($back.ok -ne $true) { throw "second click failed" }
  $off2 = (& $Vcu wait --session $sid --name "VCU-CHECK-027" --value "toggle-off" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($off2.ok -ne $true) { throw "did not toggle back off" }
  $cursor1 = [VcuCursor027]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-027 OK $cid $sid ref=$ref path=$($detail.input_path) on-then-off cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
