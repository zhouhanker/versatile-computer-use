# CU-WIN-SESSION-024: session extract reads an owned textbox value. Missing selector is empty, not a fake hit.
# No SendInput. Does not move the OS cursor.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor024 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor024]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-extract-024-" + [guid]::NewGuid().ToString("N") + ".ps1")
$formLines = @(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  'Add-Type -AssemblyName System.Drawing | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-EXTRACT-HOST"',
  '$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::FixedSingle',
  '$form.TopMost = $true',
  '$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual',
  '$form.Bounds = New-Object System.Drawing.Rectangle 200, 220, 420, 180',
  '$box = New-Object System.Windows.Forms.TextBox',
  '$box.Text = "VCU-EXTRACT-024"',
  '$box.Bounds = New-Object System.Drawing.Rectangle 24, 24, 240, 28',
  '$form.Controls.Add($box)',
  '[System.Windows.Forms.Application]::Run($form)'
)
[System.IO.File]::WriteAllLines($formScript, $formLines)
$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru
$owned = $proc.Id
$sid = $null
try {
  $cid = $null
  for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Milliseconds 200
    $raw = & $Vcu app windows --json | Out-String
    $obj = $raw | ConvertFrom-Json
    $hit = @($obj.data.windows | Where-Object { $_.id -eq "win:powershell:$owned" })
    if ($hit.Count -eq 1) { $cid = $hit[0].id; break }
  }
  if (-not $cid) { throw "owned window not listed" }
  $start = (& $Vcu session start --surface desktop --app-id $cid | Out-String) | ConvertFrom-Json
  if ($start.ok -ne $true -or $start.data.stage_hud -ne $true) { throw "stage not raised" }
  $sid = $start.data.session_id
  $got = (& $Vcu extract --session $sid --selector "VCU-EXTRACT-024" | Out-String) | ConvertFrom-Json
  if ($got.ok -ne $true) { throw "extract failed" }
  $matches = @($got.data.matches)
  if ($matches.Count -lt 1) { throw "no match" }
  $hit = $matches | Where-Object { $_.value -eq "VCU-EXTRACT-024" } | Select-Object -First 1
  if (-not $hit) { throw "value not in matches" }
  if ($hit.source -ne "desktop.scene") { throw "source $($hit.source)" }
  if ($hit.role -like "ControlType.Window*") { throw "matched the window $($hit.role)" }
  $miss = (& $Vcu extract --session $sid --selector "VCU-EXTRACT-MISSING" | Out-String) | ConvertFrom-Json
  if ($miss.ok -ne $true) { throw "missing extract should stay ok" }
  if (@($miss.data.matches).Count -ne 0) { throw "missing selector returned matches" }
  $cursor1 = [VcuCursor024]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-024 OK $cid $sid ref=$($hit.ref) value=$($hit.value) source=$($hit.source) miss=0 cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
