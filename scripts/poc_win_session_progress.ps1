# CU-WIN-SESSION-035: session type sets an owned progress bar position.
# PBM_SETPOS, read back with PBM_GETPOS. Not the managed Value property. Not SendInput.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor035b {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor035b]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-prog-035-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-PROG-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 420, 160',
  '$bar = New-Object System.Windows.Forms.ProgressBar',
  '$bar.Minimum = 0',
  '$bar.Maximum = 100',
  '$bar.Value = 10',
  '$bar.Bounds = New-Object System.Drawing.Rectangle 16, 24, 280, 24',
  '$form.Controls.Add($bar)',
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
  $found = (& $Vcu wait --session $sid --value "progress=10" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "progress not found" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "40" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "set failed $($typed.error)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "progress_set") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $seen = (& $Vcu wait --session $sid --value "progress=40" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($seen.ok -ne $true) { throw "progress value not visible" }
  $miss = (& $Vcu type --session $sid --ref $ref --text "400" | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "out of range was accepted" }
  $bad = (& $Vcu type --session $sid --ref $ref --text "nope" | Out-String) | ConvertFrom-Json
  if ($bad.ok -eq $true) { throw "non-numeric was accepted" }
  $cursor1 = [VcuCursor035b]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-035 OK $cid $sid ref=$ref path=progress_set value=40 cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
