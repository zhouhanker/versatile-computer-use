# CU-WIN-SESSION-031: session type sets an owned trackbar without moving the cursor.
# TBM_SETPOS plus a reflected scroll, not SendInput.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor031 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor031]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-track-031-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-TRACK-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 160, 140, 420, 220',
  '$track = New-Object System.Windows.Forms.TrackBar',
  '$track.AccessibleName = "VCU-TRACK"',
  '$track.Minimum = 0',
  '$track.Maximum = 100',
  '$track.Value = 10',
  '$track.Bounds = New-Object System.Drawing.Rectangle 24, 24, 260, 48',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-TRACK-10"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 24, 84, 220, 28',
  '$track.Add_ValueChanged({ $label.Text = "VCU-TRACK-" + $track.Value })',
  '$form.Controls.Add($track)',
  '$form.Controls.Add($label)',
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
  $found = (& $Vcu wait --session $sid --value "track=10" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "track not found" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "40" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "set failed $($typed.error)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "track_select") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-TRACK-40" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "label did not follow value" }
  $seen = (& $Vcu wait --session $sid --value "track=40" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($seen.ok -ne $true) { throw "track value not visible" }
  $miss = (& $Vcu type --session $sid --ref $ref --text "400" | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "out of range was accepted" }
  $bad = (& $Vcu type --session $sid --ref $ref --text "nope" | Out-String) | ConvertFrom-Json
  if ($bad.ok -eq $true) { throw "non-numeric was accepted" }
  $cursor1 = [VcuCursor031]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-031 OK $cid $sid ref=$ref path=track_select value=40 cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
