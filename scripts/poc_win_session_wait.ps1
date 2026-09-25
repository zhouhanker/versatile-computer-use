# CU-WIN-SESSION-019: session wait finds an owned button name, and misses honestly.
# Does not move the OS cursor. Does not use SendInput.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor019 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor019]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-wait-019-" + [guid]::NewGuid().ToString("N") + ".ps1")
$formLines = @(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  'Add-Type -AssemblyName System.Drawing | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-WAIT-019"',
  '$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::FixedSingle',
  '$form.TopMost = $true',
  '$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual',
  '$form.Bounds = New-Object System.Drawing.Rectangle 100, 120, 360, 180',
  '$btn = New-Object System.Windows.Forms.Button',
  '$btn.Text = "VCU-WAIT-019"',
  '$btn.Bounds = New-Object System.Drawing.Rectangle 24, 24, 180, 36',
  '$form.Controls.Add($btn)',
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
  $hit = (& $Vcu wait --session $sid --name "VCU-WAIT-019" --ms 2000 | Out-String) | ConvertFrom-Json
  if ($hit.ok -ne $true) { throw "wait hit failed" }
  $detail = $hit.data.detail
  if ($detail.input_path -ne "scene_wait") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false) { throw "cursor used" }
  if (-not $detail.found_ref) { throw "missing found_ref" }
  $miss = (& $Vcu wait --session $sid --name "VCU-WAIT-MISSING" --ms 500 | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "missing name was treated as success" }
  $msg = [string]$miss.error.message
  if ($msg -notmatch "VCU-WAIT-MISSING") { throw "timeout message missing: $msg" }
  $cursor1 = [VcuCursor019]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-019 OK $cid $sid ref=$($detail.found_ref) miss=timeout cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
