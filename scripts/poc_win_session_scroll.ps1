# CU-WIN-SESSION-005: session scroll moves an owned list, not the OS cursor.
# Does not use SendInput. Kills only the form this script starts.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "debug vcu missing: $Vcu" }
$formScript = Join-Path $env:TEMP ("vcu-scroll-011-" + [guid]::NewGuid().ToString("N") + ".ps1")
$formLines = @(
  'Add-Type -TypeDefinition @"',
  'using System;',
  'using System.Runtime.InteropServices;',
  'public static class VcuDpiScroll {',
  '  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);',
  '}',
  '"@',
  '[void][VcuDpiScroll]::SetProcessDpiAwarenessContext([IntPtr](-4))',
  'Add-Type -AssemblyName System.Windows.Forms | Out-Null',
  'Add-Type -AssemblyName System.Drawing | Out-Null',
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-SCROLL-011"',
  '$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual',
  '$form.Bounds = New-Object System.Drawing.Rectangle 160, 280, 360, 320',
  '$list = New-Object System.Windows.Forms.ListBox',
  '$list.Name = "VCU-SCROLL-LIST"',
  '$list.Left = 16',
  '$list.Top = 16',
  '$list.Width = 300',
  '$list.Height = 180',
  'for ($i = 1; $i -le 40; $i++) { [void]$list.Items.Add(("row-{0:d2}" -f $i)) }',
  '$form.Controls.Add($list)',
  '[System.Windows.Forms.Application]::Run($form)'
)
[System.IO.File]::WriteAllLines($formScript, $formLines, (New-Object System.Text.UTF8Encoding $false))
$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru
$owned = $proc.Id
$sid = $null
function Read-TopIndex([int]$TargetProcess) {
  $reader = Join-Path $env:TEMP ("vcu-scroll-read-" + [guid]::NewGuid().ToString("N") + ".ps1")
  $lines = @(
    'param([int]$TargetPid)',
    'Add-Type -AssemblyName UIAutomationClient | Out-Null',
    'Add-Type -TypeDefinition @"',
    'using System;',
    'using System.Runtime.InteropServices;',
    'public static class VcuListRead {',
    '  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);',
    '}',
    '"@',
    '$p = Get-Process -Id $TargetPid -ErrorAction Stop',
    '$p.Refresh()',
    '$hwnd = [IntPtr]$p.MainWindowHandle',
    '$win = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)',
    '$q = New-Object System.Collections.Queue',
    '$q.Enqueue($win)',
    '$n = 0',
    'while ($q.Count -gt 0 -and $n -lt 30) {',
    '  $el = $q.Dequeue()',
    '  $n++',
    '  $cls = [string]$el.Current.ClassName',
    '  if ($cls -like "*LISTBOX*") {',
    '    $h = [IntPtr]([int64]$el.Current.NativeWindowHandle)',
    '    $top = [int][VcuListRead]::SendMessage($h, 0x018E, [IntPtr]::Zero, [IntPtr]::Zero)',
    '    Write-Output ("top={0}" -f $top)',
    '    exit 0',
    '  }',
    '  $kids = $el.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)',
    '  foreach ($k in $kids) { $q.Enqueue($k) }',
    '}',
    'Write-Output "top=missing"'
  )
  [System.IO.File]::WriteAllLines($reader, $lines, (New-Object System.Text.UTF8Encoding $false))
  $out = (& "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -STA -ExecutionPolicy Bypass -File $reader -TargetPid $TargetProcess | Out-String).Trim()
  Remove-Item -LiteralPath $reader -Force -ErrorAction SilentlyContinue
  return $out
}
try {
  Start-Sleep -Milliseconds 800
  $cid = $null
  for ($i = 0; $i -lt 20; $i++) {
    $raw = & $Vcu app windows --json | Out-String
    $obj = $raw | ConvertFrom-Json
    $hit = @($obj.data.windows | Where-Object { $_.id -eq "win:powershell:$owned" })
    if ($hit.Count -eq 1) { $cid = $hit[0].id; break }
    Start-Sleep -Milliseconds 250
  }
  if (-not $cid) { throw "owned window not listed for pid $owned" }
  $beforeTop = Read-TopIndex $owned
  if ($beforeTop -ne "top=0") { throw "list did not start at 0: $beforeTop" }
  $start = (& $Vcu session start --surface desktop --app-id $cid | Out-String) | ConvertFrom-Json
  if ($start.ok -ne $true -or $start.data.stage_hud -ne $true) { throw "stage not raised" }
  $sid = $start.data.session_id
  Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class VcuCursor011 {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
}
"@
  $cur0 = New-Object VcuCursor011+POINT
  [void][VcuCursor011]::GetCursorPos([ref]$cur0)
  $scrolled = (& $Vcu scroll --session $sid --dy 600 | Out-String) | ConvertFrom-Json
  if ($scrolled.ok -ne $true) { throw "scroll command failed" }
  $detail = $scrolled.data.detail
  if ($null -eq $detail) { $detail = $scrolled.data }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  if ($detail.input_path -ne "uia_scroll" -and $detail.input_path -ne "wm_vscroll") { throw "path $($detail.input_path)" }
  Start-Sleep -Milliseconds 300
  $afterTop = Read-TopIndex $owned
  $cur1 = New-Object VcuCursor011+POINT
  [void][VcuCursor011]::GetCursorPos([ref]$cur1)
  if ($cur0.x -ne $cur1.x -or $cur0.y -ne $cur1.y) { throw "OS cursor moved" }
  if ($afterTop -notmatch 'top=(\d+)') { throw "bad readback $afterTop" }
  $top = [int]$Matches[1]
  if ($top -le 0) { throw "list did not scroll: before=$beforeTop after=$afterTop path=$($detail.input_path)" }
  Write-Host "CU-WIN-SESSION-005 OK $cid $sid $($detail.input_path) $beforeTop -> $afterTop cursor=$($cur0.x),$($cur0.y)"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) {
    $still = Get-Process -Id $owned -ErrorAction SilentlyContinue
    if ($still -and $still.ProcessName -eq "powershell") { Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue }
  }
  Remove-Item -LiteralPath $formScript -Force -ErrorAction SilentlyContinue
}
