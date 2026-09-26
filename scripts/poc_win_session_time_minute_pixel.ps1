# CU-WIN-SESSION-074: session clicks the minute field, then the up arrow.
# Minute field at 14 percent of the client width, then the up arrow. Not time_pixel. Not DTM_SETSYSTEMTIME. Not SendInput.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor074 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor074]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-time-074-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-TIME-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 420, 180',
  '$dtp = New-Object System.Windows.Forms.DateTimePicker',
  '$dtp.Format = [System.Windows.Forms.DateTimePickerFormat]::Custom',
  '$dtp.CustomFormat = "HH:mm:ss"',
  '$dtp.ShowUpDown = $true',
  '$dtp.Value = Get-Date -Year 2026 -Month 1 -Day 2 -Hour 8 -Minute 0 -Second 0',
  '$dtp.Bounds = New-Object System.Drawing.Rectangle 16, 48, 220, 28',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-TIME-OLD"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 16, 12, 320, 28',
  '$dtp.Add_ValueChanged({ $label.Text = "VCU-TIME-" + $dtp.Value.ToString("HH:mm:ss") })',
  '$form.Controls.Add($label)',
  '$form.Controls.Add($dtp)',
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
  $found = (& $Vcu wait --session $sid --name "VCU-TIME-HOST" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "time host not found" }
  $ref = [string]$found.data.detail.found_ref
  $cursorClick0 = [VcuCursor074]::Pos()
  $typed = (& $Vcu type --session $sid --ref $ref --text "timepix:mup:08:01:00" | Out-String) | ConvertFrom-Json
  $cursorClick1 = [VcuCursor074]::Pos()
  if ($cursorClick0 -ne $cursorClick1) { throw "click moved cursor $cursorClick0 -> $cursorClick1" }
  if ($typed.ok -ne $true) { throw "time minute pixel failed $($typed.error)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "time_minute_pixel") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-TIME-08:01:00" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "value changed event did not run" }
  $cSame0 = [VcuCursor074]::Pos()
  $same = (& $Vcu type --session $sid --ref $ref --text "timepix:mup:08:01:00" | Out-String) | ConvertFrom-Json
  $cSame1 = [VcuCursor074]::Pos()
  if ($cSame0 -ne $cSame1) { throw "same-time check moved cursor $cSame0 -> $cSame1" }
  if ($same.ok -eq $true) { throw "same time was accepted" }
  $bad = (& $Vcu type --session $sid --ref $ref --text "timepix:mup:25:00:00" | Out-String) | ConvertFrom-Json
  if ($bad.ok -eq $true) { throw "invalid time was accepted" }
  $nope = (& $Vcu type --session $sid --ref $ref --text "timepix:mup:nope" | Out-String) | ConvertFrom-Json
  if ($nope.ok -eq $true) { throw "non-time was accepted" }
  $skip = (& $Vcu type --session $sid --ref $ref --text "timepix:mup:08:03:00" | Out-String) | ConvertFrom-Json
  if ($skip.ok -eq $true) { throw "two-minute jump was accepted as one click" }
  $cursor1 = [VcuCursor074]::Pos()
  if ($cursorClick0 -ne $cursorClick1) { throw "click moved cursor $cursorClick0 -> $cursorClick1" }
  Write-Host "CU-WIN-SESSION-074 OK $cid $sid ref=$ref path=time_minute_pixel value=08:01:00 cursor=$cursorClick1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
