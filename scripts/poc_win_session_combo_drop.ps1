# CU-WIN-SESSION-050: session opens an owned dropdown and clicks a named item.
# Show the list, then click the visible item. Not combo_select. Not SendInput. Cursor must not move.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor050 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor050]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-drop-050-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-COMBO-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 160, 140, 420, 200',
  '$combo = New-Object System.Windows.Forms.ComboBox',
  '$combo.DropDownStyle = [System.Windows.Forms.ComboBoxStyle]::DropDownList',
  '$combo.Items.Add("VCU-COMBO-A") | Out-Null',
  '$combo.Items.Add("VCU-COMBO-B") | Out-Null',
  '$combo.SelectedIndex = 0',
  '$combo.Bounds = New-Object System.Drawing.Rectangle 24, 48, 220, 28',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-DROP-OLD"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 24, 12, 340, 28',
  '$combo.Add_SelectedIndexChanged({ $label.Text = "VCU-DROP-" + $combo.Text })',
  '$form.Controls.Add($label)',
  '$form.Controls.Add($combo)',
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
  $found = (& $Vcu wait --session $sid --name "VCU-COMBO-A" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "combo not found" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "drop:VCU-COMBO-B" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "drop select failed" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "combo_drop") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $seen = (& $Vcu wait --session $sid --name "VCU-DROP-VCU-COMBO-B" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($seen.ok -ne $true) { throw "selection event did not run" }
  $again = (& $Vcu type --session $sid --ref $ref --text "drop:VCU-COMBO-B" | Out-String) | ConvertFrom-Json
  if ($again.ok -eq $true) { throw "already selected item was accepted" }
  $miss = (& $Vcu type --session $sid --ref $ref --text "drop:VCU-COMBO-MISSING" | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "missing item was selected" }
  $cursor1 = [VcuCursor050]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-050 OK $cid $sid ref=$ref path=combo_drop selected=VCU-COMBO-B cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
