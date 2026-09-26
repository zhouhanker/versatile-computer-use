# CU-WIN-SESSION-064: session clicks an owned button by its client center.
# Not BM_CLICK. Not SendInput. Cursor must not move.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "gnu vcu missing: $Vcu" }
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor064b {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  public static string Pos() { POINT p; GetCursorPos(out p); return p.x.ToString() + "," + p.y.ToString(); }
}
"@
$cursor0 = [VcuCursor064b]::Pos()
$formScript = Join-Path $env:TEMP ("vcu-btnpix-064-" + [guid]::NewGuid().ToString("N") + ".ps1")
@(
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-CLICK-HOST"',
  '$form.TopMost = $true',
  '$form.StartPosition = "Manual"',
  '$form.Bounds = New-Object System.Drawing.Rectangle 160, 180, 420, 200',
  '$btn = New-Object System.Windows.Forms.Button',
  '$btn.Text = "VCU-CLICK-022"',
  '$btn.Bounds = New-Object System.Drawing.Rectangle 24, 24, 180, 36',
  '$label = New-Object System.Windows.Forms.Label',
  '$label.Text = "ready"',
  '$label.Bounds = New-Object System.Drawing.Rectangle 24, 80, 280, 28',
  '$btn.Add_Click({ $label.Text = "VCU-CLICKED-022" })',
  '$form.Controls.Add($btn)',
  '$form.Controls.Add($label)',
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
  $found = (& $Vcu wait --session $sid --name "VCU-CLICK-022" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($found.ok -ne $true) { throw "button not found" }
  $ref = [string]$found.data.detail.found_ref
  $typed = (& $Vcu type --session $sid --ref $ref --text "btnpix:VCU-CLICK-022|VCU-CLICKED-022" | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "button pixel failed $($typed | ConvertTo-Json -Compress -Depth 8)" }
  $detail = $typed.data.detail
  if ($detail.input_path -ne "button_pixel") { throw "path $($detail.input_path)" }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  $seen = (& $Vcu wait --session $sid --name "VCU-CLICKED-022" --ms 2500 | Out-String) | ConvertFrom-Json
  if ($seen.ok -ne $true) { throw "label did not change" }
  $again = (& $Vcu type --session $sid --ref $ref --text "btnpix:VCU-CLICK-022|VCU-CLICKED-022" | Out-String) | ConvertFrom-Json
  if ($again.ok -eq $true) { throw "already clicked label was accepted" }
  $miss = (& $Vcu type --session $sid --ref $ref --text "btnpix:VCU-CLICK-MISSING|VCU-CLICKED-022" | Out-String) | ConvertFrom-Json
  if ($miss.ok -eq $true) { throw "missing button was pixel-clicked" }
  $cursor1 = [VcuCursor064b]::Pos()
  if ($cursor0 -ne $cursor1) { throw "cursor moved $cursor0 -> $cursor1" }
  Write-Host "CU-WIN-SESSION-064 OK $cid $sid ref=$ref path=button_pixel value=VCU-CLICK-022 cursor=$cursor1"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  Remove-Item $formScript -ErrorAction SilentlyContinue
}
