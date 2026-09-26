# CU-WIN-SESSION-054: session clicks the collapse icon of a named tree node.
# Mouse messages on the collapse icon. Not TVM_EXPAND. Not SendInput. Cursor must not move.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor054 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor054]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-fold-054-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-COL-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 420, 280',
  '$tree = New-Object System.Windows.Forms.TreeView',
  '$tree.Bounds = New-Object System.Drawing.Rectangle 16, 48, 280, 180',
  '$root = $tree.Nodes.Add("VCU-COL-A")',
  '[void]$root.Nodes.Add("VCU-COL-B")',
  '$root.Expand()',
  '$tree.SelectedNode = $root',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-COL-OPEN"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 16, 12, 240, 28',
  '$tree.Add_AfterCollapse({ $label.Text = "VCU-COL-CLOSED" })',
  '$form.Controls.Add($label)',
  '$form.Controls.Add($tree)',
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
  $found = (& $Vcu wait --session $sid --value "tree=VCU-COL-A" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "tree not found" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "foldicon:VCU-COL-A" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "fold icon failed $($typed.error)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "fold_icon") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-COL-CLOSED" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "fold icon event did not run" }
  $miss = (& $Vcu type --session $sid --ref $ref --text "foldicon:VCU-COL-MISSING" | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "missing node was icon-collapsed" }
  $again = (& $Vcu type --session $sid --ref $ref --text "foldicon:VCU-COL-A" | Out-String) | ConvertFrom-Json
  if ($again.ok -eq $true) { throw "already collapsed icon click was accepted" }
  $cursor1 = [VcuCursor054]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-054 OK $cid $sid ref=$ref path=fold_icon value=VCU-COL-A cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
