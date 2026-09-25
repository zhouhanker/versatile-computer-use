# CU-WIN-SESSION-032: session type selects a named tab without moving the cursor.
# MSAA select plus a reflected TCN_SELCHANGE. Not SendInput.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor032 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor032]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-tab-032-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-TAB-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 480, 260',
  '$tabs = New-Object System.Windows.Forms.TabControl',
  '$tabs.Bounds = New-Object System.Drawing.Rectangle 16, 48, 420, 160',
  '$a = New-Object System.Windows.Forms.TabPage',
  '$a.Text = "VCU-TAB-A"',
  '$b = New-Object System.Windows.Forms.TabPage',
  '$b.Text = "VCU-TAB-B"',
  '$tabs.TabPages.Add($a)',
  '$tabs.TabPages.Add($b)',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-TAB-SHOW-A"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 16, 12, 220, 28',
  '$tabs.Add_SelectedIndexChanged({ if ($tabs.SelectedIndex -eq 1) { $label.Text = "VCU-TAB-SHOW-B" } else { $label.Text = "VCU-TAB-SHOW-A" } })',
  '$form.Controls.Add($label)',
  '$form.Controls.Add($tabs)',
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
  $found = (& $Vcu wait --session $sid --value "tab=VCU-TAB-A" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "tab not found" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "VCU-TAB-B" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "select failed $($typed.error)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "tab_select") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-TAB-SHOW-B" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "label did not follow selection" }
  $seen = (& $Vcu wait --session $sid --value "tab=VCU-TAB-B" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($seen.ok -ne $true) { throw "selected tab not visible" }
  $miss = (& $Vcu type --session $sid --ref $ref --text "VCU-TAB-MISSING" | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "missing tab was selected" }
  $cursor1 = [VcuCursor032]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-032 OK $cid $sid ref=$ref path=tab_select value=VCU-TAB-B cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
