# CU-WIN-SESSION-052: a covered window screenshot must not return the cover.
# Blank PrintWindow must not copy a covered window rectangle. Cursor must not move.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor052 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor052]::Pos()

$formScript = Join-Path $env:TEMP ("vcu-shot-052-" + [guid]::NewGuid().ToString("N") + ".ps1")
$formLines = @(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  'Add-Type -AssemblyName System.Drawing | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-SHOT-052"',
  '$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::FixedSingle',
  '$form.ShowInTaskbar = $true',
  '$form.TopMost = $false',
  '$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual',
  '$form.Bounds = New-Object System.Drawing.Rectangle 80, 80, 420, 280',
  '$form.BackColor = [System.Drawing.Color]::FromArgb(220, 30, 160)',
  '[System.Windows.Forms.Application]::Run($form)'
)
[System.IO.File]::WriteAllLines($formScript, $formLines)
$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru
$owned = $proc.Id
$sid = $null
$cover = $null
$coverScript = $null
$out = Join-Path $env:TEMP "vcu-shot-052.png"
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
  $coverScript = Join-Path $env:TEMP ("vcu-cover-052-" + [guid]::NewGuid().ToString("N") + ".ps1")
  $coverLines = @(
    'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
    'Add-Type -AssemblyName System.Drawing | Out-Null',
    '$form = New-Object System.Windows.Forms.Form',
    '$form.Text = "VCU-COVER-052"',
    '$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::FixedSingle',
    '$form.ShowInTaskbar = $true',
    '$form.TopMost = $true',
    '$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual',
    '$form.Bounds = New-Object System.Drawing.Rectangle 80, 80, 420, 280',
    '$form.BackColor = [System.Drawing.Color]::FromArgb(20, 180, 40)',
    '[System.Windows.Forms.Application]::Run($form)'
  )
  [System.IO.File]::WriteAllLines($coverScript, $coverLines)
  $coverProc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$coverScript) -PassThru
  $cover = $coverProc.Id
  Start-Sleep -Milliseconds 700
  Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class VcuCover052 {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
  [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr hwnd, uint flags);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder sb, int n);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  public static string Hit(IntPtr target) {
    RECT r; GetWindowRect(target, out r);
    POINT p = new POINT(); p.x = (r.Left + r.Right) / 2; p.y = (r.Top + r.Bottom) / 2;
    IntPtr hit = WindowFromPoint(p);
    IntPtr root = GetAncestor(hit, 2);
    var sb = new StringBuilder(256);
    GetWindowText(root, sb, 256);
    return p.x + "," + p.y + " root=" + root + " title=" + sb.ToString() + " owned=" + (root == target);
  }
}
"@
  $ownedHwnd = (Get-Process -Id $owned).MainWindowHandle
  $hitInfo = [VcuCover052]::Hit($ownedHwnd)
  if ($hitInfo -match "owned=True") { throw "cover did not occlude center: $hitInfo" }
  $start = (& $Vcu session start --surface desktop --app-id $cid | Out-String) | ConvertFrom-Json
  if ($start.ok -ne $true -or $start.data.stage_hud -ne $true) { throw "stage not raised" }
  $sid = $start.data.session_id
  if (Test-Path $out) { Remove-Item $out -Force }
  $shot = (& $Vcu screenshot --session $sid --out $out | Out-String) | ConvertFrom-Json
  if ($shot.ok -eq $true) {
    Add-Type -AssemblyName System.Drawing | Out-Null
    $bmp = [System.Drawing.Bitmap]::FromFile($out)
    $c = $bmp.GetPixel([int]($bmp.Width / 2), [int]($bmp.Height / 2))
    $bmp.Dispose()
    $green = [Math]::Abs($c.R - 20) + [Math]::Abs($c.G - 180) + [Math]::Abs($c.B - 40)
    if ($green -le 80) { throw "covered screenshot returned the cover $($c.R),$($c.G),$($c.B)" }
    $magenta = [Math]::Abs($c.R - 220) + [Math]::Abs($c.G - 30) + [Math]::Abs($c.B - 160)
    if ($magenta -gt 80) { throw "unexpected screenshot color $($c.R),$($c.G),$($c.B)" }
    $mode = "window-pixels"
  } else {
    $err = [string]$shot.error
    if ($err -notmatch "covered") { throw "screenshot failed without covered refusal: $err" }
    $mode = "refused"
  }
  $cursor1 = [VcuCursor052]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-052 OK $cid $sid mode=$mode cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($cover) { Stop-Process -Id $cover -Force -ErrorAction SilentlyContinue }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $coverScript -ErrorAction SilentlyContinue
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
