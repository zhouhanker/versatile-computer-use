# CU-WIN-SESSION-004: session hover moves the Guide, not the OS cursor.
# Owned window only. Does not use SendInput. Kills only the form this script starts.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "debug vcu missing: $Vcu" }
$formScript = Join-Path $env:TEMP ("vcu-hover-010-" + [guid]::NewGuid().ToString("N") + ".ps1")
$formLines = @(
  'Add-Type -TypeDefinition @"',
  'using System;',
  'using System.Runtime.InteropServices;',
  'public static class VcuDpiHover {',
  '  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);',
  '}',
  '"@',
  '[void][VcuDpiHover]::SetProcessDpiAwarenessContext([IntPtr](-4))',
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  'Add-Type -AssemblyName System.Drawing | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-HOVER-010"',
  '$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 320, 420, 280',
  '$form.BackColor = [System.Drawing.Color]::FromArgb(255, 220, 0)',
  '$btn = New-Object System.Windows.Forms.Button',
  '$btn.Text = "VCU-HOVER-HIT"',
  '$btn.Left = 48',
  '$btn.Top = 72',
  '$btn.Width = 200',
  '$btn.Height = 72',
  '$btn.BackColor = [System.Drawing.Color]::FromArgb(255, 220, 0)',
  '$form.Controls.Add($btn)',
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
  if (-not $cid) { throw "owned window not listed for pid $owned" }
  $start = (& $Vcu session start --surface desktop --app-id $cid | Out-String) | ConvertFrom-Json
  if ($start.ok -ne $true -or $start.data.stage_hud -ne $true) { throw "stage not raised" }
  $sid = $start.data.session_id
  $snap = (& $Vcu snapshot --session $sid | Out-String) | ConvertFrom-Json
  $btn = @($snap.data.dom_refs | Where-Object { $_.name -eq "VCU-HOVER-HIT" } | Select-Object -First 1)
  if ($btn.Count -ne 1) { throw "hover button not in snapshot" }
  Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor010 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
}
"@
  $before = New-Object VcuCursor010+POINT
  [void][VcuCursor010]::GetCursorPos([ref]$before)
  $actionPath = Join-Path $env:TEMP ("vcu-hover-010-" + [guid]::NewGuid().ToString("N") + ".json")
  @{ type = "hover"; target = @{ ref = $btn[0].ref }; args = @{} } | ConvertTo-Json -Compress | Set-Content -LiteralPath $actionPath -Encoding ascii
  $act = (& $Vcu act --session $sid --action-json $actionPath | Out-String) | ConvertFrom-Json
  Remove-Item -LiteralPath $actionPath -Force -ErrorAction SilentlyContinue
  if ($act.ok -ne $true) { throw "hover failed" }
  $detail = $act.data.detail
  if ($null -eq $detail) { $detail = $act.data }
  if ($detail.input_path -ne "guide_hover") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $after = New-Object VcuCursor010+POINT
  [void][VcuCursor010]::GetCursorPos([ref]$after)
  if ($before.x -ne $after.x -or $before.y -ne $after.y) { throw "OS cursor moved $($before.x),$($before.y) -> $($after.x),$($after.y)" }
  $gx = [int]$detail.guide.x
  $gy = [int]$detail.guide.y
  Start-Sleep -Milliseconds 350
  $capScript = Join-Path $env:TEMP ("vcu-hover-cap-" + [guid]::NewGuid().ToString("N") + ".ps1")
  $capLines = @(
    'param([int]$X, [int]$Y)',
    'Add-Type -TypeDefinition @"',
    'using System;',
    'using System.Runtime.InteropServices;',
    'public static class VcuDpiHoverCap {',
    '  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);',
    '}',
    '"@',
    '[void][VcuDpiHoverCap]::SetProcessDpiAwarenessContext([IntPtr](-4))',
    'Add-Type -AssemblyName System.Drawing | Out-Null',
    '$bmp = New-Object System.Drawing.Bitmap 48, 48',
    '$g = [System.Drawing.Graphics]::FromImage($bmp)',
    '$g.CopyFromScreen(($X - 6), ($Y - 6), 0, 0, (New-Object System.Drawing.Size 48, 48))',
    '$g.Dispose()',
    '$dart = 0; $yellow = 0',
    'for ($yy = 0; $yy -lt 48; $yy += 2) {',
    '  for ($xx = 0; $xx -lt 48; $xx += 2) {',
    '    $c = $bmp.GetPixel($xx, $yy)',
    '    if ($c.R -gt 200 -and $c.G -gt 180 -and $c.B -lt 80) { $yellow++ }',
    '    elseif ($c.R -lt 140 -and $c.G -lt 150 -and $c.B -gt 70 -and $c.B -lt 160) { $dart++ }',
    '    elseif ($c.R -gt 230 -and $c.G -gt 230 -and $c.B -gt 230) { $dart++ }',
    '  }',
    '}',
    'Write-Output ("dart={0} yellow={1}" -f $dart, $yellow)'
  )
  [System.IO.File]::WriteAllLines($capScript, $capLines, (New-Object System.Text.UTF8Encoding $false))
  $seen = (& "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -STA -ExecutionPolicy Bypass -File $capScript -X $gx -Y $gy | Out-String).Trim()
  Remove-Item -LiteralPath $capScript -Force -ErrorAction SilentlyContinue
  if ($seen -notmatch 'dart=(\d+)') { throw "bad capture $seen" }
  if ([int]$Matches[1] -lt 3) { throw "guide not visible at $gx,$gy $seen" }
  Write-Host "CU-WIN-SESSION-004 OK $cid $sid $($btn[0].ref) guide=$gx,$gy $seen cursor=$($before.x),$($before.y)"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) {
    $still = Get-Process -Id $owned -ErrorAction SilentlyContinue
    if ($still -and $still.ProcessName -eq "powershell") { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  }
  Remove-Item -LiteralPath $formScript -Force -ErrorAction SilentlyContinue
}
