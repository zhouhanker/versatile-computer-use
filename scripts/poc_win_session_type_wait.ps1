# CU-WIN-SESSION-021: session type writes an owned textbox, then wait sees the new value.
# No SendInput. Does not move the OS cursor.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor021 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor021]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-type-021-" + [guid]::NewGuid().ToString("N") + ".ps1")
$formLines = @(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  'Add-Type -AssemblyName System.Drawing | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-TYPE-021"',
  '$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::FixedSingle',
  '$form.TopMost = $true',
  '$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual',
  '$form.Bounds = New-Object System.Drawing.Rectangle 140, 160, 420, 180',
  '$box = New-Object System.Windows.Forms.TextBox',
  '$box.Name = "VCU-BOX-021"',
  '$box.AccessibleName = "VCU-BOX-021"',
  '$box.Text = "VCU-BOX-021"',
  '$box.Bounds = New-Object System.Drawing.Rectangle 24, 24, 240, 28',
  '$form.Controls.Add($box)',
  '[System.Windows.Forms.Application]::Run($form)'
)
[System.IO.File]::WriteAllLines($formScript, $formLines)
$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru
$owned = $proc.Id
$sid = $null
try {
  $cid = $null
  for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Milliseconds 200
    $raw = & $Vcu app windows --json | Out-String
    $obj = $raw | ConvertFrom-Json
    $hit = @($obj.data.windows | Where-Object { $_.id -eq "win:powershell:$owned" })
    if ($hit.Count -eq 1) { $cid = $hit[0].id; break }
  }
  if (-not $cid) { throw "owned window not listed" }
  $start = (& $Vcu session start --surface desktop --app-id $cid | Out-String) | ConvertFrom-Json
  if ($start.ok -ne $true -or $start.data.stage_hud -ne $true) { throw "stage not raised" }
  $sid = $start.data.session_id
  $found = (& $Vcu wait --session $sid --name "VCU-BOX-021" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "seed textbox not found" }
  $ref = [string]$found.data.detail.found_ref
  if (-not $ref) { throw "missing textbox ref" }
  $typed = (& $Vcu type --session $sid --ref $ref --text "VCU-TYPED-021" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "type failed" }
  $detail = $typed.data.detail
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  if ($detail.input_path -eq "clipboard_paste") { throw "clipboard paste is not a textbox write" }
  $seen = (& $Vcu wait --session $sid --value "VCU-TYPED-021" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($seen.ok -ne $true) { throw "typed value not visible" }
  if ($seen.data.detail.os_cursor_used -ne $false) { throw "wait moved cursor" }
  $cursor1 = [VcuCursor021]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-021 OK $cid $sid ref=$ref path=$($detail.input_path) value=VCU-TYPED-021 cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
