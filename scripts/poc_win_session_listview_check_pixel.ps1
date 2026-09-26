# CU-WIN-SESSION-065: session clicks the checkbox of a named list-view row.
# Helper posts the mouse message because same-process UIA swallows it.
# Not LVM_SETITEMSTATE. Not SendInput. Cursor must not move.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor065 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor065]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-lvchkpix-065-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-LVCHK-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 420, 240',
  '$lv = New-Object System.Windows.Forms.ListView',
  '$lv.View = [System.Windows.Forms.View]::Details',
  '$lv.CheckBoxes = $true',
  '$lv.Bounds = New-Object System.Drawing.Rectangle 16, 48, 300, 140',
  '[void]$lv.Columns.Add("Name", 200)',
  '[void]$lv.Items.Add("VCU-LV-A")',
  '[void]$lv.Items.Add("VCU-LV-B")',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-LV-OLD"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 16, 12, 280, 28',
  '$lv.Add_ItemCheck({ if ($_.NewValue -eq [System.Windows.Forms.CheckState]::Checked -and $_.Index -eq 1) { $label.Text = "VCU-LV-ON-B" } })',
  '$form.Controls.Add($label)',
  '$form.Controls.Add($lv)',
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
  $found = (& $Vcu wait --session $sid --name "VCU-LVCHK-HOST" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "list view host not found" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "lvcheckpix:VCU-LV-B" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "lvcheck pixel failed $($typed | ConvertTo-Json -Compress -Depth 8)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "lvcheck_pixel") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-LV-ON-B" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "item check event did not run" }
  $again = (& $Vcu type --session $sid --ref $ref --text "lvcheckpix:VCU-LV-B" | Out-String) | ConvertFrom-Json
  if ($again.ok -eq $true) { throw "already checked was accepted" }
  $miss = (& $Vcu type --session $sid --ref $ref --text "lvcheckpix:VCU-LV-MISSING" | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "missing row was checked" }
  $cursor1 = [VcuCursor065]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-065 OK $cid $sid ref=$ref path=lvcheck_pixel value=VCU-LV-B cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
