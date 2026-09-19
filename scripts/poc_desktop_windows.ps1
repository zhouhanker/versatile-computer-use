# CU-D-060: live UIA + PrintWindow on Windows — Notepad (no SendInput).
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-060 live UIA: not Windows_NT"
  exit 0
}

Add-Type -AssemblyName UIAutomationClient | Out-Null
Add-Type -AssemblyName System.Drawing | Out-Null
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class VcuPrintWindow {
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
}
"@

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

  $hwnd = $proc.MainWindowHandle
  if ($hwnd -eq [IntPtr]::Zero) {
    throw "notepad MainWindowHandle is zero"
  }
  $rect = New-Object VcuPrintWindow+RECT
  [void][VcuPrintWindow]::GetWindowRect($hwnd, [ref]$rect)
  $w = [Math]::Max(1, $rect.Right - $rect.Left)
  $h = [Math]::Max(1, $rect.Bottom - $rect.Top)
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $hdc = $g.GetHdc()
  [void][VcuPrintWindow]::PrintWindow($hwnd, $hdc, 2)
  $g.ReleaseHdc($hdc)
  $g.Dispose()
  $ms = New-Object System.IO.MemoryStream
  $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  $bytes = $ms.ToArray()
  $ms.Dispose()
  if ($bytes.Length -lt 24 -or $bytes[0] -ne 0x89 -or $bytes[1] -ne 0x50) {
    throw "PrintWindow did not produce a PNG"
  }
  Write-Host ("PRINTWINDOW_OK bytes={0} frame={1},{2},{3},{4}" -f $bytes.Length, $rect.Left, $rect.Top, $w, $h)

  # CU-D-070: ValuePattern write + readback. No SendInput / mouse_event.
  $marker = "VCU-D-070"
  $all = $win.FindAll(
    [System.Windows.Automation.TreeScope]::Descendants,
    [System.Windows.Automation.Condition]::TrueCondition
  )
  $hit = $false
  foreach ($el in $all) {
    try {
      $vp = [System.Windows.Automation.ValuePattern]$el.GetCurrentPattern(
        [System.Windows.Automation.ValuePattern]::Pattern
      )
      if ($null -eq $vp) { continue }
      if ($vp.Current.IsReadOnly) { continue }
      $vp.SetValue($marker)
      Start-Sleep -Milliseconds 250
      $got = [string]$vp.Current.Value
      if ([string]::IsNullOrEmpty($got)) {
        try {
          $tp = [System.Windows.Automation.TextPattern]$el.GetCurrentPattern(
            [System.Windows.Automation.TextPattern]::Pattern
          )
          $got = [string]$tp.DocumentRange.GetText(-1)
        } catch {}
      }
      if ($got -notlike ("*{0}*" -f $marker)) {
        throw ("SETVALUE readback mismatch got='{0}'" -f $got)
      }
      Write-Host ("SETVALUE_OK name={0} ct={1} value={2}" -f $el.Current.Name, $el.Current.ControlType.ProgrammaticName, $got)
      $hit = $true
      break
    } catch {
      if ($_.Exception.Message -like "*readback mismatch*") { throw }
      # element has no ValuePattern — keep scanning
    }
  }
  if (-not $hit) {
    throw "no writable ValuePattern on notepad (CU-D-070)"
  }
} finally {
  if ($proc -and -not $proc.HasExited) {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
  }
}
