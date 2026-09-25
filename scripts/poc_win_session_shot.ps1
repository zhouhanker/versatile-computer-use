# CU-WIN-SESSION-018: session screenshot of an owned window is a real PNG of that window.
# PrintWindow, not desktop CopyFromScreen. Does not move the OS cursor.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor018 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor018]::Pos()

$formScript = Join-Path $env:TEMP ("vcu-shot-018-" + [guid]::NewGuid().ToString("N") + ".ps1")
$formLines = @(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  'Add-Type -AssemblyName System.Drawing | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-SHOT-018"',
  '$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::FixedSingle',
  '$form.ShowInTaskbar = $true',
  '$form.TopMost = $true',
  '$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual',
  '$form.Bounds = New-Object System.Drawing.Rectangle 80, 80, 420, 280',
  '$form.BackColor = [System.Drawing.Color]::FromArgb(220, 30, 160)',
  '[System.Windows.Forms.Application]::Run($form)'
)
[System.IO.File]::WriteAllLines($formScript, $formLines)
$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru
$owned = $proc.Id
$sid = $null
$out = Join-Path $env:TEMP "vcu-shot-018.png"
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
  if (Test-Path $out) { Remove-Item $out -Force }
  $shot = (& $Vcu screenshot --session $sid --out $out | Out-String) | ConvertFrom-Json
  if ($shot.ok -ne $true) { throw "screenshot failed" }
  $d = $shot.data
  if ($d.bytes -lt 500 -or $d.width -lt 50 -or $d.height -lt 50) { throw "screenshot too small $($d.width)x$($d.height)" }
  $bytes = [System.IO.File]::ReadAllBytes($out)
  if ($bytes[0] -ne 0x89 -or $bytes[1] -ne 0x50) { throw "not a png" }
  Add-Type -AssemblyName System.Drawing | Out-Null
  $bmp = [System.Drawing.Bitmap]::FromFile($out)
  $c = $bmp.GetPixel([int]($bmp.Width / 2), [int]($bmp.Height / 2))
  $bmp.Dispose()
  $near = [Math]::Abs($c.R - 220) + [Math]::Abs($c.G - 30) + [Math]::Abs($c.B - 160)
  if ($near -gt 80) { throw "center not window color $($c.R),$($c.G),$($c.B) dist=$near" }
  $cursor1 = [VcuCursor018]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-018 OK $cid $sid $($d.width)x$($d.height) bytes=$($d.bytes) pixel=$($c.R),$($c.G),$($c.B) cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
