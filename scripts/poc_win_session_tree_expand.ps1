# CU-WIN-SESSION-036: session type expands a named tree node without moving the cursor.
# TVM_EXPAND. Not a pixel click. Not SendInput.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor036 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor036]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-exp-036-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-EXP-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 420, 280',
  '$tree = New-Object System.Windows.Forms.TreeView',
  '$tree.Bounds = New-Object System.Drawing.Rectangle 16, 48, 280, 180',
  '$root = $tree.Nodes.Add("VCU-EXP-A")',
  '[void]$root.Nodes.Add("VCU-EXP-B")',
  '$tree.SelectedNode = $root',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-EXP-CLOSED"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 16, 12, 240, 28',
  '$tree.Add_AfterExpand({ $label.Text = "VCU-EXP-OPEN" })',
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
  $found = (& $Vcu wait --session $sid --value "tree=VCU-EXP-A" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "tree not found" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "expand:VCU-EXP-A" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "expand failed $($typed.error)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "tree_expand") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-EXP-OPEN" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "expand event did not run" }
  $miss = (& $Vcu type --session $sid --ref $ref --text "expand:VCU-EXP-MISSING" | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "missing node was expanded" }
  $cursor1 = [VcuCursor036]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-036 OK $cid $sid ref=$ref path=tree_expand value=VCU-EXP-A cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
