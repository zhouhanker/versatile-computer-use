# CU-D-060: live UIA smoke on Windows — Notepad window via UIAutomation (no SendInput).
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-060 live UIA: not Windows_NT"
  exit 0
}

Add-Type -AssemblyName UIAutomationClient | Out-Null
$proc = Start-Process -FilePath "notepad.exe" -PassThru
Start-Sleep -Seconds 1
try {
  $root = [System.Windows.Automation.AutomationElement]::RootElement
  $cond = New-Object System.Windows.Automation.PropertyCondition(
    [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
    [int]$proc.Id
  )
  $win = $root.FindFirst([System.Windows.Automation.TreeScope]::Children, $cond)
  if ($null -eq $win) {
    throw "UIA did not find a window for notepad pid=$($proc.Id)"
  }
  $name = $win.Current.Name
  $kids = $win.FindAll(
    [System.Windows.Automation.TreeScope]::Children,
    [System.Windows.Automation.Condition]::TrueCondition
  )
  Write-Host ("UIA_OK pid={0} title={1} children={2}" -f $proc.Id, $name, $kids.Count)
  if ($kids.Count -lt 1) {
    throw "UIA tree empty"
  }
} finally {
  if ($proc -and -not $proc.HasExited) {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
  }
}
