# WIN-FIX-009: powershell.exe WinForms edit is set on the child control.
# Do not paste to the main window and call it success. Do not move the OS cursor.
# Do not touch the user Notepad or Windows Terminal. Kill only the form this script starts.
$ErrorActionPreference = "Stop"
$Vcu = Join-Path (Split-Path $PSScriptRoot -Parent) "target\debug\vcu.exe"
if (-not (Test-Path -LiteralPath $Vcu)) { throw "debug vcu missing: $Vcu" }

$marker = "vcu-own-009-" + [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
$formScript = Join-Path $env:TEMP ("vcu-own-edit-009-" + [guid]::NewGuid().ToString("N") + ".ps1")
$formLines = @(
  "Add-Type -AssemblyName System.Windows.Forms",
  '$form = New-Object System.Windows.Forms.Form',
  '$form.Text = "VCU-OWN-EDIT-009"',
  '$form.Width = 460',
  '$form.Height = 200',
  '$box = New-Object System.Windows.Forms.TextBox',
  '$box.Name = "vcuOwnBox"',
  '$box.Width = 400',
  '$box.Left = 20',
  '$box.Top = 48',
  '$form.Controls.Add($box)',
  '[System.Windows.Forms.Application]::Run($form)'
)
[System.IO.File]::WriteAllLines($formScript, $formLines, (New-Object System.Text.UTF8Encoding $false))

$proc = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @("-NoProfile","-STA","-ExecutionPolicy","Bypass","-File",$formScript) -PassThru
$owned = $proc.Id
$sid = $null
try {
  $cid = $null
  for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Milliseconds 300
    $raw = & $Vcu app windows --json | Out-String
    $obj = $raw | ConvertFrom-Json
    $hit = @($obj.data.windows | Where-Object { $_.id -eq "win:powershell:$owned" } | Select-Object -First 1)
    if ($hit.Count -eq 1 -and $hit[0].id) { $cid = $hit[0].id; break }
  }
  if (-not $cid) { throw "owned form not listed for pid $owned" }

  $start = (& $Vcu session start --surface desktop --app-id $cid | Out-String) | ConvertFrom-Json
  if ($start.ok -ne $true) { throw "session start failed" }
  $sid = $start.data.session_id
  $snap = (& $Vcu snapshot --session $sid | Out-String) | ConvertFrom-Json
  $refs = @($snap.data.dom_refs)
  $edit = @($refs | Where-Object { $_.role -like "*EDIT*" -or $_.role -like "*Edit*" } | Select-Object -First 1)
  if ($edit.Count -ne 1) {
    $edit = @($refs | Where-Object { $_.ref -ne "e1" } | Select-Object -First 1)
  }
  if ($edit.Count -ne 1) { throw "edit ref not found" }

  $typed = (& $Vcu type --session $sid --text $marker --ref $edit[0].ref | Out-String) | ConvertFrom-Json
  if ($typed.ok -ne $true) { throw "type failed" }
  $detail = $typed.data.detail
  if ($null -eq $detail) { $detail = $typed.data }
  if ($detail.os_cursor_used -ne $false -or $detail.hid_injected -ne $false) { throw "cursor or hid used" }
  if ($detail.input_path -eq "clipboard_paste") { throw "powershell GUI used console paste" }
  if ($detail.input_path -ne "uia_set_value" -and $detail.input_path -ne "wm_settext") { throw "unexpected path $($detail.input_path)" }

  $readLines = @(
    'Add-Type -TypeDefinition @"'
    'using System;'
    'using System.Runtime.InteropServices;'
    'using System.Text;'
    'public static class VcuRead009 {'
    '  [DllImport("user32.dll", CharSet=CharSet.Unicode)]'
    '  public static extern int SendMessage(IntPtr hWnd, uint Msg, int wParam, StringBuilder lParam);'
    '}'
    '"@ | Out-Null'
    'Add-Type -AssemblyName UIAutomationClient | Out-Null'
    ('$p = Get-Process -Id {0} -ErrorAction Stop' -f $owned)
    '$p.Refresh()'
    '$hwnd = [IntPtr]$p.MainWindowHandle'
    'if ($hwnd -eq [IntPtr]::Zero) { "no-hwnd"; exit 0 }'
    '$win = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)'
    '$q = New-Object System.Collections.Queue'
    '$q.Enqueue($win)'
    '$n = 0'
    'while ($q.Count -gt 0 -and $n -lt 40) {'
    '  $el = $q.Dequeue()'
    '  $n++'
    '  if ($el.Current.ClassName -like "*EDIT*") {'
    '    $h = [IntPtr]([int64]$el.Current.NativeWindowHandle)'
    '    $sb = New-Object System.Text.StringBuilder 1024'
    '    [void][VcuRead009]::SendMessage($h, 13, 1024, $sb)'
    '    "value:$($sb.ToString())"'
    '    exit 0'
    '  }'
    '  $kids = $el.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)'
    '  foreach ($k in $kids) { $q.Enqueue($k) }'
    '}'
    '"empty"'
  )
  $readPath = Join-Path $env:TEMP ("vcu-own-read-009-" + [guid]::NewGuid().ToString("N") + ".ps1")
  [System.IO.File]::WriteAllLines($readPath, $readLines, (New-Object System.Text.UTF8Encoding $false))
  $got = (& "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -STA -ExecutionPolicy Bypass -File $readPath | Out-String).Trim()
  Remove-Item -LiteralPath $readPath -Force -ErrorAction SilentlyContinue
  if ($got -ne "value:$marker") { throw "readback [$got] expected value:$marker" }

  Write-Host "CU-WIN-FIX-009 OK $cid $sid $($edit[0].ref) $($detail.input_path) $marker"
} finally {
  if ($sid) { & $Vcu session abort $sid | Out-Null }
  if ($owned) {
    $still = Get-Process -Id $owned -ErrorAction SilentlyContinue
    if ($still -and $still.ProcessName -eq "powershell") {
      Stop-Process -Id $owned -Force -ErrorAction SilentlyContinue
    }
  }
  Remove-Item -LiteralPath $formScript -Force -ErrorAction SilentlyContinue
}
