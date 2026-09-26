# CU-WIN-SESSION-063: session clicks a radio button by its client center.
# Not BM_CLICK. Not SendInput. Cursor must not move.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor063 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor063]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-radiopix-063-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-RADIO-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 460, 200',
  '$a = New-Object System.Windows.Forms.RadioButton',
  '$a.Text = "VCU-RADIO-A"',
  '$a.Checked = $true',
  '$a.Bounds = New-Object System.Drawing.Rectangle 24, 24, 200, 28',
  '$b = New-Object System.Windows.Forms.RadioButton',
  '$b.Text = "VCU-RADIO-B"',
  '$b.Checked = $false',
  '$b.Bounds = New-Object System.Drawing.Rectangle 24, 64, 200, 28',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-RADIO-SHOW-A"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 240, 24, 180, 28',
  '$b.Add_CheckedChanged({ if ($b.Checked) { $label.Text = "VCU-RADIO-SHOW-B" } })',
  '$form.Controls.Add($label)',
  '$form.Controls.Add($a)',
  '$form.Controls.Add($b)',
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
  $b = (& $Vcu wait --session $sid --name "VCU-RADIO-B" --value "toggle-off" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($b.ok -ne $true) { throw "radio B was not off" }
  $ref = [string]$b.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "radiopix:VCU-RADIO-B" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "radio pixel failed $($typed | ConvertTo-Json -Compress -Depth 8)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "radio_pixel") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $bOn = (& $Vcu wait --session $sid --name "VCU-RADIO-B" --value "toggle-on" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($bOn.ok -ne $true) { throw "radio B did not turn on" }
  $aOff = (& $Vcu wait --session $sid --name "VCU-RADIO-A" --value "toggle-off" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($aOff.ok -ne $true) { throw "radio A stayed on" }
  $label = (& $Vcu wait --session $sid --name "VCU-RADIO-SHOW-B" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "checked event did not run" }
  $again = (& $Vcu type --session $sid --ref $ref --text "radiopix:VCU-RADIO-B" | Out-String) | ConvertFrom-Json
  if ($again.ok -eq $true) { throw "already selected radio was clicked" }
  $miss = (& $Vcu type --session $sid --ref $ref --text "radiopix:VCU-RADIO-MISSING" | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "missing radio was pixel-clicked" }
  $cursor1 = [VcuCursor063]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-063 OK $cid $sid ref=$ref path=radio_pixel value=VCU-RADIO-B cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
