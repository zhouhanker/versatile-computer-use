# CU-WIN-SESSION-026: session wait --role matches the control role and rejects the window role.
# No SendInput. Does not move the OS cursor.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor026 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor026]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-role-026-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-ROLE-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 240, 260, 420, 180',
  '$btn = New-Object System.Windows.Forms.Button',
  '$btn.Text = "VCU-ROLE-026"',
  '$btn.Bounds = New-Object System.Drawing.Rectangle 24, 24, 180, 36',
  '$form.Controls.Add($btn)',
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
  $snap = (& $Vcu snapshot --session $sid | Out-String) | ConvertFrom-Json
  $node = @($snap.data.dom_refs | Where-Object { $_.name -eq "VCU-ROLE-026" } | Select-Object -First 1)
  if ($node.Count -ne 1) { throw "button missing from snapshot" }
  $role = [string]$node[0].role
  $found = (& $Vcu wait --session $sid --name "VCU-ROLE-026" --role $role --ms 2000 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "role wait failed for $role" }
  if ($found.data.detail.found_ref -ne $node[0].ref) { throw "ref mismatch" }
  if ($found.data.detail.os_cursor_used -ne $false) { throw "cursor used" }
  $miss = (& $Vcu wait --session $sid --name "VCU-ROLE-026" --role "ControlType.Window" --ms 600 | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "window role matched the button" }
  $cursor1 = [VcuCursor026]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-026 OK $cid $sid ref=$($node[0].ref) role=$role miss=window cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
