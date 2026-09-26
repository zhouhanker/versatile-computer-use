# CU-WIN-SESSION-049: session presses the owned numeric up-down arrows without moving the cursor.
# WM_LBUTTONDOWN/UP on the spinner button halves. Not SendInput. Not number_set. Cursor must not move.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor049 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor049]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-spin-049-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-SPIN-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 360, 180',
  '$num = New-Object System.Windows.Forms.NumericUpDown',
  '$num.Minimum = 0',
  '$num.Maximum = 11',
  '$num.Value = 10',
  '$num.Bounds = New-Object System.Drawing.Rectangle 16, 48, 120, 28',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-SPIN-OLD"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 16, 12, 280, 28',
  '$num.Add_ValueChanged({ $label.Text = "VCU-SPIN-" + $num.Value.ToString([Globalization.CultureInfo]::InvariantCulture) })',
  '$form.Controls.Add($label)',
  '$form.Controls.Add($num)',
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
  $found = (& $Vcu wait --session $sid --name "VCU-SPIN-HOST" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "spin host not found" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "spin:up" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "spin up failed $($typed.error)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "spin_up") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-SPIN-11" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "up event did not run" }
  $top = (& $Vcu type --session $sid --ref $ref --text "spin:up" | Out-String) | ConvertFrom-Json
  if ($top.ok -eq $true) { throw "up at maximum was accepted" }
  $down = (& $Vcu type --session $sid --ref $ref --text "spin:down" | Out-String) | ConvertFrom-Json
  if ($down.ok -ne $true) { throw "spin down failed $($down.error)" }
  if ($down.data.detail.input_path -ne "spin_down") { throw "down path $($down.data.detail.input_path)" }
  if ($down.data.detail.os_cursor_used -ne $false -or $down.data.detail.hid_injected -ne $false) { throw "down cursor or hid used" }
  $back = (& $Vcu wait --session $sid --name "VCU-SPIN-10" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($back.ok -ne $true) { throw "down event did not run" }
  $nope = (& $Vcu type --session $sid --ref $ref --text "spin:side" | Out-String) | ConvertFrom-Json
  if ($nope.ok -eq $true) { throw "bad direction was accepted" }
  $cursor1 = [VcuCursor049]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-049 OK $cid $sid ref=$ref path=spin_up value=11 cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
