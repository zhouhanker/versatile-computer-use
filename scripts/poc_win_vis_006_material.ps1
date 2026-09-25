# WIN-VIS-006: Windows HUD samples the desktop and blurs it into the capsule.
# Not macOS NSVisualEffectView, not a live DWM material, not a full product CU.
# Does not move the OS cursor. Does not use SendInput. Kills only the form it starts.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "debug vcu missing: $Vcu" }

$formScript = Join-Path $env:TEMP ("vcu-vis-006-" + [guid]::NewGuid().ToString("N") + ".ps1")
$formLines = @(
  'Add-Type -TypeDefinition @"',
  'using System;',
  'using System.Runtime.InteropServices;',
  'public static class VcuDpi006 {',
  '  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);',
  '}',
  '"@',
  '[void][VcuDpi006]::SetProcessDpiAwarenessContext([IntPtr](-4))',
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  'Add-Type -AssemblyName System.Drawing | Out-Null',
  '$screen = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea',
  '$dpi = 96',
  'try { $dpi = [int][System.Drawing.Graphics]::FromHwnd([IntPtr]::Zero).DpiX } catch {}',
  '$scale = $dpi / 96.0',
  '$w = [int][Math]::Round(280 * $scale)',
  '$h = [int][Math]::Round(28 * $scale)',
  '$x = [int]($screen.Left + ($screen.Width - $w) / 2)',
  '$y = [int]($screen.Top + 8)',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-VIS-006"',
  '$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None',
  '$form.ShowInTaskbar = $false',
  '$form.TopMost = $true',
  '$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual',
  '$form.Bounds = New-Object System.Drawing.Rectangle ($x - 30), ($y - 20), ($w + 60), ($h + 40)',
  '$form.BackColor = [System.Drawing.Color]::FromArgb(255, 0, 180)',
  '[System.Windows.Forms.Application]::Run($form)'
)
[System.IO.File]::WriteAllLines($formScript, $formLines, (New-Object System.Text.UTF8Encoding $false))

$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru
$owned = $proc.Id
$sid = $null
try {
  Start-Sleep -Milliseconds 700
  $cid = $null
  for ($i = 0; $i -lt 20; $i++) {
    $raw = & $Vcu app windows --json | Out-String
    $obj = $raw | ConvertFrom-Json
    $hit = @($obj.data.windows | Where-Object { $_.id -eq "win:powershell:$owned" })
    if ($hit.Count -eq 1) { $cid = $hit[0].id; break }
    Start-Sleep -Milliseconds 250
  }
  if (-not $cid) { throw "owned color window not listed for pid $owned" }

  $start = (& $Vcu session start --surface desktop --app-id $cid | Out-String) | ConvertFrom-Json
  if ($start.ok -ne $true) { throw "session start failed" }
  if ($start.data.stage_hud -ne $true) { throw "stage hud not raised" }
  $sid = $start.data.session_id
  Start-Sleep -Milliseconds 700

  $capScript = Join-Path $env:TEMP ("vcu-vis-006-cap-" + [guid]::NewGuid().ToString("N") + ".ps1")
  $capLines = @(
    'Add-Type -TypeDefinition @"',
    'using System;',
    'using System.Runtime.InteropServices;',
    'public static class VcuDpi006b {',
    '  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);',
    '}',
    '"@',
    '[void][VcuDpi006b]::SetProcessDpiAwarenessContext([IntPtr](-4))',
    'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
    'Add-Type -AssemblyName System.Drawing | Out-Null',
    '$screen = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea',
    '$dpi = 96',
    'try { $dpi = [int][System.Drawing.Graphics]::FromHwnd([IntPtr]::Zero).DpiX } catch {}',
    '$scale = $dpi / 96.0',
    '$w = [int][Math]::Round(280 * $scale)',
    '$h = [int][Math]::Round(28 * $scale)',
    '$x = [int]($screen.Left + ($screen.Width - $w) / 2)',
    '$y = [int]($screen.Top + 8)',
    '$bmp = New-Object System.Drawing.Bitmap 24, 16',
    '$g = [System.Drawing.Graphics]::FromImage($bmp)',
    '$g.CopyFromScreen(($x + 8), ($y + [int]($h / 2) - 4), 0, 0, (New-Object System.Drawing.Size 24, 16))',
    '$g.Dispose()',
    '$rs = 0; $gs = 0; $bs = 0; $n = 0',
    'for ($yy = 2; $yy -le 10; $yy += 2) {',
    '  for ($xx = 2; $xx -le 14; $xx += 2) {',
    '    $c = $bmp.GetPixel($xx, $yy)',
    '    $rs += $c.R; $gs += $c.G; $bs += $c.B; $n++',
    '  }',
    '}',
    '$r = [int]($rs / $n); $gch = [int]($gs / $n); $b = [int]($bs / $n)',
    'Write-Output ("pixel={0},{1},{2} n={3} origin={4},{5}" -f $r, $gch, $b, $n, $x, $y)'
  )
  [System.IO.File]::WriteAllLines($capScript, $capLines, (New-Object System.Text.UTF8Encoding $false))
  $sample = (& "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -STA -ExecutionPolicy Bypass -File $capScript | Out-String).Trim()
  Remove-Item -LiteralPath $capScript -Force -ErrorAction SilentlyContinue
  if ($sample -notmatch 'pixel=(\d+),(\d+),(\d+)') { throw "bad sample $sample" }
  $r = [int]$Matches[1]; $gch = [int]$Matches[2]; $b = [int]$Matches[3]
  if ($gch -lt 8) { throw "looks like raw backdrop, HUD not composited: $sample" }
  if ($r -lt 70 -or $r -le $gch) { throw "blur did not take the magenta sample: $sample" }
  Write-Host "WIN-VIS-006 OK $cid $sid $sample"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) {
    $still = Get-Process -Id $owned -ErrorAction SilentlyContinue
    if ($still -and $still.ProcessName -eq "powershell") { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  }
  Remove-Item -LiteralPath $formScript -Force -ErrorAction SilentlyContinue
}
