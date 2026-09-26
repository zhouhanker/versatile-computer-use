# CU-WIN-SESSION-048: session type sets a decimal on an owned numeric up-down without moving the cursor.
# WM_SETTEXT plus WM_KILLFOCUS on the spinner edit. Decimal readback must match. Not number_set. Not a pixel click. Not SendInput.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor048 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor048]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-dec-048-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-DEC-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 360, 180',
  '$num = New-Object System.Windows.Forms.NumericUpDown',
  '$num.DecimalPlaces = 2',
  '$num.Increment = 0.25',
  '$num.Minimum = 0',
  '$num.Maximum = 100',
  '$num.Value = 10.25',
  '$num.Bounds = New-Object System.Drawing.Rectangle 16, 48, 120, 28',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-DEC-OLD"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 16, 12, 280, 28',
  '$num.Add_ValueChanged({ $label.Text = "VCU-DEC-" + $num.Value.ToString([Globalization.CultureInfo]::InvariantCulture) })',
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
  $found = (& $Vcu wait --session $sid --name "VCU-DEC-HOST" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "decimal host not found" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "decimal:12.5" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "decimal failed $($typed.error)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "decimal_set") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-DEC-12.5" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "value changed event did not run" }
  $same = (& $Vcu type --session $sid --ref $ref --text "decimal:12.5" | Out-String) | ConvertFrom-Json
  if ($same.ok -eq $true) { throw "same decimal was accepted" }
  $high = (& $Vcu type --session $sid --ref $ref --text "decimal:150.5" | Out-String) | ConvertFrom-Json
  if ($high.ok -eq $true) { throw "out of range decimal was accepted" }
  $nope = (& $Vcu type --session $sid --ref $ref --text "decimal:nope" | Out-String) | ConvertFrom-Json
  if ($nope.ok -eq $true) { throw "non-decimal was accepted" }
  $plain = (& $Vcu type --session $sid --ref $ref --text "decimal:40" | Out-String) | ConvertFrom-Json
  if ($plain.ok -eq $true) { throw "integer text was accepted as decimal" }
  $cursor1 = [VcuCursor048]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-048 OK $cid $sid ref=$ref path=decimal_set value=12.5 cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
