# CU-WIN-SESSION-047: session type sets an owned time picker without moving the cursor.
# DTM_SETSYSTEMTIME plus reflected DTN_DATETIMECHANGE for HH:mm:ss. Not a pixel click. Not date_set. Not SendInput.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor047 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor047]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-time-047-" + [guid]::NewGuid().ToString("N") + ".ps1")
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
  $typed = (& $Vcu type --session $sid --ref $ref --text "time:15:30:45" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "time failed $($typed.error)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "time_set") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-TIME-15:30:45" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "value changed event did not run" }
  $same = (& $Vcu type --session $sid --ref $ref --text "time:15:30:45" | Out-String) | ConvertFrom-Json
  if ($same.ok -eq $true) { throw "same time was accepted" }
  $bad = (& $Vcu type --session $sid --ref $ref --text "time:25:00:00" | Out-String) | ConvertFrom-Json
  if ($bad.ok -eq $true) { throw "invalid time was accepted" }
  $nope = (& $Vcu type --session $sid --ref $ref --text "time:nope" | Out-String) | ConvertFrom-Json
  if ($nope.ok -eq $true) { throw "non-time was accepted" }
  $cursor1 = [VcuCursor047]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-047 OK $cid $sid ref=$ref path=time_set value=15:30:45 cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
