# CU-WIN-SESSION-079: session double-clicks a named ListView row without moving the cursor.
# Helper posts WM_LBUTTONDBLCLK on the row text. Not lvrow_pixel. Not LVM_SETITEMSTATE. Not SendInput.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor079 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor079]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-lvdbl-079-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-LVD-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 420, 280',
  '$lv = New-Object System.Windows.Forms.ListView',
  '$lv.View = "Details"',
  '$lv.FullRowSelect = $true',
  '$lv.MultiSelect = $false',
  '$lv.Bounds = New-Object System.Drawing.Rectangle 16, 48, 280, 180',
  '[void]$lv.Columns.Add("Name", 200)',
  '[void]$lv.Items.Add("VCU-LV-A")',
  '[void]$lv.Items.Add("VCU-LV-B")',
  '$lv.Items[0].Selected = $true',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-LVD-OLD"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 16, 12, 280, 28',
  '$script:dbl = 0',
  '$script:dblAt = [datetime]::MinValue',
  '$mark = { if ($lv.SelectedItems.Count -lt 1) { return }; if ($lv.SelectedItems[0].Text -ne "VCU-LV-B") { return }; if (([datetime]::UtcNow - $script:dblAt).TotalMilliseconds -lt 300) { return }; $script:dblAt = [datetime]::UtcNow; $script:dbl++; if ($script:dbl -eq 1) { $label.Text = "VCU-LVD-HIT" } else { $label.Text = "VCU-LVD-HIT2" } }',
  '$lv.Add_ItemSelectionChanged({ })',
  '$lv.Add_DoubleClick($mark)',
  '$lv.Add_ItemActivate($mark)',
  '$form.Controls.Add($label)',
  '$form.Controls.Add($lv)',
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
  $found = (& $Vcu wait --session $sid --value "lv=VCU-LV-A" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "listview not found" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "lvrowdbl:VCU-LV-B" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "row double-click failed $($typed | ConvertTo-Json -Compress -Depth 8)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "lvrow_dblclick") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-LVD-HIT" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "double-click event did not run" }
  $seen = (& $Vcu wait --session $sid --value "lv=VCU-LV-B" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($seen.ok -ne $true) { throw "selected row not visible" }
  $again = (& $Vcu type --session $sid --ref $ref --text "lvrowdbl:VCU-LV-B" | Out-String) | ConvertFrom-Json
  if ($again.ok -ne $true) { throw "second double-click on selected row failed" }
  if ($again.data.detail.input_path -ne "lvrow_dblclick") { throw "second path $($again.data.detail.input_path)" }
  $label2 = (& $Vcu wait --session $sid --name "VCU-LVD-HIT2" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label2.ok -ne $true) { throw "second double-click event did not run" }
  $miss = (& $Vcu type --session $sid --ref $ref --text "lvrowdbl:VCU-LV-MISSING" | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "missing row was double-clicked" }
  $single = (& $Vcu type --session $sid --ref $ref --text "lvrowpix:VCU-LV-A" | Out-String) | ConvertFrom-Json
  if ($single.ok -ne $true) { throw "single click fallback failed" }
  if ($single.data.detail.input_path -ne "lvrow_pixel") { throw "single path $($single.data.detail.input_path)" }
  $still = (& $Vcu wait --session $sid --name "VCU-LVD-HIT2" --ms 800 | Out-String) | ConvertFrom-Json
  if ($still.ok -ne $true) { throw "single click changed the double-click label" }
  $farScript = Join-Path $env:TEMP ("vcu-lvdbl-079-far-" + [guid]::NewGuid().ToString("N") + ".ps1")
  @(
    'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
    '$form = New-Object System.Windows.Forms.Form',
    '$form.Text = "VCU-LVD-FAR-HOST"',
    '$form.TopMost = $true',
    '$form.StartPosition = "Manual"',
    '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 420, 280',
    '$lv = New-Object System.Windows.Forms.ListView',
    '$lv.View = "Details"',
    '$lv.FullRowSelect = $true',
    '$lv.MultiSelect = $false',
    '$lv.Bounds = New-Object System.Drawing.Rectangle 16, 48, 280, 120',
    '[void]$lv.Columns.Add("Name", 200)',
    'for ($i = 0; $i -lt 40; $i++) { [void]$lv.Items.Add(("VCU-ROW-{0:d2}" -f $i)) }',
    '$lv.Items[30].Text = "VCU-LV-FAR"',
    '$lv.Items[0].Selected = $true',
    '$label = New-Object System.Windows.Forms.Label',
    '$label.Text = "VCU-LVD-FAR-OLD"',
    '$label.Bounds = New-Object System.Drawing.Rectangle 16, 12, 280, 28',
    '$farMark = { if ($lv.SelectedItems.Count -ge 1 -and $lv.SelectedItems[0].Text -eq "VCU-LV-FAR") { $label.Text = "VCU-LVD-FAR-HIT" } }',
    '$lv.Add_ItemSelectionChanged({ })',
    '$lv.Add_DoubleClick($farMark)',
    '$lv.Add_ItemActivate($farMark)',
    '$form.Controls.Add($label)',
    '$form.Controls.Add($lv)',
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
  $found = (& $Vcu wait --session $sid --value "lv=VCU-ROW-00" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "far listview not found" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "lvrowdbl:VCU-LV-FAR" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "far double-click failed $($typed | ConvertTo-Json -Compress -Depth 8)" }
  if ($typed.data.detail.input_path -ne "lvrow_dblclick") { throw "far path $($typed.data.detail.input_path)" }
  $farHit = (& $Vcu wait --session $sid --name "VCU-LVD-FAR-HIT" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($farHit.ok -ne $true) { throw "far double-click event did not run" }
  $cursor1 = [VcuCursor079]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-079 OK $cid $sid ref=$ref path=lvrow_dblclick hit=VCU-LVD-HIT2 far=VCU-LV-FAR cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
  if ($farScript) { Remove-Item $farScript -ErrorAction SilentlyContinue }
}