# CU-WIN-SESSION-043: session click activates an owned link label without moving the cursor.
# IAccessible default action on role 30. Not BM_CLICK. Not SendInput.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor043 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor043]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-link-043-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-LINK-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 180, 160, 420, 180',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "VCU-LINK-OLD"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 16, 12, 280, 28',
  '$link = New-Object System.Windows.Forms.LinkLabel',
  '$link.Text = "VCU-LINK-B"',
  '$link.Bounds = New-Object System.Drawing.Rectangle 16, 48, 200, 28',
  '$link.Add_LinkClicked({ $label.Text = "VCU-LINK-HIT" })',
  '$plain = New-Object System.Windows.Forms.Label',
  '$plain.Text = "VCU-PLAIN"',
  '$plain.Bounds = New-Object System.Drawing.Rectangle 16, 84, 200, 28',
  '$form.Controls.Add($label)',
  '$form.Controls.Add($link)',
  '$form.Controls.Add($plain)',
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
  $found = (& $Vcu wait --session $sid --name "VCU-LINK-B" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "link not found" }
  $ref = [string]$found.data.detail.found_ref
  $click = (& $Vcu click --session $sid --ref $ref | Out-String) | ConvertFrom-Json
  if ($click.ok -ne $true) { throw "link click failed $($click.error)" }
  $detail = $click.data.detail
  if ($detail.input_path -ne "link_click") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $label = (& $Vcu wait --session $sid --name "VCU-LINK-HIT" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($label.ok -ne $true) { throw "link clicked event did not run" }
  $plain = (& $Vcu wait --session $sid --name "VCU-PLAIN" --ms 1500 | Out-String) | ConvertFrom-Json
  if ($plain.ok -ne $true) { throw "plain label not found" }
  $plainClick = (& $Vcu click --session $sid --ref $plain.data.detail.found_ref | Out-String) | ConvertFrom-Json
  if ($plainClick.ok -eq $true) { throw "plain label click was a false success" }
  $cursor1 = [VcuCursor043]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-043 OK $cid $sid ref=$ref path=link_click value=VCU-LINK-B cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
