# CU-D-060/070: live UIA + PrintWindow + write on Windows Notepad (no SendInput).
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
using System.Text;
public static class VcuWin {
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
  [DllImport("user32.dll", CharSet=CharSet.Unicode, EntryPoint="SendMessageW")]
  public static extern IntPtr SetWindowTextMsg(IntPtr hWnd, uint Msg, IntPtr wParam, string lParam);
  [DllImport("user32.dll", CharSet=CharSet.Unicode, EntryPoint="SendMessageW")]
  public static extern int GetWindowTextMsg(IntPtr hWnd, uint Msg, int wParam, StringBuilder lParam);
  [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr hWnd);
  public const uint WM_SETTEXT = 0x000C;
  public const uint WM_GETTEXT = 0x000D;
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
  $rect = New-Object VcuWin+RECT
  [void][VcuWin]::GetWindowRect($hwnd, [ref]$rect)
  $w = [Math]::Max(1, $rect.Right - $rect.Left)
  $h = [Math]::Max(1, $rect.Bottom - $rect.Top)
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $hdc = $g.GetHdc()
  [void][VcuWin]::PrintWindow($hwnd, $hdc, 2)
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

  # CU-D-070: write marker without SendInput / mouse_event.
  $marker = "VCU-D-070"
  $all = New-Object System.Collections.Generic.List[Object]
  [void]$all.Add($win)
  $desc = $win.FindAll(
    [System.Windows.Automation.TreeScope]::Descendants,
    [System.Windows.Automation.Condition]::TrueCondition
  )
  foreach ($el in $desc) { [void]$all.Add($el) }

  $nlog = 0
  foreach ($el in $all) {
    $nlog++
    if ($nlog -gt 24) { break }
    $pats = @()
    try { $pats = @($el.GetSupportedPatterns() | ForEach-Object { $_.ProgrammaticName }) } catch {}
    Write-Host ("UIA_NODE ct={0} class={1} hwnd={2} pats={3}" -f `
      $el.Current.ControlType.ProgrammaticName, $el.Current.ClassName, `
      $el.Current.NativeWindowHandle, ($pats -join ","))
  }

  $hit = $false
  $valuePat = [System.Windows.Automation.ValuePattern]::Pattern
  foreach ($el in $all) {
    $obj = $null
    $ok = $false
    try { $ok = $el.TryGetCurrentPattern($valuePat, [ref]$obj) } catch { $ok = $false }
    if (-not $ok -or $null -eq $obj) { continue }
    try {
      if ($obj.Current.IsReadOnly) { continue }
      $obj.SetValue($marker)
      Start-Sleep -Milliseconds 250
      $got = [string]$obj.Current.Value
      if ([string]::IsNullOrEmpty($got)) {
        try {
          $tpObj = $null
          if ($el.TryGetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern, [ref]$tpObj) -and $tpObj) {
            $got = [string]$tpObj.DocumentRange.GetText(-1)
          }
        } catch {}
      }
      if ($got -notlike ("*{0}*" -f $marker)) {
        throw ("SETVALUE readback mismatch path=uia_value got='{0}'" -f $got)
      }
      Write-Host ("SETVALUE_OK path=uia_value ct={0} value={1}" -f $el.Current.ControlType.ProgrammaticName, $got)
      $hit = $true
      break
    } catch {
      if ("$($_.Exception.Message)" -like "*readback mismatch*") { throw }
    }
  }

  if (-not $hit) {
    foreach ($el in $all) {
      $nh = [int64]$el.Current.NativeWindowHandle
      if ($nh -eq 0) { continue }
      $ct = [string]$el.Current.ControlType.ProgrammaticName
      $cls = [string]$el.Current.ClassName
      $looksEdit = ($cls -eq "Edit") -or ($ct -like "*Edit*") -or ($ct -like "*Document*")
      if (-not $looksEdit) { continue }
      $h = [IntPtr]$nh
      [void][VcuWin]::SetWindowTextMsg($h, [VcuWin]::WM_SETTEXT, [IntPtr]::Zero, $marker)
      Start-Sleep -Milliseconds 250
      $len = [Math]::Max(16, [VcuWin]::GetWindowTextLength($h) + 1)
      $sb = New-Object System.Text.StringBuilder $len
      [void][VcuWin]::GetWindowTextMsg($h, [VcuWin]::WM_GETTEXT, $len, $sb)
      $got = $sb.ToString()
      if ($got -notlike ("*{0}*" -f $marker)) {
        throw ("SETVALUE readback mismatch path=wm_settext got='{0}' ct={1} class={2}" -f $got, $ct, $cls)
      }
      Write-Host ("SETVALUE_OK path=wm_settext ct={0} class={1} value={2}" -f $ct, $cls, $got)
      $hit = $true
      break
    }
  }

  if (-not $hit) {
    throw "no writable ValuePattern or Edit HWND on notepad (CU-D-070)"
  }
} finally {
  if ($proc -and -not $proc.HasExited) {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
  }
}
