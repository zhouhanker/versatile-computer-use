# CU-WIN-SESSION-060: blank PrintWindow plus a covered center must refuse.
# Not a screen-rectangle copy. Not SendInput. Cursor must not move.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor060 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor060]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-blank-060-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type @"',
  'using System;',
  'using System.Runtime.InteropServices;',
  'public static class VcuBlankHost {',
  '  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr CreateWindowEx(int ex, string cls, string name, int style, int x, int y, int w, int h, IntPtr parent, IntPtr menu, IntPtr inst, IntPtr param);',
  '  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);',
  '  [DllImport("user32.dll")] public static extern bool DestroyWindow(IntPtr h);',
  '  public static IntPtr Open() {',
  '    IntPtr h = CreateWindowEx(0x00200000, "Static", "VCU-BLANK-060", 0x10CF0000, 140, 140, 360, 220, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);',
  '    if (h != IntPtr.Zero) ShowWindow(h, 5);',
  '    return h;',
  '  }',
  '}',
  '"@',
  '$hwnd = [VcuBlankHost]::Open()',
  'if ($hwnd -eq [IntPtr]::Zero) { throw "blank window missing" }',
  'Start-Sleep -Seconds 30'
) | Set-Content -Encoding ASCII -Path $formScript
$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru
$owned = $proc.Id
$sid = $null
$cover = $null
$coverScript = $null
try {
  $cid = $null
  for ($i = 0; $i -lt 25; $i++) {
    Start-Sleep -Milliseconds 200
    $raw = & $Vcu app windows --json | Out-String
    $hit = @((($raw | ConvertFrom-Json).data.windows) | Where-Object { $_.id -eq "win:powershell:$owned" })
    if ($hit.Count -eq 1) { $cid = $hit[0].id; break }
  }
  if (-not $cid) { throw "owned window not listed" }
  $coverScript = Join-Path $env:TEMP ("vcu-cover-060-" + [guid]::NewGuid().ToString("N") + ".ps1")
  @(
    'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
    'Add-Type -AssemblyName System.Drawing | Out-Null',
    '$form = New-Object System.Windows.Forms.Form',
    '$form.Text = "VCU-COVER-060"',
    '$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None',
    '$form.ShowInTaskbar = $false',
    '$form.TopMost = $true',
    '$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual',
    '$form.Bounds = New-Object System.Drawing.Rectangle 140, 140, 360, 220',
    '$form.BackColor = [System.Drawing.Color]::FromArgb(20, 180, 40)',
    '[System.Windows.Forms.Application]::Run($form)'
  ) | Set-Content -Encoding ASCII -Path $coverScript
  $coverProc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$coverScript) -PassThru
  $cover = $coverProc.Id
  Start-Sleep -Milliseconds 800
  Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class VcuCover060 {
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
    return p.x + "," + p.y + " title=" + sb.ToString() + " owned=" + (root == target || hit == target);
  }
}
"@
  $ownedHwnd = (Get-Process -Id $owned).MainWindowHandle
  if ($ownedHwnd -eq [IntPtr]::Zero) { throw "owned hwnd missing" }
  $hitInfo = [VcuCover060]::Hit($ownedHwnd)
  if ($hitInfo -match "owned=True") { throw "cover did not occlude center: $hitInfo" }
  $start = (& $Vcu session start --surface desktop --app-id $cid | Out-String) | ConvertFrom-Json
  if ($start.ok -ne $true -or $start.data.stage_hud -ne $true) { throw "stage not raised $($start | ConvertTo-Json -Compress -Depth 6)" }
  $sid = $start.data.session_id
  $shot = (& $Vcu screenshot --session $sid | Out-String) | ConvertFrom-Json
  if ($shot.ok -eq $true) { throw "blank covered window was copied" }
  $err = [string]$shot.error.message
  if (-not $err) { $err = [string]$shot.error }
  if ($err -notmatch "covered") { throw "screenshot failed without covered refusal: $err" }
  $cursor1 = [VcuCursor060]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-060 OK $cid $sid mode=OCCLUDED cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($cover) { Stop-Process -Id $cover -Force -ErrorAction SilentlyContinue }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  if ($coverScript) { Remove-Item $coverScript -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
