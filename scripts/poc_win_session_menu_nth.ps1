# CU-WIN-SESSION-051: the second same-named menu item is the one that runs Click.
# Not a pixel click. Not SendInput. Same name must not invoke the first item.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor051 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor051]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-menu-051-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-MENUNTH-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 420, 200',
  '$menu = New-Object System.Windows.Forms.MenuStrip',
  '$file = New-Object System.Windows.Forms.ToolStripMenuItem("VCU-FILE")',
  '$first = New-Object System.Windows.Forms.ToolStripMenuItem("VCU-SAME")',
  '$second = New-Object System.Windows.Forms.ToolStripMenuItem("VCU-SAME")',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-SAME-OLD"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 16, 40, 280, 28',
  '$first.Add_Click({ $label.Text = "VCU-SAME-1" })',
  '$second.Add_Click({ $label.Text = "VCU-SAME-2" })',
  '[void]$file.DropDownItems.Add($first)',
  '[void]$file.DropDownItems.Add($second)',
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
  $file = (& $Vcu wait --session $sid --name "VCU-FILE" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($file.ok -ne $true) { throw "parent menu item not in scene" }
  $snap = (& $Vcu snapshot --session $sid --json | Out-String) | ConvertFrom-Json
  $refs = @()
  $data = $snap.data
  if ($null -eq $data) { $data = $snap }
  $nodes = @()
  if ($data.dom_refs) { $nodes = @($data.dom_refs) }
  elseif ($data.elements) { $nodes = @($data.elements) }
  foreach ($node in $nodes) {
    if ([string]$node.name -eq "VCU-SAME") { $refs += [string]$node.ref }
  }
  if ($refs.Count -lt 2) { throw "expected two same-named menu refs, got $($refs.Count)" }
  $second = $refs[1]
  $click = (& $Vcu click --session $sid --ref $second | Out-String) | ConvertFrom-Json
  if ($click.ok -ne $true) { throw "second click failed $($click.error)" }
  $detail = $click.data.detail
  if ($detail.input_path -ne "menu_nth") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-SAME-2" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "second click event did not run" }
  $firstClick = (& $Vcu click --session $sid --ref $refs[0] | Out-String) | ConvertFrom-Json
  if ($firstClick.ok -ne $true) { throw "first click failed $($firstClick.error)" }
  if ($firstClick.data.detail.input_path -ne "menu_click") { throw "first path $($firstClick.data.detail.input_path)" }
  $back = (& $Vcu wait --session $sid --name "VCU-SAME-1" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($back.ok -ne $true) { throw "first click event did not run" }
  $cursor1 = [VcuCursor051]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-051 OK $cid $sid ref=$second path=menu_nth value=VCU-SAME cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
