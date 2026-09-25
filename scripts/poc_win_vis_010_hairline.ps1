# WIN-VIS-010: acrylic capsule keeps a macOS-like hairline and soft shadow.
# Semibold title. Not NSVisualEffectView. Does not move the OS cursor.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor009 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
[void][VcuCursor009]::SetProcessDpiAwarenessContext([IntPtr](-4))
$cursor0 = [VcuCursor009]::Pos()
$modeFile = Join-Path $env:TEMP "vcu-vis-010-mode.txt"
Set-Content -LiteralPath $modeFile -Value "green" -Encoding ascii
Remove-Item (Join-Path $env:TEMP "vcu-stage-material.txt") -ErrorAction SilentlyContinue
$formScript = Join-Path $env:TEMP ("vcu-vis-010-" + [guid]::NewGuid().ToString("N") + ".ps1")
$formLines = @(
  'Add-Type -TypeDefinition @"',
  'using System;',
  'using System.Runtime.InteropServices;',
  'public static class VcuDpi009 {',
  '  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);',
  '}',
  '"@',
  '[void][VcuDpi009]::SetProcessDpiAwarenessContext([IntPtr](-4))',
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  'Add-Type -AssemblyName System.Drawing | Out-Null',
  '$screen = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea',
  '$dpi = 96',
  'try { $dpi = [int][System.Drawing.Graphics]::FromHwnd([IntPtr]::Zero).DpiX } catch {}',
  '$scale = $dpi / 96.0',
  '$script:W = [int][Math]::Round(280 * $scale)',
  '$script:H = [int][Math]::Round(28 * $scale)',
  '$x = [int]($screen.Left + ($screen.Width - $script:W) / 2)',
  '$y = [int]($screen.Top + 8)',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-VIS-009"',
  '$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None',
  '$form.ShowInTaskbar = $false',
  '$form.TopMost = $true',
  '$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual',
  '$form.Bounds = New-Object System.Drawing.Rectangle ($x - 24), ($y - 16), ($script:W + 48), ($script:H + 40)',
  '$form.Add_Paint({',
  '  param($sender, $e)',
  '  $mode = "green"',
  '  $mf = "MODEFILE"',
  '  if (Test-Path -LiteralPath $mf) { $mode = (Get-Content -LiteralPath $mf -Raw).Trim() }',
  '  if ($mode -eq "red") { $e.Graphics.Clear([System.Drawing.Color]::FromArgb(220, 20, 20)) }',
  '  else { $e.Graphics.Clear([System.Drawing.Color]::FromArgb(0, 180, 40)) }',
  '})',
  '$timer = New-Object System.Windows.Forms.Timer',
  '$timer.Interval = 80',
  '$timer.Add_Tick({ $form.Invalidate() })',
  '$timer.Start()',
  '[System.Windows.Forms.Application]::Run($form)'
)
$formLines = $formLines | ForEach-Object { $_.Replace("MODEFILE", $modeFile) }
[System.IO.File]::WriteAllLines($formScript, $formLines, (New-Object System.Text.UTF8Encoding $false))
$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru -WindowStyle Hidden
$owned = $proc.Id
$sid = $null
function Get-HudOrigin {
  Add-Type -AssemblyName System.Drawing | Out-Null
  Add-Type -AssemblyName System.Windows.Forms | Out-Null
  $screen = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
  $dpi = [int][System.Drawing.Graphics]::FromHwnd([IntPtr]::Zero).DpiX
  $scale = $dpi / 96.0
  $script:HW = [int][Math]::Round(280 * $scale)
  $script:HH = [int][Math]::Round(28 * $scale)
  $script:HX = [int]($screen.Left + ($screen.Width - $script:HW) / 2)
  $script:HY = [int]($screen.Top + 8)
}
function Read-At([int]$dx, [int]$dy) {
  Get-HudOrigin
  $bmp = New-Object System.Drawing.Bitmap 1, 1
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen(($script:HX + [int]($script:HW / 2) + $dx), ($script:HY + $dy), 0, 0, (New-Object System.Drawing.Size 1, 1))
  $c = $bmp.GetPixel(0, 0)
  $g.Dispose(); $bmp.Dispose()
  return $c
}
function Read-Gap {
  return Read-At 0 ([int]($script:HH / 2))
}
function Count-Bright {
  Add-Type -AssemblyName System.Drawing | Out-Null
  Add-Type -AssemblyName System.Windows.Forms | Out-Null
  $screen = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
  $dpi = [int][System.Drawing.Graphics]::FromHwnd([IntPtr]::Zero).DpiX
  $scale = $dpi / 96.0
  $w = [int][Math]::Round(280 * $scale)
  $h = [int][Math]::Round(28 * $scale)
  $x = [int]($screen.Left + ($screen.Width - $w) / 2)
  $y = [int]($screen.Top + 8)
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($x, $y, 0, 0, (New-Object System.Drawing.Size $w, $h))
  $g.Dispose()
  $n = 0
  for ($i = 8; $i -lt [int]($w * 0.45); $i += 2) {
    for ($j = 4; $j -lt ($h - 4); $j += 2) {
      $c = $bmp.GetPixel($i, $j)
      if ($c.R -gt 200 -and $c.G -gt 200 -and $c.B -gt 200) { $n++ }
    }
  }
  $bmp.Dispose()
  return $n
}
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
  $mat = $null
  for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Milliseconds 150
    $mf = Join-Path $env:TEMP "vcu-stage-material.txt"
    if (Test-Path -LiteralPath $mf) { $mat = (Get-Content -LiteralPath $mf -Raw).Trim(); break }
  }
  if ($mat -ne "acrylic") { throw "material=$mat" }
  Start-Sleep -Milliseconds 300
  $green = Read-Gap
  $bright = Count-Bright
  if ($bright -lt 8) { throw "white text missing bright=$bright" }
  if ($green.G -le $green.R -or $green.G -le $green.B) { throw "green acrylic not visible $($green.R),$($green.G),$($green.B)" }
  Set-Content -LiteralPath $modeFile -Value "red" -Encoding ascii
  Start-Sleep -Milliseconds 220
  $red = Read-Gap
  if ($red.R -le $red.G) { throw "acrylic did not follow within 220ms green=$($green.R),$($green.G),$($green.B) red=$($red.R),$($red.G),$($red.B)" }
  $edge = Read-At 0 2
  $mid = Read-At 0 ([int]($script:HH / 2))
  $near = Read-At 0 ($script:HH + 6)
  $far = Read-At 0 ($script:HH + 22)
  $edgeSum = $edge.R + $edge.G + $edge.B
  $midSum = $mid.R + $mid.G + $mid.B
  $nearSum = $near.R + $near.G + $near.B
  $farSum = $far.R + $far.G + $far.B
  if ($edgeSum -le ($midSum + 15)) { throw "hairline not brighter edge=$edgeSum mid=$midSum" }
  if ($nearSum -ge ($farSum - 8)) { throw "shadow not darker near=$nearSum far=$farSum" }
  $cursor1 = [VcuCursor009]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "WIN-VIS-010 OK acrylic hairline=$edgeSum/$midSum shadow=$nearSum/$farSum green=$($green.R),$($green.G),$($green.B) red=$($red.R),$($red.G),$($red.B) bright=$bright cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
