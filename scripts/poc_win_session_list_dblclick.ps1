# CU-WIN-SESSION-078: session double-clicks a named row in an owned list box.
# WM_LBUTTONDBLCLK on the row. Not list_pixel. Not LB_SETCURSEL. Not SendInput. Cursor must not move.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor078 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor078]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-ldbl-078-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-DBL-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 140, 120, 420, 240',
  '$list = New-Object System.Windows.Forms.ListBox',
  '$list.Items.Add("VCU-LIST-A") | Out-Null',
  '$list.Items.Add("VCU-LIST-B") | Out-Null',
  '$list.SelectedIndex = 0',
  '$list.Bounds = New-Object System.Drawing.Rectangle 24, 56, 220, 120',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-DBL-OLD"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 24, 16, 280, 28',
  '$script:dbl = 0',
  '$list.Add_SelectedIndexChanged({ })',
  '$list.Add_DoubleClick({ $script:dbl++; if ($script:dbl -eq 1) { $label.Text = "VCU-DBL-HIT" } else { $label.Text = "VCU-DBL-HIT2" } })',
  '$form.Controls.Add($label)',
  '$form.Controls.Add($list)',
  '[System.Windows.Forms.Application]::Run($form)'
) | Set-Content -Encoding ASCII -Path $formScript
$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru
$owned = $proc.Id
$sid = $null
$farScript = $null
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
  $found = (& $Vcu wait --session $sid --value "VCU-LIST-A" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "list selection A not visible" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "listdbl:VCU-LIST-B" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "list double-click failed $($typed | ConvertTo-Json -Compress -Depth 8)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "list_dblclick") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $seen = (& $Vcu wait --session $sid --value "VCU-LIST-B" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($seen.ok -ne $true) { throw "selected row not visible" }
  $hit = (& $Vcu wait --session $sid --name "VCU-DBL-HIT" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($hit.ok -ne $true) { throw "double-click event did not run" }
  $again = (& $Vcu type --session $sid --ref $ref --text "listdbl:VCU-LIST-B" | Out-String) | ConvertFrom-Json
  if ($again.ok -ne $true) { throw "second double-click on selected row failed" }
  if ($again.data.detail.input_path -ne "list_dblclick") { throw "second path $($again.data.detail.input_path)" }
  $hit2 = (& $Vcu wait --session $sid --name "VCU-DBL-HIT2" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($hit2.ok -ne $true) { throw "second double-click event did not run" }
  $miss = (& $Vcu type --session $sid --ref $ref --text "listdbl:VCU-LIST-MISSING" | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "missing row was double-clicked" }
  $single = (& $Vcu type --session $sid --ref $ref --text "listpix:VCU-LIST-A" | Out-String) | ConvertFrom-Json
  if ($single.ok -ne $true) { throw "single click fallback failed" }
  if ($single.data.detail.input_path -ne "list_pixel") { throw "single path $($single.data.detail.input_path)" }
  $still = (& $Vcu wait --session $sid --name "VCU-DBL-HIT2" --ms 800 | Out-String) | ConvertFrom-Json
  if ($still.ok -ne $true) { throw "single click changed the double-click label" }
  $farScript = Join-Path $env:TEMP ("vcu-ldbl-078-far-" + [guid]::NewGuid().ToString("N") + ".ps1")
  @(
    'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
    '$form = New-Object System.Windows.Forms.Form',
    '$form.Text = "VCU-DBL-FAR-HOST"',
    '$form.TopMost = $true',
    '$form.StartPosition = "Manual"',
    '$form.Bounds = New-Object System.Drawing.Rectangle 140, 120, 420, 240',
    '$list = New-Object System.Windows.Forms.ListBox',
    'for ($i = 0; $i -lt 40; $i++) { [void]$list.Items.Add(("VCU-ROW-{0:d2}" -f $i)) }',
    '$list.Items[30] = "VCU-LIST-FAR"',
    '$list.SelectedIndex = 0',
    '$list.Bounds = New-Object System.Drawing.Rectangle 24, 56, 220, 120',
    '$label = New-Object System.Windows.Forms.Label',
    '$label.Text = "VCU-DBL-FAR-OLD"',
    '$label.Bounds = New-Object System.Drawing.Rectangle 24, 16, 280, 28',
    '$list.Add_SelectedIndexChanged({ })',
    '$list.Add_DoubleClick({ if ([string]$list.SelectedItem -eq "VCU-LIST-FAR") { $label.Text = "VCU-DBL-FAR-HIT" } })',
    '$form.Controls.Add($label)',
    '$form.Controls.Add($list)',
    '[System.Windows.Forms.Application]::Run($form)'
  ) | Set-Content -Encoding ASCII -Path $farScript
  if ($sid) { & $Vcu session abort $sid | Out-Null; $sid = $null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue; $owned = $null }
  $far = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$farScript) -PassThru
  $owned = $far.Id
  $cid = $null
  for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Milliseconds 200
    $raw = & $Vcu app windows --json | Out-String
    $hit = @((($raw | ConvertFrom-Json).data.windows) | Where-Object { $_.id -eq "win:powershell:$owned" })
    if ($hit.Count -eq 1) { $cid = $hit[0].id; break }
  }
  if (-not $cid) { throw "far window not listed" }
  $start = (& $Vcu session start --surface desktop --app-id $cid | Out-String) | ConvertFrom-Json
  if ($start.ok -ne $true -or $start.data.stage_hud -ne $true) { throw "far stage not raised" }
  $sid = $start.data.session_id
  $found = (& $Vcu wait --session $sid --value "VCU-ROW-00" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "far list not visible" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "listdbl:VCU-LIST-FAR" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "far double-click failed $($typed | ConvertTo-Json -Compress -Depth 8)" }
  if ($typed.data.detail.input_path -ne "list_dblclick") { throw "far path $($typed.data.detail.input_path)" }
  $farHit = (& $Vcu wait --session $sid --name "VCU-DBL-FAR-HIT" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($farHit.ok -ne $true) { throw "far double-click event did not run" }
  $cursor1 = [VcuCursor078]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-078 OK $cid $sid ref=$ref path=list_dblclick hit=VCU-DBL-HIT2 far=VCU-LIST-FAR cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
  if ($farScript) { Remove-Item $farScript -ErrorAction SilentlyContinue }
}