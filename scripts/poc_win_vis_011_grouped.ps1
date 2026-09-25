# WIN-VIS-011: Windows capsule groups an accent dot, semibold title, and regular Esc.
# Not NSVisualEffectView. Does not move the OS cursor. Does not click Allow.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class VcuVis011 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x + "," + p.y; }
}
"@
[void][VcuVis011]::SetProcessDpiAwarenessContext([IntPtr](-4))
$cursor0 = [VcuVis011]::Pos()
Remove-Item (Join-Path $env:TEMP "vcu-stage-hud.txt") -ErrorAction SilentlyContinue
Remove-Item (Join-Path $env:TEMP "vcu-stage-material.txt") -ErrorAction SilentlyContinue
$formScript = Join-Path $env:TEMP ("vcu-vis-011-" + [guid]::NewGuid().ToString("N") + ".ps1")
$formLines = @(
  'Add-Type -TypeDefinition @"',
  'using System;',
  'using System.Runtime.InteropServices;',
  'public static class VcuDpi011 {',
  '  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);',
  '}',
  '"@',
  '[void][VcuDpi011]::SetProcessDpiAwarenessContext([IntPtr](-4))',
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  'Add-Type -AssemblyName System.Drawing | Out-Null',
  '$screen = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea',
  '$dpi = 96',
  'try { $dpi = [int][System.Drawing.Graphics]::FromHwnd([IntPtr]::Zero).DpiX } catch {}',
  '$scale = $dpi / 96.0',
  '$w = [int][Math]::Round(360 * $scale)',
  '$h = [int][Math]::Round(80 * $scale)',
  '$x = [int]($screen.Left + ($screen.Width - $w) / 2)',
  '$y = [int]($screen.Top)',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-VIS-011"',
  '$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None',
  '$form.ShowInTaskbar = $false',
  '$form.TopMost = $false',
  '$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual',
  '$form.Bounds = New-Object System.Drawing.Rectangle $x, $y, $w, $h',
  '$form.BackColor = [System.Drawing.Color]::FromArgb(0, 180, 40)',
  '[System.Windows.Forms.Application]::Run($form)'
)
[System.IO.File]::WriteAllLines($formScript, $formLines, (New-Object System.Text.UTF8Encoding $false))
$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru -WindowStyle Hidden
$owned = $proc.Id
$sid = $null
function Read-HudFile {
  $mf = Join-Path $env:TEMP "vcu-stage-hud.txt"
  if (-not (Test-Path -LiteralPath $mf)) { return $null }
  $raw = (Get-Content -LiteralPath $mf -Raw).Trim()
  $map = @{}
  foreach ($part in $raw.Split(" ")) {
    $kv = $part.Split("=", 2)
    if ($kv.Count -eq 2) { $map[$kv[0]] = $kv[1] }
  }
  return $map
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
  $geo = $null
  $mat = $null
  for ($i = 0; $i -lt 25; $i++) {
    Start-Sleep -Milliseconds 150
    $geo = Read-HudFile
    $mf = Join-Path $env:TEMP "vcu-stage-material.txt"
    if (Test-Path -LiteralPath $mf) { $mat = (Get-Content -LiteralPath $mf -Raw).Trim() }
    if ($geo -and $mat) { break }
  }
  if (-not $geo) { throw "hud geometry missing" }
  if ($mat -ne "acrylic") { throw "material=$mat" }
  $hx = [int]$geo.x
  $hy = [int]$geo.y
  $hw = [int]$geo.w
  $hh = [int]$geo.h
  if ($hw -lt 200 -or $hw -gt 520) { throw "hud width out of scaled 220-320 range: $hw" }
  Add-Type -AssemblyName System.Drawing | Out-Null
  $bmp = $null
  for ($try = 0; $try -lt 10; $try++) {
    $probe = New-Object System.Drawing.Bitmap $hw, $hh
    $pg = [System.Drawing.Graphics]::FromImage($probe)
    $pg.CopyFromScreen($hx, $hy, 0, 0, (New-Object System.Drawing.Size $hw, $hh))
    $pg.Dispose()
    $seen = 0
    $mid = [int]($hh / 2)
    for ($px = 4; $px -lt 36; $px += 2) {
      $c = $probe.GetPixel($px, $mid)
      if ($c.B -gt 90 -and $c.R -lt 90 -and $c.G -gt 40) { $seen++ }
    }
    if ($seen -ge 1) { $bmp = $probe; break }
    $probe.Dispose()
    Start-Sleep -Milliseconds 200
  }
  if ($null -eq $bmp) { throw "accent dot not visible at $hx,$hy ${hw}x$hh" }
  $accent = $geo.accent.Split(",")
  $ar = [int]$accent[0]; $ag = [int]$accent[1]; $ab = [int]$accent[2]
  $dotHits = 0
  $dotX = 9999
  for ($x = 4; $x -lt [Math]::Min($hw, 48); $x++) {
    for ($y = 4; $y -lt ($hh - 4); $y++) {
      $c = $bmp.GetPixel($x, $y)
      if ([Math]::Abs($c.R - $ar) -le 60 -and [Math]::Abs($c.G - $ag) -le 60 -and [Math]::Abs($c.B - $ab) -le 70 -and ($c.R + $c.G + $c.B) -lt 700) {
        $dotHits++
        if ($x -lt $dotX) { $dotX = $x }
      }
    }
  }
  $bright = 0
  $brightX = 9999
  for ($x = 8; $x -lt [int]($hw * 0.72); $x += 1) {
    for ($y = 4; $y -lt ($hh - 4); $y += 2) {
      $c = $bmp.GetPixel($x, $y)
      if ($c.R -gt 200 -and $c.G -gt 200 -and $c.B -gt 200) {
        $bright++
        if ($x -lt $brightX) { $brightX = $x }
      }
    }
  }
  $rightBright = 0
  $rightStart = [int]($hw * 0.82)
  for ($x = $rightStart; $x -lt ($hw - 2); $x++) {
    for ($y = 4; $y -lt ($hh - 4); $y += 2) {
      $c = $bmp.GetPixel($x, $y)
      if ($c.R -gt 210 -and $c.G -gt 210 -and $c.B -gt 210) { $rightBright++ }
    }
  }
  $bmp.Dispose()
  if ($dotHits -lt 4) { throw "accent dot missing hits=$dotHits accent=$($geo.accent)" }
  if ($bright -lt 8) { throw "title text missing bright=$bright" }
  if ($brightX -le $dotX) { throw "title is not to the right of the accent dot dotX=$dotX textX=$brightX" }
  if ($rightBright -gt 8) { throw "Esc still pinned to the right edge rightBright=$rightBright" }
  $cursor1 = [VcuVis011]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "WIN-VIS-011 OK acrylic grouped w=$hw h=$hh dot=$dotHits/$dotX text=$bright/$brightX right=$rightBright accent=$($geo.accent) cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
