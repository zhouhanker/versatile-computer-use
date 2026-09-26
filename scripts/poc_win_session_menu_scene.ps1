# CU-WIN-SESSION-045: a named menu item appears in the scene and click runs Click.
# Not a pixel click. Not SendInput. The item is still not a UIA child by itself.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor045 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor045]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-menu-045-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-MENUSCENE-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 420, 200',
  '$menu = New-Object System.Windows.Forms.MenuStrip',
  '$file = New-Object System.Windows.Forms.ToolStripMenuItem("VCU-SCENE-FILE")',
  '$item = New-Object System.Windows.Forms.ToolStripMenuItem("VCU-SCENE-B")',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-SCENE-OLD"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 16, 40, 280, 28',
  '$item.Add_Click({ $label.Text = "VCU-SCENE-HIT" })',
  '[void]$file.DropDownItems.Add($item)',
  '[void]$menu.Items.Add($file)',
  '$form.Controls.Add($label)',
  '$form.Controls.Add($menu)',
  '$form.MainMenuStrip = $menu',
  '[System.Windows.Forms.Application]::Run($form)'
) | Set-Content -Encoding ASCII -Path $formScript
$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru
$owned = $proc.Id
$sid = $null
try {
  $cid = $null
  for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Milliseconds 200
    $raw = & $Vcu app windows --json | Out-String
    $hit = @((($raw | ConvertFrom-Json).data.windows) | Where-Object { $_.id -eq "win:powershell:$owned" })
    if ($hit.Count -eq 1) { $cid = $hit[0].id; break }
  }
  if (-not $cid) { throw "owned window not listed" }
  $start = (& $Vcu session start --surface desktop --app-id $cid | Out-String) | ConvertFrom-Json
  if ($start.ok -ne $true -or $start.data.stage_hud -ne $true) { throw "stage not raised" }
  $sid = $start.data.session_id
  $found = (& $Vcu wait --session $sid --name "VCU-SCENE-B" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "menu item not in scene" }
  $ref = [string]$found.data.detail.found_ref
  $file = (& $Vcu wait --session $sid --name "VCU-SCENE-FILE" --ms 800 | Out-String) | ConvertFrom-Json
  if ($file.ok -ne $true) { throw "parent menu item not in scene" }
  $miss = (& $Vcu wait --session $sid --name "VCU-SCENE-MISSING" --ms 600 | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "missing menu item was invented" }
  $click = (& $Vcu click --session $sid --ref $ref | Out-String) | ConvertFrom-Json
  if ($click.ok -ne $true) { throw "scene click failed $($click.error)" }
  $detail = $click.data.detail
  if ($detail.input_path -ne "menu_click") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-SCENE-HIT" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "menu click event did not run" }
  $cursor1 = [VcuCursor045]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-045 OK $cid $sid ref=$ref path=menu_click value=VCU-SCENE-B cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
