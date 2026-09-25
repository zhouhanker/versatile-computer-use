# WIN-VIS-007: capsule blur follows a backdrop change without restarting Stage.
# Samples the strip below the capsule, so the HUD stays visible in screenshots.
# Not NSVisualEffectView. Does not move the OS cursor. Kills only the form it starts.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "debug vcu missing: $Vcu" }
$colorFile = Join-Path $env:TEMP "vcu-vis-007-color.txt"
Set-Content -LiteralPath $colorFile -Value "255,0,180" -Encoding ascii
$formScript = Join-Path $env:TEMP ("vcu-vis-007-" + [guid]::NewGuid().ToString("N") + ".ps1")
$formLines = @(
  'Add-Type -TypeDefinition @"',
  'using System;',
  'using System.Runtime.InteropServices;',
  'public static class VcuDpi007 {',
  '  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);',
  '}',
  '"@',
  '[void][VcuDpi007]::SetProcessDpiAwarenessContext([IntPtr](-4))',
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
  '$form.Text = "VCU-VIS-007"',
  '$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None',
  '$form.ShowInTaskbar = $false',
  '$form.TopMost = $true',
  '$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual',
  '$form.Bounds = New-Object System.Drawing.Rectangle ($x - 40), ($y - 30), ($w + 80), ($h + 80)',
  '$form.BackColor = [System.Drawing.Color]::FromArgb(255, 0, 180)',
  '$timer = New-Object System.Windows.Forms.Timer',
  '$timer.Interval = 120',
  '$timer.Add_Tick({',
  '  if (-not (Test-Path -LiteralPath $ColorFile)) { return }',
  '  $parts = @((Get-Content -LiteralPath $ColorFile -Raw).Trim().Split(","))',
  '  if ($parts.Count -lt 3) { return }',
  '  $form.BackColor = [System.Drawing.Color]::FromArgb([int]$parts[0], [int]$parts[1], [int]$parts[2])',
  '})',
  '$timer.Start()',
  '[System.Windows.Forms.Application]::Run($form)'
)
# ColorFile must be expanded in the child script, not left as a parent variable.
$formLines = $formLines | ForEach-Object { $_ -replace '\$ColorFile', $colorFile }
[System.IO.File]::WriteAllLines($formScript, $formLines, (New-Object System.Text.UTF8Encoding $false))
$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru
$owned = $proc.Id
$sid = $null
function Read-HudPixel {
  $capScript = Join-Path $env:TEMP ("vcu-vis-007-cap-" + [guid]::NewGuid().ToString("N") + ".ps1")
  $capLines = @(
    'Add-Type -TypeDefinition @"',
    'using System;',
    'using System.Runtime.InteropServices;',
    'public static class VcuDpi007b {',
    '  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);',
    '}',
    '"@',
    '[void][VcuDpi007b]::SetProcessDpiAwarenessContext([IntPtr](-4))',
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
    '$bmp = New-Object System.Drawing.Bitmap 20, 12',
    '$g = [System.Drawing.Graphics]::FromImage($bmp)',
    '$g.CopyFromScreen(($x + 10), ($y + 8), 0, 0, (New-Object System.Drawing.Size 20, 12))',
    '$g.Dispose()',
    '$c = $bmp.GetPixel(6, 4)',
    'Write-Output ("pixel={0},{1},{2}" -f $c.R, $c.G, $c.B)'
  )
  [System.IO.File]::WriteAllLines($capScript, $capLines, (New-Object System.Text.UTF8Encoding $false))
  $sample = (& "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -STA -ExecutionPolicy Bypass -File $capScript | Out-String).Trim()
  Remove-Item -LiteralPath $capScript -Force -ErrorAction SilentlyContinue
  return $sample
}
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
  if ($start.ok -ne $true -or $start.data.stage_hud -ne $true) { throw "stage not raised" }
  $sid = $start.data.session_id
  Start-Sleep -Milliseconds 600
  $before = Read-HudPixel
  if ($before -notmatch 'pixel=(\d+),(\d+),(\d+)') { throw "bad before $before" }
  $br = [int]$Matches[1]; $bg = [int]$Matches[2]
  if ($bg -lt 8 -or $br -lt 70 -or $br -le $bg) { throw "initial blur was not magenta composite: $before" }
  Set-Content -LiteralPath $colorFile -Value "0,255,40" -Encoding ascii
  $after = $null
  for ($i = 0; $i -lt 12; $i++) {
    Start-Sleep -Milliseconds 400
    $after = Read-HudPixel
    if ($after -match 'pixel=(\d+),(\d+),(\d+)') {
      $ar = [int]$Matches[1]; $ag = [int]$Matches[2]; $ab = [int]$Matches[3]
      if ($ag -gt 70 -and $ag -gt $ar -and $ar -gt 8 -and $ab -lt 80) { break }
    }
  }
  if ($after -notmatch 'pixel=(\d+),(\d+),(\d+)') { throw "bad after $after" }
  $ar = [int]$Matches[1]; $ag = [int]$Matches[2]; $ab = [int]$Matches[3]
  if ($ag -le 70 -or $ag -le $ar -or $ar -le 8) { throw "blur did not follow green: before=$before after=$after" }
  Write-Host "WIN-VIS-007 OK $cid $sid before=$before after=$after"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) {
    $still = Get-Process -Id $owned -ErrorAction SilentlyContinue
    if ($still -and $still.ProcessName -eq "powershell") { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  }
  Remove-Item -LiteralPath $formScript -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $colorFile -Force -ErrorAction SilentlyContinue
}
