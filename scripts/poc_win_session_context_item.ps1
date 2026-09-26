# CU-WIN-SESSION-081: session opens an owned context menu and clicks a named item.
# Not context_pixel. Not menu_pixel. Not SendInput. Cursor must not move.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor081 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor081]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-ctxitem-081-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-CTXITEM-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 420, 220',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-CTXITEM-OLD"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 16, 16, 320, 28',
  '$button = New-Object System.Windows.Forms.Button',
  '$button.Text = "VCU-CTX-TARGET"',
  '$button.Bounds = New-Object System.Drawing.Rectangle 16, 64, 180, 36',
  '$menu = New-Object System.Windows.Forms.ContextMenuStrip',
  '$other = New-Object System.Windows.Forms.ToolStripMenuItem("VCU-CTX-OTHER")',
  '$item = New-Object System.Windows.Forms.ToolStripMenuItem("VCU-CTX-ITEM")',
  '[void]$menu.Items.Add($other)',
  '[void]$menu.Items.Add($item)',
  '$menu.Add_Opening({ if ($label.Text -ne "VCU-CTX-HIT") { $label.Text = "VCU-CTX-OPEN" } })',
  '$item.Add_Click({ $label.Text = "VCU-CTX-HIT" })',
  '$button.ContextMenuStrip = $menu',
  '$button.Add_Click({ if ($label.Text -ne "VCU-CTX-HIT") { $label.Text = "VCU-CTX-LEFT" } })',
  '$form.Controls.Add($label)',
  '$form.Controls.Add($button)',
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
  $found = (& $Vcu wait --session $sid --name "VCU-CTX-TARGET" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "target not visible" }
  $ref = [string]$found.data.detail.found_ref
  $left = (& $Vcu type --session $sid --ref $ref --text "btnpix:VCU-CTX-TARGET|VCU-CTX-LEFT" | Out-String) | ConvertFrom-Json
  if ($left.ok -ne $true) { throw "left click failed $($left | ConvertTo-Json -Compress -Depth 8)" }
  if ($left.data.detail.input_path -ne "button_pixel") { throw "left path $($left.data.detail.input_path)" }
  $hitEarly = (& $Vcu wait --session $sid --name "VCU-CTX-HIT" --ms 400 | Out-String) | ConvertFrom-Json
  if ($hitEarly.ok -eq $true) { throw "left click invoked the context item" }
  $typed = (& $Vcu type --session $sid --ref $ref --text "ctxitem:VCU-CTX-TARGET|VCU-CTX-ITEM|VCU-CTX-HIT" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "context item failed $($typed | ConvertTo-Json -Compress -Depth 8)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "context_item") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $hit = (& $Vcu wait --session $sid --name "VCU-CTX-HIT" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($hit.ok -ne $true) { throw "item click did not run" }
  $again = (& $Vcu type --session $sid --ref $ref --text "ctxitem:VCU-CTX-TARGET|VCU-CTX-ITEM|VCU-CTX-HIT" | Out-String) | ConvertFrom-Json
  if ($again.ok -eq $true) { throw "already clicked item was accepted" }
  $found2 = (& $Vcu wait --session $sid --name "VCU-CTX-TARGET" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found2.ok -ne $true) { throw "target not visible after click" }
  $ref2 = [string]$found2.data.detail.found_ref
  $miss = (& $Vcu type --session $sid --ref $ref2 --text "ctxitem:VCU-CTX-TARGET|VCU-CTX-NOPE|VCU-CTX-MISS" | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "missing item was clicked" }
  $cursor1 = [VcuCursor081]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-081 OK $cid $sid ref=$ref path=context_item hit=VCU-CTX-HIT cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}