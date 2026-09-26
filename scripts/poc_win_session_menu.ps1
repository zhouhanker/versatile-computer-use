# CU-WIN-SESSION-044: session type invokes a named menu item without moving the cursor.
# IAccessible default action on role 12. Not a pixel click. Not SendInput.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor044 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor044]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-menu-044-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-MENU-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 420, 200',
  '$menu = New-Object System.Windows.Forms.MenuStrip',
  '$file = New-Object System.Windows.Forms.ToolStripMenuItem("VCU-MENU-FILE")',
  '$item = New-Object System.Windows.Forms.ToolStripMenuItem("VCU-MENU-B")',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-MENU-OLD"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 16, 40, 280, 28',
  '$item.Add_Click({ $label.Text = "VCU-MENU-HIT" })',
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
  $found = (& $Vcu wait --session $sid --name "VCU-MENU-HOST" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "menu host not found" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "menu:VCU-MENU-B" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "menu failed $($typed.error)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "menu_click") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-MENU-HIT" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "menu click event did not run" }
  $miss = (& $Vcu type --session $sid --ref $ref --text "menu:VCU-MENU-MISSING" | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "missing menu item was invoked" }
  $cursor1 = [VcuCursor044]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-044 OK $cid $sid ref=$ref path=menu_click value=VCU-MENU-B cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
