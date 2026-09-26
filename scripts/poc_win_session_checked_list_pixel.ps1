# CU-WIN-SESSION-061: session clicks the checkbox of a named checked-list row.
# Not LBC_SETCHECKSTATE. Not SendInput. Cursor must not move.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor061 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor061]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-chkpix-061-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-CHK-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 360, 240',
  '$list = New-Object System.Windows.Forms.CheckedListBox',
  '$list.Bounds = New-Object System.Drawing.Rectangle 16, 48, 300, 140',
  '$list.AccessibleName = "VCU-CHK-LIST"',
  '[void]$list.Items.Add("VCU-CHK-A")',
  '[void]$list.Items.Add("VCU-CHK-B")',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-CHK-OFF"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 16, 12, 280, 28',
  '$list.Add_ItemCheck({ if ($_.NewValue -eq [System.Windows.Forms.CheckState]::Checked -and $_.Index -eq 1) { $label.Text = "VCU-CHK-ON-B" } })',
  '$form.Controls.Add($label)',
  '$form.Controls.Add($list)',
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
  $found = (& $Vcu wait --session $sid --name "VCU-CHK-HOST" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "list not found" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "checkpix:VCU-CHK-B" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "check pixel failed $($typed | ConvertTo-Json -Compress -Depth 8)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "check_pixel") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-CHK-ON-B" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "item check event did not run" }
  $again = (& $Vcu type --session $sid --ref $ref --text "checkpix:VCU-CHK-B" | Out-String) | ConvertFrom-Json
  if ($again.ok -eq $true) { throw "already checked row was clicked" }
  $miss = (& $Vcu type --session $sid --ref $ref --text "checkpix:VCU-CHK-MISSING" | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "missing row was pixel-checked" }
  $cursor1 = [VcuCursor061]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-061 OK $cid $sid ref=$ref path=check_pixel value=VCU-CHK-B cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
