# WIN-VIS-008: refresh samples the pixels directly behind the capsule, not only the strip below.
# Affinity is cleared after the sample so a screenshot still shows the HUD.
# Not NSVisualEffectView. Does not move the OS cursor. Kills only the form it starts.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "debug vcu missing: $Vcu" }
$modeFile = Join-Path $env:TEMP "vcu-vis-008-mode.txt"
Set-Content -LiteralPath $modeFile -Value "blue" -Encoding ascii
$formScript = Join-Path $env:TEMP ("vcu-vis-008-" + [guid]::NewGuid().ToString("N") + ".ps1")
$formLines = @(
  'Add-Type -TypeDefinition @"',
  'using System;',
  'using System.Runtime.InteropServices;',
  'public static class VcuDpi008 {',
  '  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);',
  '}',
  '"@',
  '[void][VcuDpi008]::SetProcessDpiAwarenessContext([IntPtr](-4))',
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
  '$form.Text = "VCU-VIS-008"',
  '$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None',
  '$form.ShowInTaskbar = $false',
  '$form.TopMost = $true',
  '$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual',
  '$form.Bounds = New-Object System.Drawing.Rectangle ($x - 20), ($y - 12), ($script:W + 40), ($script:H + 48)',
  '$form.BackColor = [System.Drawing.Color]::FromArgb(0, 80, 255)',
  '$form.Add_Paint({',
  '  param($sender, $e)',
  '  $mode = "blue"',
  '  $mf = "MODEFILE"',
  '  if (Test-Path -LiteralPath $mf) { $mode = (Get-Content -LiteralPath $mf -Raw).Trim() }',
  '  $e.Graphics.Clear([System.Drawing.Color]::FromArgb(0, 80, 255))',
  '  if ($mode -eq "split") {',
  '    $brush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(0, 255, 40))',
  '    $e.Graphics.FillRectangle($brush, 20, 12, $script:W, $script:H)',
  '    $brush.Dispose()',
  '  }',
  '})',
  '$timer = New-Object System.Windows.Forms.Timer',
  '$timer.Interval = 120',
  '$timer.Add_Tick({ $form.Invalidate() })',
  '$timer.Start()',
  '[System.Windows.Forms.Application]::Run($form)'
)
$formLines = $formLines | ForEach-Object { $_.Replace("MODEFILE", $modeFile) }
[System.IO.File]::WriteAllLines($formScript, $formLines, (New-Object System.Text.UTF8Encoding $false))
$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru
$owned = $proc.Id
$sid = $null
function Read-HudPixel {
  $capScript = Join-Path $env:TEMP ("vcu-vis-008-cap-" + [guid]::NewGuid().ToString("N") + ".ps1")
  $capLines = @(
    'Add-Type -TypeDefinition @"',
    'using System;',
    'using System.Runtime.InteropServices;',
    'public static class VcuDpi008b {',
    '  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);',
    '}',
    '"@',
    '[void][VcuDpi008b]::SetProcessDpiAwarenessContext([IntPtr](-4))',
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
    '$bmp = New-Object System.Drawing.Bitmap 16, 10',
    '$g = [System.Drawing.Graphics]::FromImage($bmp)',
    '$g.CopyFromScreen(($x + 30), ($y + 10), 0, 0, (New-Object System.Drawing.Size 16, 10))',
    '$g.Dispose()',
    '$c = $bmp.GetPixel(4, 4)',
    'Write-Output ("pixel={0},{1},{2}" -f $c.R, $c.G, $c.B)'
  )
  [System.IO.File]::WriteAllLines($capScript, $capLines, (New-Object System.Text.UTF8Encoding $false))
  $sample = (& "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -STA -ExecutionPolicy Bypass -File $capScript | Out-String).Trim()
  Remove-Item -LiteralPath $capScript -Force -ErrorAction SilentlyContinue
  return $sample
}
try {
  Start-Sleep -Milliseconds 800
  $cid = $null
  for ($i = 0; $i -lt 20; $i++) {
    $raw = & $Vcu app windows --json | Out-String
    $obj = $raw | ConvertFrom-Json
    $hit = @($obj.data.windows | Where-Object { $_.id -eq "win:powershell:$owned" })
    if ($hit.Count -eq 1) { $cid = $hit[0].id; break }
    Start-Sleep -Milliseconds 250
  }
  if (-not $cid) { throw "owned window not listed for pid $owned" }
  $start = (& $Vcu session start --surface desktop --app-id $cid | Out-String) | ConvertFrom-Json
  if ($start.ok -ne $true -or $start.data.stage_hud -ne $true) { throw "stage not raised" }
  $sid = $start.data.session_id
  Start-Sleep -Milliseconds 500
  $before = Read-HudPixel
  if ($before -notmatch 'pixel=(\d+),(\d+),(\d+)') { throw "bad before $before" }
  $br = [int]$Matches[1]; $bg = [int]$Matches[2]; $bb = [int]$Matches[3]
  if ($bb -lt 90 -or $bb -le $bg) { throw "initial blur was not blue composite: $before" }
  Set-Content -LiteralPath $modeFile -Value "split" -Encoding ascii
  $after = $null
  for ($i = 0; $i -lt 12; $i++) {
    Start-Sleep -Milliseconds 400
    $after = Read-HudPixel
    if ($after -match 'pixel=(\d+),(\d+),(\d+)') {
      $ar = [int]$Matches[1]; $ag = [int]$Matches[2]; $ab = [int]$Matches[3]
      if ($ag -gt 90 -and $ag -gt $ab -and $ar -gt 8) { break }
    }
  }
  if ($after -notmatch 'pixel=(\d+),(\d+),(\d+)') { throw "bad after $after" }
  $ar = [int]$Matches[1]; $ag = [int]$Matches[2]; $ab = [int]$Matches[3]
  if ($ag -le 90 -or $ag -le $ab -or $ar -le 8) { throw "did not follow the pixels behind: before=$before after=$after" }
  Write-Host "WIN-VIS-008 OK $cid $sid before=$before after=$after"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) {
    $still = Get-Process -Id $owned -ErrorAction SilentlyContinue
    if ($still -and $still.ProcessName -eq "powershell") { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  }
  Remove-Item -LiteralPath $formScript -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $modeFile -Force -ErrorAction SilentlyContinue
}
