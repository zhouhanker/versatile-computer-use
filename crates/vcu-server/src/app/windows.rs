//! Windows app adapter (PowerShell list on Windows hosts).
use super::{is_denied_app, AppBackend, AppElement, AppSnapshot, AppTarget};
use async_trait::async_trait;
use vcu_core::{ErrorCode, VcuError, VcuResult};

pub struct WindowsAppBackend {
    pub allowlist: Vec<String>,
}

impl WindowsAppBackend {
    pub fn new() -> Self {
        Self {
            allowlist: vec![
                "notepad".into(),
                "explorer".into(),
                "msedge".into(),
                "chrome".into(),
                "windows terminal".into(),
                "windowsterminal".into(),
                "powershell".into(),
                "cmd".into(),
                "conhost".into(),
                "calc".into(),
                "calculator".into(),
                "systemsettings".into(),
            ],
        }
    }

    #[allow(dead_code)]
    fn allowed(&self, name: &str) -> bool {
        if is_denied_app(name) {
            return false;
        }
        if self.allowlist.is_empty() {
            return true;
        }
        let lower = name.to_lowercase();
        self.allowlist
            .iter()
            .any(|a| lower.contains(&a.to_lowercase()))
    }

    #[cfg(windows)]
    fn run_powershell(script: &str) -> VcuResult<String> {
        use std::process::Command;
        let path = std::env::temp_dir().join(format!("vcu-ps-{}.ps1", ulid::Ulid::new()));
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(powershell_utf8_prelude().as_bytes());
        bytes.extend_from_slice(script.as_bytes());
        std::fs::write(&path, &bytes).map_err(|e| {
            VcuError::with_detail(
                ErrorCode::Internal,
                "write powershell script",
                e.to_string(),
            )
        })?;
        let ps = {
            let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
            std::path::PathBuf::from(root).join(r"System32\WindowsPowerShell\v1.0\powershell.exe")
        };
        let output = Command::new(ps)
            .args([
                "-NoProfile",
                "-STA",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                path.to_string_lossy().as_ref(),
            ])
            .output();
        let _ = std::fs::remove_file(&path);
        let output = output.map_err(|e| {
            VcuError::with_detail(
                ErrorCode::Internal,
                "powershell spawn failed",
                e.to_string(),
            )
        })?;
        if !output.status.success() {
            let err = decode_powershell_output(&output.stderr);
            return Err(VcuError::with_detail(
                ErrorCode::ActionFailed,
                "powershell failed",
                err,
            ));
        }
        Ok(decode_powershell_output(&output.stdout))
    }
}

fn powershell_utf8_prelude() -> &'static str {
    r#"
try {
  $utf8 = New-Object System.Text.UTF8Encoding $false
  $OutputEncoding = $utf8
  [Console]::OutputEncoding = $utf8
} catch {}
"#
}

pub fn decode_powershell_output(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    String::from_utf8_lossy(bytes).trim().to_string()
}

/// Parse `name\tpid\ttitle` lines from the Windows process listing script.

/// Packaged apps such as Calculator show their window on ApplicationFrameHost.
/// Alias only a matching title. Never allowlist the host process itself.
pub fn frame_host_alias(title: &str) -> Option<&'static str> {
    let t = title.trim();
    if t.is_empty() {
        return None;
    }
    let lower = t.to_lowercase();
    if lower.contains("calculator") || lower == "calc" || t.contains("计算器") {
        return Some("Calculator");
    }
    None
}

fn is_frame_host(name: &str) -> bool {
    name.eq_ignore_ascii_case("ApplicationFrameHost")
}

fn is_packaged_calc_stub(name: &str, title: &str) -> bool {
    let n = name.to_ascii_lowercase();
    let t = title.trim();
    (n == "calculatorapp" || n == "calc" || n == "calculator")
        && (t.is_empty() || t.eq_ignore_ascii_case(name))
}

pub fn parse_process_list_lines(raw: &str, allowed: impl Fn(&str) -> bool) -> Vec<AppTarget> {
    let has_calc_window = raw.lines().any(|line| {
        let mut parts = if line.contains('|') {
            line.split('|')
        } else {
            line.split('\t')
        };
        let name = parts.next().unwrap_or("").trim();
        let _pid = parts.next();
        let title = parts.next().unwrap_or("").trim();
        is_frame_host(name) && frame_host_alias(title) == Some("Calculator")
    });
    let mut out = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let mut parts = if line.contains('|') {
            line.split('|')
        } else {
            line.split('\t')
        };
        let name = parts.next().unwrap_or("").trim();
        let pid = parts.next().and_then(|s| s.trim().parse().ok());
        let title = parts.next().unwrap_or(name).trim();
        if name.is_empty() {
            continue;
        }
        if has_calc_window && is_packaged_calc_stub(name, title) {
            continue;
        }
        if is_denied_app(name) || is_denied_app(title) {
            continue;
        }
        let mut listed = name.to_string();
        if is_frame_host(name) {
            match frame_host_alias(title) {
                Some(alias) => listed = alias.to_string(),
                None => continue,
            }
        }
        if !allowed(&listed) && !allowed(title) {
            continue;
        }
        out.push(AppTarget {
            id: format!(
                "win:{}:{}",
                listed.replace(' ', "_"),
                pid.unwrap_or(idx as i32)
            ),
            title: if title.is_empty() {
                name.to_string()
            } else {
                title.to_string()
            },
            bundle_or_exe: listed.clone(),
            pid,
            allowed: true,
            browser_profile: None,
        });
    }
    out
}

/// PowerShell: UIA tree walk. No SendInput / mouse_event.
fn pid_from_win_id(id: &str) -> Option<i32> {
    id.rsplit(':').next()?.parse().ok()
}

/// Newline refusal and paste eligibility by process name.
/// Not proof of a console window: powershell.exe can host a WinForms GUI.
/// Paste still requires a console window class inside the set-value script.
fn windows_console_app(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("cmd")
        || n.contains("conhost")
        || n.contains("powershell")
        || n.contains("pwsh")
        || n.contains("windows terminal")
        || n.contains("windowsterminal")
        || n.contains("wt")
}

/// Resolve HWND for processes whose window is owned by conhost (cmd.exe MainWindowHandle is often 0).
fn hwnd_resolve_ps() -> &'static str {
    r#"
if (-not ("VcuHwndResolve" -as [type])) {
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class VcuHwndResolve {
  public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc lpEnumFunc, IntPtr lParam);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("kernel32.dll")] public static extern bool AttachConsole(uint dwProcessId);
  [DllImport("kernel32.dll")] public static extern bool FreeConsole();
  [DllImport("kernel32.dll")] public static extern IntPtr GetConsoleWindow();
  static IntPtr found = IntPtr.Zero;
  static uint want = 0;
  public static bool VisibleOwnedCb(IntPtr h, IntPtr l) {
    uint wpid = 0;
    GetWindowThreadProcessId(h, out wpid);
    if (wpid == want && IsWindowVisible(h)) { found = h; return false; }
    return true;
  }
  public static bool OwnedCb(IntPtr h, IntPtr l) {
    uint wpid = 0;
    GetWindowThreadProcessId(h, out wpid);
    if (wpid == want) { found = h; return false; }
    return true;
  }
  public static IntPtr ForPid(uint pid) {
    found = IntPtr.Zero;
    want = pid;
    EnumWindows(new EnumProc(VisibleOwnedCb), IntPtr.Zero);
    if (found != IntPtr.Zero) return found;
    EnumWindows(new EnumProc(OwnedCb), IntPtr.Zero);
    if (found != IntPtr.Zero) return found;
    FreeConsole();
    if (AttachConsole(pid)) {
      found = GetConsoleWindow();
      FreeConsole();
      if (found != IntPtr.Zero) return found;
    }
    return IntPtr.Zero;
  }
}
"@
}
function Get-VcuHwnd([int]$ProcessId) {
  $p = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
  if ($null -eq $p) { return [IntPtr]::Zero }
  try { $p.Refresh() } catch {}
  if ($p.MainWindowHandle -ne [IntPtr]::Zero) { return [IntPtr]$p.MainWindowHandle }
  $owned = [VcuHwndResolve]::ForPid([uint32]$ProcessId)
  if ($owned -ne [IntPtr]::Zero) { return $owned }
  # Store Notepad: Start-Process notepad.exe often returns a windowless stub.
  # Use the unique visible Notepad window. Do not guess when several are open.
  $name = [string]$p.ProcessName
  if ($name -eq "Notepad") {
    $wins = @(Get-Process -Name Notepad -ErrorAction SilentlyContinue | Where-Object {
      $_.Id -ne $ProcessId -and $_.MainWindowHandle -ne 0
    })
    $near = @()
    foreach ($w in $wins) {
      try {
        $delta = [Math]::Abs(($w.StartTime - $p.StartTime).TotalSeconds)
        if ($delta -lt 8) { $near += $w }
      } catch {}
    }
    if ($near.Count -eq 1) { return [IntPtr]$near[0].MainWindowHandle }
    if ($wins.Count -eq 1) { return [IntPtr]$wins[0].MainWindowHandle }
  }
  return [IntPtr]::Zero
}
function Wait-VcuHwnd([int]$ProcessId) {
  $hwnd = [IntPtr]::Zero
  for ($t = 0; $t -lt 15; $t++) {
    $hwnd = Get-VcuHwnd $ProcessId
    if ($hwnd -ne [IntPtr]::Zero) { return $hwnd }
    Start-Sleep -Milliseconds 200
  }
  return $hwnd
}
"#
}

pub fn uia_tree_script(pid: i32, max_nodes: i32) -> String {
    let mut s = hwnd_resolve_ps().to_string();
    s.push_str(&format!(
        r#"
Add-Type -AssemblyName UIAutomationClient | Out-Null
$targetPid = {pid}
$max = {max}
$hwnd = Wait-VcuHwnd $targetPid
if ($hwnd -eq [IntPtr]::Zero) {{ 'MISSING'; exit 0 }}
$win = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$hwnd)
if ($null -eq $win) {{
  $proc = Get-Process -Id $targetPid -ErrorAction SilentlyContinue
  $nm = if ($null -ne $proc) {{ $proc.ProcessName }} else {{ 'window' }}
  'e1|ControlType.Window|{{0}}|0,0,0,0|ConsoleWindowClass' -f $nm
  exit 0
}}
$q = New-Object System.Collections.Queue
$q.Enqueue($win)
$n = 0
while ($q.Count -gt 0 -and $n -lt $max) {{
  $el = $q.Dequeue()
  $n++
  $ct = $el.Current.ControlType.ProgrammaticName
  $nm = ($el.Current.Name -replace '[\r\n\|]', ' ')
  if ([string]::IsNullOrWhiteSpace($nm)) {{ $nm = ($el.Current.AutomationId -replace '[\r\n\|]', ' ') }}
  if ([string]::IsNullOrWhiteSpace($nm)) {{
    try {{
      $leg = $el.GetCurrentPattern([System.Windows.Automation.LegacyIAccessiblePattern]::Pattern)
      $nm = ($leg.Current.Name -replace '[\r\n\|]', ' ')
    }} catch {{}}
  }}
  $cls = ($el.Current.ClassName -replace '[\r\n\|]', ' ')
  $val = ''
  try {{
    $vp = $el.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
    $val = [string]$vp.Current.Value
  }} catch {{}}
  if ([string]::IsNullOrWhiteSpace($val)) {{
    $nh = [int64]$el.Current.NativeWindowHandle
    if ($nh -ne 0) {{
      if (-not ("Vcu.VcuGetText190" -as [type])) {{
        $sig = @'
[DllImport("user32.dll", CharSet=CharSet.Unicode)]
public static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder lpString, int nMaxCount);
'@
        Add-Type -MemberDefinition $sig -Name VcuGetText190 -Namespace Vcu | Out-Null
      }}
      $sb = New-Object System.Text.StringBuilder 512
      [void][Vcu.VcuGetText190]::GetWindowText([IntPtr]$nh, $sb, 512)
      $val = $sb.ToString()
    }}
  }}
  try {{
    $nhAcc = [int64]$el.Current.NativeWindowHandle
    if ($nhAcc -ne 0) {{
      if (-not ("Vcu.VcuAccState" -as [type])) {{
        Add-Type -MemberDefinition '[DllImport("oleacc.dll")] public static extern int AccessibleObjectFromWindow(IntPtr hwnd, uint id, ref Guid iid, [MarshalAs(UnmanagedType.IUnknown)] out object acc);' -Name VcuAccState -Namespace Vcu | Out-Null
      }}
      $iid = [Guid]"618736e0-3c3d-11cf-810c-00aa00389b71"
      $acc = $null
      $hr = [Vcu.VcuAccState]::AccessibleObjectFromWindow([IntPtr]$nhAcc, [uint32]4294967292, [ref]$iid, [ref]$acc)
      if ($hr -eq 0 -and $null -ne $acc) {{
        $roleId = [int]$acc.GetType().InvokeMember("accRole", [Reflection.BindingFlags]::GetProperty, $null, $acc, @(0))
        $state = [int]$acc.GetType().InvokeMember("accState", [Reflection.BindingFlags]::GetProperty, $null, $acc, @(0))
        if ($roleId -eq 44 -or $roleId -eq 45) {{
          if (($state -band 16) -ne 0) {{ $mark = 'toggle-on' }} else {{ $mark = 'toggle-off' }}
          if ([string]::IsNullOrWhiteSpace($val)) {{ $val = $mark }} else {{ $val = "$val $mark" }}
        }}
      }}
    }}
  }} catch {{}}
  if ($cls -match 'LISTBOX') {{
    $nhList = [int64]$el.Current.NativeWindowHandle
    if ($nhList -ne 0) {{
      if (-not ("Vcu.VcuListRead" -as [type])) {{
        Add-Type -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam); [DllImport("user32.dll", CharSet=CharSet.Unicode, EntryPoint="SendMessage")] public static extern int SendMessageGetText(IntPtr hWnd, uint msg, int wParam, System.Text.StringBuilder lParam);' -Name VcuListRead -Namespace Vcu | Out-Null
      }}
      $cur = [int][Vcu.VcuListRead]::SendMessage([IntPtr]$nhList, 0x0188, [IntPtr]::Zero, [IntPtr]::Zero)
      if ($cur -ge 0) {{
        $sbList = New-Object System.Text.StringBuilder 512
        [void][Vcu.VcuListRead]::SendMessageGetText([IntPtr]$nhList, 0x0189, $cur, $sbList)
        $picked = $sbList.ToString()
        if ($picked) {{
          if ([string]::IsNullOrWhiteSpace($val)) {{ $val = $picked }} else {{ $val = "$val $picked" }}
        }}
      }}
    }}
  }}
  if ($cls -match 'TRACKBAR') {{
    $nhTrack = [int64]$el.Current.NativeWindowHandle
    if ($nhTrack -ne 0) {{
      if (-not ("Vcu.VcuTrackRead" -as [type])) {{
        Add-Type -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);' -Name VcuTrackRead -Namespace Vcu | Out-Null
      }}
      $pos = [int][Vcu.VcuTrackRead]::SendMessage([IntPtr]$nhTrack, 0x0400, [IntPtr]::Zero, [IntPtr]::Zero)
      $mark = "track=$pos"
      if ([string]::IsNullOrWhiteSpace($val)) {{ $val = $mark }} else {{ $val = "$val $mark" }}
    }}
  }}
  if ($cls -match 'TabControl') {{
    $nhTab = [int64]$el.Current.NativeWindowHandle
    if ($nhTab -ne 0) {{
      if (-not ("Vcu.VcuTabRead" -as [type])) {{
        Add-Type -MemberDefinition '[DllImport("oleacc.dll")] public static extern int AccessibleObjectFromWindow(IntPtr hwnd, uint id, ref Guid iid, [MarshalAs(UnmanagedType.IUnknown)] out object acc);' -Name VcuTabRead -Namespace Vcu | Out-Null
      }}
      $iidTab = [Guid]"618736e0-3c3d-11cf-810c-00aa00389b71"
      $accTab = $null
      $hrTab = [Vcu.VcuTabRead]::AccessibleObjectFromWindow([IntPtr]$nhTab, [uint32]4294967292, [ref]$iidTab, [ref]$accTab)
      if ($hrTab -eq 0 -and $null -ne $accTab) {{
        $tabCount = [int]$accTab.GetType().InvokeMember("accChildCount", [Reflection.BindingFlags]::GetProperty, $null, $accTab, $null)
        for ($ti = 1; $ti -le $tabCount; $ti++) {{
          $roleTab = [int]$accTab.GetType().InvokeMember("accRole", [Reflection.BindingFlags]::GetProperty, $null, $accTab, @($ti))
          if ($roleTab -ne 37) {{ continue }}
          $stateTab = [int]$accTab.GetType().InvokeMember("accState", [Reflection.BindingFlags]::GetProperty, $null, $accTab, @($ti))
          if (($stateTab -band 2) -eq 0) {{ continue }}
          $nameTab = [string]$accTab.GetType().InvokeMember("accName", [Reflection.BindingFlags]::GetProperty, $null, $accTab, @($ti))
          if ($nameTab) {{
            $markTab = "tab=$nameTab"
            if ([string]::IsNullOrWhiteSpace($val)) {{ $val = $markTab }} else {{ $val = "$val $markTab" }}
          }}
          break
        }}
      }}
    }}
  }}

  $val = ($val -replace '[\r\n\|]', ' ')
  if ($val.Length -gt 200) {{ $val = $val.Substring(0, 200) }}
  $r = $el.Current.BoundingRectangle
  '{{0}}|{{1}}|{{2}}|{{3}},{{4}},{{5}},{{6}}|{{7}}|{{8}}' -f ("e$n"), $ct, $nm, [int]$r.X, [int]$r.Y, [int]$r.Width, [int]$r.Height, $cls, $val
  if ($n -ge $max) {{ break }}
  $kids = $el.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
  foreach ($k in $kids) {{ $q.Enqueue($k) }}
}}
"#,
        pid = pid,
        max = max_nodes
    ));
    s
}

/// PowerShell: InvokePattern on the Nth UIA node (eN). No SendInput.
pub fn uia_invoke_script(pid: i32, eref: &str) -> String {
    let n = eref.trim_start_matches('e').parse::<i32>().unwrap_or(0);
    let mut s = hwnd_resolve_ps().to_string();
    s.push_str(&format!(
        r#"
Add-Type -AssemblyName UIAutomationClient | Out-Null
$targetPid = {pid}
$want = {n}
$hwnd = Wait-VcuHwnd $targetPid
if ($hwnd -eq [IntPtr]::Zero) {{ 'not-found'; exit 0 }}
$win = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$hwnd)
if ($null -eq $win) {{ 'not-found'; exit 0 }}
$q = New-Object System.Collections.Queue
$q.Enqueue($win)
$i = 0
while ($q.Count -gt 0) {{
  $el = $q.Dequeue()
  $i++
  if ($i -eq $want) {{
    $pat = [System.Windows.Automation.InvokePattern]::Pattern
    try {{
      $inv = $el.GetCurrentPattern($pat)
      $inv.Invoke()
      'ok:uia_invoke'
      exit 0
    }} catch {{}}
    try {{
      $leg = $el.GetCurrentPattern([System.Windows.Automation.LegacyIAccessiblePattern]::Pattern)
      $leg.DoDefaultAction()
      'ok:legacy_invoke'
      exit 0
    }} catch {{}}
    try {{
      $sel = $el.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern)
      $sel.Select()
      'ok:selection_item'
      exit 0
    }} catch {{}}
    try {{
      $exp = $el.GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern)
      $exp.Expand()
      'ok:expand_collapse'
      exit 0
    }} catch {{}}
    $inner = $el.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($k in $inner) {{
      try {{
        $ktog = $k.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
        $ktog.Toggle()
        'ok:toggle'
        exit 0
      }} catch {{}}
      try {{
        $kinv = $k.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $kinv.Invoke()
        'ok:uia_invoke'
        exit 0
      }} catch {{}}
      try {{
        $kexp = $k.GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern)
        $kexp.Expand()
        'ok:expand_collapse'
        exit 0
      }} catch {{}}
    }}
    if (-not ("Vcu.VcuInvoke100" -as [type])) {{
      $sig = @'
[DllImport("user32.dll")]
public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
'@
      Add-Type -MemberDefinition $sig -Name VcuInvoke100 -Namespace Vcu | Out-Null
    }}
    $nh = [int64]$el.Current.NativeWindowHandle
    if ($nh -ne 0) {{
      [void][Vcu.VcuInvoke100]::SendMessage([IntPtr]$nh, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)
      'ok:bm_click'
      exit 0
    }}
    'error:no-invoke-pattern'
    exit 0
  }}
  $kids = $el.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
  foreach ($k in $kids) {{ $q.Enqueue($k) }}
}}
'not-found'
"#,
        pid = pid,
        n = n
    ));
    s
}

/// PowerShell: ValuePattern.SetValue on eN, else WM_SETTEXT. Consoles skip WM_SETTEXT (false green) and paste. No SendInput.
pub fn uia_set_value_script(pid: i32, eref: &str, value: &str) -> String {
    uia_set_value_script_for(pid, eref, value, false)
}

fn uia_set_value_script_for(pid: i32, eref: &str, value: &str, console: bool) -> String {
    let n = eref.trim_start_matches('e').parse::<i32>().unwrap_or(0);
    let val = value.replace('\'', "''");
    let allow = if console { "1" } else { "0" };
    let body = r#"
Add-Type -AssemblyName UIAutomationClient | Out-Null
if (-not ("Vcu.VcuSetValue070" -as [type])) {
  $sig = @'
[DllImport("user32.dll", CharSet=CharSet.Unicode)]
public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, string lParam);
[DllImport("user32.dll", CharSet=CharSet.Unicode, EntryPoint="SendMessage")]
public static extern int SendMessageGetText(IntPtr hWnd, uint Msg, int wParam, System.Text.StringBuilder lParam);
[DllImport("user32.dll", CharSet=CharSet.Unicode)]
public static extern int GetClassName(IntPtr hWnd, System.Text.StringBuilder lpClassName, int nMaxCount);
'@
  Add-Type -MemberDefinition $sig -Name VcuSetValue070 -Namespace Vcu | Out-Null
}
if (-not ("Vcu.VcuPaste140" -as [type])) {
  $sig = @'
[DllImport("user32.dll")]
public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
'@
  Add-Type -MemberDefinition $sig -Name VcuPaste140 -Namespace Vcu | Out-Null
}
function Get-VcuClass([IntPtr]$h) {
  if ($h -eq [IntPtr]::Zero) { return '' }
  $sb = New-Object System.Text.StringBuilder 256
  [void][Vcu.VcuSetValue070]::GetClassName($h, $sb, 256)
  return $sb.ToString()
}
function Test-VcuConsoleClass([string]$cls) {
  return $cls -eq 'ConsoleWindowClass' -or $cls -eq 'CASCADIA_HOSTING_WINDOW_CLASS' -or $cls -eq 'PseudoConsoleWindow'
}
function Test-VcuEditClass([string]$cls) {
  if ([string]::IsNullOrWhiteSpace($cls)) { return $false }
  $c = $cls.ToLowerInvariant()
  if ($c -eq 'edit') { return $true }
  if ($c.Contains('windowsforms10.edit')) { return $true }
  if ($c.Contains('richedit')) { return $true }
  if ($c.Contains('textbox')) { return $true }
  return $false
}
function Test-VcuComboClass([string]$cls) {
  if ([string]::IsNullOrWhiteSpace($cls)) { return $false }
  return $cls.ToLowerInvariant().Contains('combobox')
}
function Select-VcuCombo([IntPtr]$h, [string]$expect) {
  $count = [int][Vcu.VcuPaste140]::SendMessage($h, 0x0146, [IntPtr]::Zero, [IntPtr]::Zero)
  for ($i = 0; $i -lt $count; $i++) {
    $sb = New-Object System.Text.StringBuilder 512
    [void][Vcu.VcuSetValue070]::SendMessageGetText($h, 0x0148, $i, $sb)
    if ($sb.ToString() -eq $expect) {
      [void][Vcu.VcuPaste140]::SendMessage($h, 0x014E, [IntPtr]$i, [IntPtr]::Zero)
      'ok:combo_select'
      exit 0
    }
  }
}
function Test-VcuListClass([string]$cls) {
  if ([string]::IsNullOrWhiteSpace($cls)) { return $false }
  return $cls.ToLowerInvariant().Contains('listbox')
}
function Test-VcuTrackClass([string]$cls) {
  if ([string]::IsNullOrWhiteSpace($cls)) { return $false }
  return $cls.ToLowerInvariant().Contains('trackbar')
}
function Select-VcuTrack([IntPtr]$h, [string]$expect) {
  $pos = 0
  if (-not [int]::TryParse($expect, [ref]$pos)) { return }
  $min = [int][Vcu.VcuPaste140]::SendMessage($h, 0x0401, [IntPtr]::Zero, [IntPtr]::Zero)
  $max = [int][Vcu.VcuPaste140]::SendMessage($h, 0x0402, [IntPtr]::Zero, [IntPtr]::Zero)
  if ($pos -lt $min -or $pos -gt $max) { return }
  [void][Vcu.VcuPaste140]::SendMessage($h, 0x0405, [IntPtr]1, [IntPtr]$pos)
  $thumb = [IntPtr](($pos -shl 16) -bor 4)
  [void][Vcu.VcuPaste140]::SendMessage($h, 0x2114, $thumb, $h)
  [void][Vcu.VcuPaste140]::SendMessage($h, 0x2114, [IntPtr]8, $h)
  $got = [int][Vcu.VcuPaste140]::SendMessage($h, 0x0400, [IntPtr]::Zero, [IntPtr]::Zero)
  if ($got -eq $pos) {
    'ok:track_select'
    exit 0
  }
}
function Select-VcuList([IntPtr]$h, [string]$expect) {
  $count = [int][Vcu.VcuPaste140]::SendMessage($h, 0x018B, [IntPtr]::Zero, [IntPtr]::Zero)
  for ($i = 0; $i -lt $count; $i++) {
    $sb = New-Object System.Text.StringBuilder 512
    [void][Vcu.VcuSetValue070]::SendMessageGetText($h, 0x0189, $i, $sb)
    if ($sb.ToString() -eq $expect) {
      [void][Vcu.VcuPaste140]::SendMessage($h, 0x0186, [IntPtr]$i, [IntPtr]::Zero)
      $wp = [IntPtr](1 -shl 16)
      [void][Vcu.VcuPaste140]::SendMessage($h, 0x2111, $wp, $h)
      'ok:list_select'
      exit 0
    }
  }
}
function Test-VcuTabClass([string]$cls) {
  if ([string]::IsNullOrWhiteSpace($cls)) { return $false }
  $c = $cls.ToLowerInvariant()
  return $c.Contains("tabcontrol") -or $c.Contains("systabcontrol32")
}
function Select-VcuTab([IntPtr]$h, [string]$expect, [int]$ownerPid) {
  if (-not ("Vcu.VcuTabSelect" -as [type])) {
    $sig = @"
[DllImport("oleacc.dll")] public static extern int AccessibleObjectFromWindow(IntPtr hwnd, uint id, ref Guid iid, [MarshalAs(UnmanagedType.IUnknown)] out object acc);
[DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hWnd, int msg, IntPtr wParam, IntPtr lParam);
[DllImport("kernel32.dll")] public static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
[DllImport("kernel32.dll")] public static extern IntPtr VirtualAllocEx(IntPtr proc, IntPtr addr, UIntPtr size, uint type, uint protect);
[DllImport("kernel32.dll")] public static extern bool WriteProcessMemory(IntPtr proc, IntPtr addr, IntPtr buffer, UIntPtr size, out UIntPtr written);
[DllImport("kernel32.dll")] public static extern bool VirtualFreeEx(IntPtr proc, IntPtr addr, UIntPtr size, uint type);
[DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr handle);
[StructLayout(LayoutKind.Sequential)] public struct NMHDR { public IntPtr hwndFrom; public IntPtr idFrom; public int code; }
public static bool Notify(int pid, IntPtr hwnd) {
  IntPtr proc = OpenProcess(0x0438, false, pid);
  if (proc == IntPtr.Zero) return false;
  int size = System.Runtime.InteropServices.Marshal.SizeOf(typeof(NMHDR));
  IntPtr remote = VirtualAllocEx(proc, IntPtr.Zero, (UIntPtr)size, 0x1000, 0x04);
  if (remote == IntPtr.Zero) { CloseHandle(proc); return false; }
  NMHDR hdr = new NMHDR();
  hdr.hwndFrom = hwnd;
  hdr.code = -551;
  IntPtr local = System.Runtime.InteropServices.Marshal.AllocHGlobal(size);
  System.Runtime.InteropServices.Marshal.StructureToPtr(hdr, local, false);
  UIntPtr wrote;
  bool ok = WriteProcessMemory(proc, remote, local, (UIntPtr)size, out wrote);
  System.Runtime.InteropServices.Marshal.FreeHGlobal(local);
  if (ok) SendMessage(hwnd, 0x204E, IntPtr.Zero, remote);
  VirtualFreeEx(proc, remote, UIntPtr.Zero, 0x8000);
  CloseHandle(proc);
  return ok;
}
"@
    Add-Type -MemberDefinition $sig -Name VcuTabSelect -Namespace Vcu | Out-Null
  }
  $iid = [Guid]"618736e0-3c3d-11cf-810c-00aa00389b71"
  $acc = $null
  $hr = [Vcu.VcuTabSelect]::AccessibleObjectFromWindow($h, [uint32]4294967292, [ref]$iid, [ref]$acc)
  if ($hr -ne 0 -or $null -eq $acc) { return }
  $count = [int]$acc.GetType().InvokeMember("accChildCount", [Reflection.BindingFlags]::GetProperty, $null, $acc, $null)
  $child = 0
  $tabs = 0
  for ($i = 1; $i -le $count; $i++) {
    $role = [int]$acc.GetType().InvokeMember("accRole", [Reflection.BindingFlags]::GetProperty, $null, $acc, @($i))
    if ($role -ne 37) { continue }
    $tabs++
    $name = [string]$acc.GetType().InvokeMember("accName", [Reflection.BindingFlags]::GetProperty, $null, $acc, @($i))
    if ($name -eq $expect) { $child = $i; break }
  }
  if ($tabs -eq 0) { return }
  if ($child -eq 0) { 'error:tab-name'; exit 0 }
  [void]$acc.GetType().InvokeMember("accSelect", [Reflection.BindingFlags]::InvokeMethod, $null, $acc, @(2, $child))
  $cur = [int][Vcu.VcuPaste140]::SendMessage($h, 0x130B, [IntPtr]::Zero, [IntPtr]::Zero)
  if ($cur -ne ($child - 1)) {
    [void][Vcu.VcuPaste140]::SendMessage($h, 0x130C, [IntPtr]($child - 1), [IntPtr]::Zero)
    $cur = [int][Vcu.VcuPaste140]::SendMessage($h, 0x130B, [IntPtr]::Zero, [IntPtr]::Zero)
  }
  if ($cur -ne ($child - 1)) { 'error:tab-index'; exit 0 }
  [void][Vcu.VcuTabSelect]::Notify($ownerPid, $h)
  "ok:tab_select"
  exit 0
}
function Set-VcuElement($el, [string]$expect) {
  try {
    $vp = $el.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
    $vp.SetValue($expect)
    if (([string]$vp.Current.Value) -eq $expect) {
      'ok:uia_set_value'
      exit 0
    }
  } catch {}
  $nh = [int64]$el.Current.NativeWindowHandle
  if ($nh -eq 0) { return }
  $cls = Get-VcuClass ([IntPtr]$nh)
  $uiaCls = [string]$el.Current.ClassName
  if (Test-VcuComboClass $cls) {
    Select-VcuCombo ([IntPtr]$nh) $expect
    return
  }
  if (Test-VcuListClass $cls) {
    Select-VcuList ([IntPtr]$nh) $expect
    return
  }
  if (Test-VcuTrackClass $cls) {
    Select-VcuTrack ([IntPtr]$nh) $expect
    return
  }
  Select-VcuTab ([IntPtr]$nh) $expect $targetPid
  if ((Test-VcuTabClass $cls) -or (Test-VcuTabClass $uiaCls)) { return }
  if (Test-VcuConsoleClass $cls) { return }
  if (-not (Test-VcuEditClass $cls)) { return }
  [void][Vcu.VcuSetValue070]::SendMessage([IntPtr]$nh, 12, [IntPtr]::Zero, $expect)
  $tb = New-Object System.Text.StringBuilder 1024
  # GetWindowText across processes returns only captions, so an edit looks empty.
  [void][Vcu.VcuSetValue070]::SendMessageGetText([IntPtr]$nh, 13, 1024, $tb)
  if ($tb.ToString() -eq $expect) {
    'ok:wm_settext'
    exit 0
  }
}
$targetPid = @@PID@@
$want = @@WANT@@
$val = '@@VAL@@'
$allowPaste = @@ALLOW@@
$hwnd = Wait-VcuHwnd $targetPid
if ($hwnd -eq [IntPtr]::Zero) { 'not-found'; exit 0 }
$win = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$hwnd)
if ($null -eq $win) { 'not-found'; exit 0 }
$q = New-Object System.Collections.Queue
$q.Enqueue($win)
$i = 0
$target = $null
while ($q.Count -gt 0) {
  $el = $q.Dequeue()
  $i++
  if ($i -eq $want) { $target = $el; break }
  $kids = $el.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
  foreach ($k in $kids) { $q.Enqueue($k) }
}
if ($null -eq $target) { 'not-found'; exit 0 }
Set-VcuElement $target $val
$dq = New-Object System.Collections.Queue
$dq.Enqueue($target)
$seen = 0
while ($dq.Count -gt 0 -and $seen -lt 40) {
  $el = $dq.Dequeue()
  $seen++
  if ($seen -gt 1) { Set-VcuElement $el $val }
  $kids = $el.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
  foreach ($k in $kids) { $dq.Enqueue($k) }
}
$mainCls = Get-VcuClass $hwnd
$elHwnd = [IntPtr]::Zero
$enh = [int64]$target.Current.NativeWindowHandle
if ($enh -ne 0) { $elHwnd = [IntPtr]$enh }
$elCls = Get-VcuClass $elHwnd
if ($allowPaste -eq 1 -and ((Test-VcuConsoleClass $mainCls) -or (Test-VcuConsoleClass $elCls))) {
  $pasteHwnd = $hwnd
  if (Test-VcuConsoleClass $elCls) { $pasteHwnd = $elHwnd }
  Set-Clipboard -Value $val
  Start-Sleep -Milliseconds 80
  [void][Vcu.VcuPaste140]::SendMessage($pasteHwnd, 0x0302, [IntPtr]::Zero, [IntPtr]::Zero)
  'ok:clipboard_paste'
  exit 0
}
'error:value-not-set'
"#;
    let body = body
        .replace("@@PID@@", &pid.to_string())
        .replace("@@WANT@@", &n.to_string())
        .replace("@@VAL@@", &val)
        .replace("@@ALLOW@@", allow);
    let mut s = hwnd_resolve_ps().to_string();
    s.push_str(&body);
    s
}


/// PowerShell: PrintWindow of the process main HWND to PNG (base64). If that bitmap is blank, copy only the window rectangle. Not a full-desktop capture, not SendInput.
pub fn uia_capture_script(pid: i32) -> String {
    let mut s = hwnd_resolve_ps().to_string();
    s.push_str(&format!(
        r#"
Add-Type -AssemblyName System.Drawing | Out-Null
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class VcuPrintWindow {{
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
  [StructLayout(LayoutKind.Sequential)] public struct RECT {{ public int Left; public int Top; public int Right; public int Bottom; }}
}}
"@
$hwnd = Wait-VcuHwnd {pid}
if ($hwnd -eq [IntPtr]::Zero) {{ 'MISSING'; exit 0 }}
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
# PrintWindow can return an empty black bitmap for some WinForms windows.
# Only then copy this window rectangle, not the whole desktop. Occluders can appear.
$blank = $true
foreach ($pt in @(@([int]($w/2), [int]($h/2)), @(2, 2), @([Math]::Max(0, $w-3), [Math]::Max(0, $h-3)))) {{
  $px = $bmp.GetPixel($pt[0], $pt[1])
  if ($px.R -gt 40 -or $px.G -gt 40 -or $px.B -gt 40) {{ $blank = $false; break }}
}}
if ($blank) {{
  try {{
    $g2 = [System.Drawing.Graphics]::FromImage($bmp)
    $g2.CopyFromScreen($rect.Left, $rect.Top, 0, 0, (New-Object System.Drawing.Size $w, $h))
    $g2.Dispose()
  }} catch {{}}
}}
$ms = New-Object System.IO.MemoryStream
$bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
$b64 = [Convert]::ToBase64String($ms.ToArray())
$ms.Dispose()
'FRAME|{{0}},{{1}},{{2}},{{3}}' -f $rect.Left, $rect.Top, $w, $h
$b64
"#,
        pid = pid
    ));
    s
}

pub fn parse_uia_capture_output(raw: &str) -> Option<(Vec<u8>, [f64; 4])> {
    let mut lines = raw.lines().map(|l| l.trim()).filter(|l| !l.is_empty());
    let header = lines.next()?;
    let rest = header.strip_prefix("FRAME|")?;
    let nums: Vec<f64> = rest
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    if nums.len() != 4 {
        return None;
    }
    let frame = [nums[0], nums[1], nums[2], nums[3]];
    let b64 = lines.next()?;
    let png = base64_decode(b64)?;
    if super::png_ihdr_size(&png).is_none() {
        return None;
    }
    Some((png, frame))
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    if bytes.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let a = val(chunk[0])?;
        let b = val(chunk[1])?;
        let c = if chunk[2] == b'=' { 0 } else { val(chunk[2])? };
        let d = if chunk[3] == b'=' { 0 } else { val(chunk[3])? };
        out.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' {
            out.push((b << 4) | (c >> 2));
        }
        if chunk[3] != b'=' {
            out.push((c << 6) | d);
        }
    }
    Some(out)
}

pub fn parse_uia_element_lines(raw: &str) -> Vec<AppElement> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line == "MISSING" {
            continue;
        }
        let mut sp = line.split('|');
        let eref = sp.next().unwrap_or("").trim();
        let mut role = sp.next().unwrap_or("").trim().to_string();
        let name = sp.next().unwrap_or("").trim().to_string();
        let fr = sp.next().unwrap_or("").trim();
        let class = sp.next().unwrap_or("").trim();
        let value = sp.next().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        if eref.is_empty() {
            continue;
        }
        if !class.is_empty() {
            let cl = class.to_ascii_lowercase();
            if cl == "edit"
                || cl == "document"
                || cl.contains("richedit")
                || cl.contains("windowsforms10.edit")
            {
                role = format!("{role}/{class}");
            }
        }
        let frame = {
            let p: Vec<&str> = fr.split(',').collect();
            if p.len() == 4 {
                let nums: Option<Vec<f64>> = p.iter().map(|x| x.trim().parse().ok()).collect();
                nums.and_then(|n| {
                    if n.iter().all(|v| *v == 0.0) {
                        None
                    } else {
                        Some([n[0], n[1], n[2], n[3]])
                    }
                })
            } else {
                None
            }
        };
        out.push(AppElement {
            r#ref: eref.to_string(),
            role,
            name,
            value,
            frame,
        });
    }
    out
}

impl Default for WindowsAppBackend {
    fn default() -> Self {
        Self::new()
    }
}

pub fn snapshot_from_uia(
    id: &str,
    name: &str,
    pid: Option<i32>,
    raw: &str,
    budget: u64,
) -> AppSnapshot {
    let mut elements = parse_uia_element_lines(raw);
    if budget > 0 && budget < 10 {
        elements.clear();
    }
    let title = elements
        .first()
        .map(|e| e.name.clone())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| name.to_string());
    AppSnapshot {
        target: AppTarget {
            id: id.to_string(),
            title: title.clone(),
            bundle_or_exe: name.to_string(),
            pid,
            allowed: true,
            browser_profile: None,
        },
        summary: if elements.is_empty() {
            format!(
                "process=\"{name}\" elements=0 note=uia_tree raw={}",
                raw.chars().take(160).collect::<String>()
            )
        } else {
            format!(
                "process=\"{name}\" elements={} note=uia_tree",
                elements.len()
            )
        },
        elements,
        truncated: true,
        window_frame: None,
        webview: false,
        webview_ref: None,
        page_title: None,
        page_url: None,
        tabs: vec![],
        ax_enhanced: false,
    }
}

pub fn invoke_from_uia_output(
    name: &str,
    element_ref: &str,
    out: &str,
) -> VcuResult<serde_json::Value> {
    let path = if out.contains("ok:uia_invoke") {
        "uia_invoke"
    } else if out.contains("ok:legacy_invoke") {
        "legacy_invoke"
    } else if out.contains("ok:selection_item") {
        "selection_item"
    } else if out.contains("ok:expand_collapse") {
        "expand_collapse"
    } else if out.contains("ok:toggle") {
        "toggle"
    } else if out.contains("ok:bm_click") {
        "bm_click"
    } else {
        return Err(VcuError::with_detail(
            ErrorCode::ActionFailed,
            "uia invoke failed",
            out,
        ));
    };
    Ok(serde_json::json!({
        "ok": true,
        "process": name,
        "ref": element_ref,
        "result": out,
        "input_path": path,
        "os_cursor_used": false,
        "hid_injected": false
    }))
}

pub fn set_value_from_uia_output(
    name: &str,
    element_ref: &str,
    out: &str,
) -> VcuResult<serde_json::Value> {
    if out.contains("ok:uia_set_value")
        || out.contains("ok:wm_settext")
        || out.contains("ok:clipboard_paste")
        || out.contains("ok:combo_select")
        || out.contains("ok:list_select")
        || out.contains("ok:track_select")
        || out.contains("ok:tab_select")
    {
        let path = if out.contains("ok:uia_set_value") {
            "uia_set_value"
        } else if out.contains("ok:combo_select") {
            "combo_select"
        } else if out.contains("ok:list_select") {
            "list_select"
        } else if out.contains("ok:track_select") {
            "track_select"
        } else if out.contains("ok:tab_select") {
            "tab_select"
        } else if out.contains("ok:wm_settext") {
            "wm_settext"
        } else {
            "clipboard_paste"
        };
        Ok(serde_json::json!({
            "ok": true,
            "process": name,
            "ref": element_ref,
            "result": out,
            "input_path": path,
            "os_cursor_used": false,
            "hid_injected": false
        }))
    } else {
        Err(VcuError::with_detail(
            ErrorCode::ActionFailed,
            "uia set_value failed",
            out,
        ))
    }
}

/// PowerShell: open a directory in Explorer. No SendInput.
pub fn uia_scroll_script(pid: i32, eref: &str, dy: i32) -> String {
    let n = eref.trim_start_matches('e').parse::<i32>().unwrap_or(0);
    let mut s = hwnd_resolve_ps().to_string();
    let body = r#"
Add-Type -AssemblyName UIAutomationClient | Out-Null
if (-not ("Vcu.VcuScroll180" -as [type])) {
  $sig = @'
[DllImport("user32.dll")]
public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
'@
  Add-Type -MemberDefinition $sig -Name VcuScroll180 -Namespace Vcu | Out-Null
}
$targetPid = @@PID@@
$want = @@WANT@@
$dy = @@DY@@
$hwnd = Wait-VcuHwnd $targetPid
if ($hwnd -eq [IntPtr]::Zero) { 'not-found'; exit 0 }
$win = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$hwnd)
if ($null -eq $win) { 'not-found'; exit 0 }
function Try-VcuScrollPattern($el, $delta) {
  try {
    $sp = $el.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
    $before = -1.0
    try { $before = [double]$sp.Current.VerticalScrollPercent } catch {}
    $vert = if ($delta -ge 0) { [System.Windows.Automation.ScrollAmount]::LargeIncrement } else { [System.Windows.Automation.ScrollAmount]::LargeDecrement }
    $sp.Scroll([System.Windows.Automation.ScrollAmount]::NoAmount, $vert)
    $after = $before
    try { $after = [double]$sp.Current.VerticalScrollPercent } catch {}
    if ($before -lt 0 -or $after -ne $before) {
      'ok:uia_scroll'
      exit 0
    }
  } catch {}
}
function Try-VcuListScroll($el, $delta) {
  $cls = ''
  try { $cls = [string]$el.Current.ClassName } catch {}
  if ($cls -notlike '*LISTBOX*') { return }
  $nh = [int64]$el.Current.NativeWindowHandle
  if ($nh -eq 0) { return }
  $top0 = [int][Vcu.VcuScroll180]::SendMessage([IntPtr]$nh, 0x018E, [IntPtr]::Zero, [IntPtr]::Zero)
  $sb = if ($delta -ge 0) { 3 } else { 2 }
  [void][Vcu.VcuScroll180]::SendMessage([IntPtr]$nh, 0x0115, [IntPtr]$sb, [IntPtr]::Zero)
  $top1 = [int][Vcu.VcuScroll180]::SendMessage([IntPtr]$nh, 0x018E, [IntPtr]::Zero, [IntPtr]::Zero)
  if ($top1 -ne $top0) {
    'ok:wm_vscroll'
    exit 0
  }
}
function Walk-VcuScroll($root, $delta) {
  $q = New-Object System.Collections.Queue
  $q.Enqueue($root)
  $n = 0
  while ($q.Count -gt 0 -and $n -lt 80) {
    $el = $q.Dequeue()
    $n++
    Try-VcuScrollPattern $el $delta
    Try-VcuListScroll $el $delta
    $kids = $el.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($k in $kids) { $q.Enqueue($k) }
  }
}
$root = $win
if ($want -gt 0) {
  $q = New-Object System.Collections.Queue
  $q.Enqueue($win)
  $i = 0
  while ($q.Count -gt 0) {
    $el = $q.Dequeue()
    $i++
    if ($i -eq $want) { $root = $el; break }
    $kids = $el.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($k in $kids) { $q.Enqueue($k) }
  }
  if ($i -ne $want) { 'not-found'; exit 0 }
}
Walk-VcuScroll $root $dy
$sb = if ($dy -ge 0) { 3 } else { 2 }
[void][Vcu.VcuScroll180]::SendMessage([IntPtr]$hwnd, 0x0115, [IntPtr]$sb, [IntPtr]::Zero)
'ok:wm_vscroll'
"#;
    let body = body
        .replace("@@PID@@", &pid.to_string())
        .replace("@@WANT@@", &n.to_string())
        .replace("@@DY@@", &dy.to_string());
    s.push_str(&body);
    s
}


pub fn scroll_from_uia_output(name: &str, out: &str) -> VcuResult<serde_json::Value> {
    let path = if out.contains("ok:uia_scroll") {
        "uia_scroll"
    } else if out.contains("ok:wm_vscroll") {
        "wm_vscroll"
    } else {
        return Err(VcuError::with_detail(ErrorCode::ActionFailed, "uia scroll failed", out));
    };
    Ok(serde_json::json!({
        "ok": true,
        "process": name,
        "result": out,
        "input_path": path,
        "os_cursor_used": false,
        "hid_injected": false
    }))
}

pub fn explorer_open_script(path: &str) -> String {
    let p = path.replace('\'', "''");
    format!(
        r#"
$path = '{p}'
if (-not (Test-Path -LiteralPath $path -PathType Container)) {{ 'error:not-dir'; exit 1 }}
Start-Process -FilePath explorer.exe -ArgumentList $path | Out-Null
'ok:explorer_open'
"#,
        p = p
    )
}

/// PowerShell: select a file in Explorer. No SendInput.
pub fn explorer_reveal_script(path: &str) -> String {
    let p = path.replace('\'', "''");
    format!(
        r#"
$path = '{p}'
if (-not (Test-Path -LiteralPath $path)) {{ 'error:missing'; exit 1 }}
Start-Process -FilePath explorer.exe -ArgumentList ('/select,' + $path) | Out-Null
'ok:explorer_reveal'
"#,
        p = p
    )
}

#[async_trait]
impl AppBackend for WindowsAppBackend {
    fn platform(&self) -> &str {
        "windows"
    }

    async fn list_windows(&self) -> VcuResult<Vec<AppTarget>> {
        #[cfg(not(windows))]
        {
            Err(VcuError::coded(
                ErrorCode::NotImplemented,
                "WindowsAppBackend::list_windows only runs on Windows hosts",
            ))
        }
        #[cfg(windows)]
        {
            let script = r#"
Get-Process | Where-Object {
  $_.MainWindowTitle -ne '' -or
  @('cmd','conhost','powershell','pwsh','WindowsTerminal','Calculator','calc','CalculatorApp','SystemSettings') -contains $_.ProcessName
} |
  ForEach-Object { '{0}|{1}|{2}' -f $_.ProcessName, $_.Id, ($_.MainWindowTitle -replace '[\r\n\t]',' ') }
# Windowless Notepad stubs alias onto the list when one nearby visible Notepad exists.
$visibleNp = @(Get-Process -Name Notepad -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 })
Get-Process -Name Notepad -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -eq 0 } | ForEach-Object {
  $stub = $_
  $near = @()
  foreach ($w in $visibleNp) {
    try {
      $delta = [Math]::Abs(($w.StartTime - $stub.StartTime).TotalSeconds)
      if ($delta -lt 8) { $near += $w }
    } catch {}
  }
  $pick = $null
  if ($near.Count -eq 1) { $pick = $near[0] }
  elseif ($visibleNp.Count -eq 1) { $pick = $visibleNp[0] }
  if ($null -ne $pick) {
    $title = [string]$pick.MainWindowTitle
    '{0}|{1}|{2}' -f $stub.ProcessName, $stub.Id, ($title -replace '[\r\n\t]',' ')
  }
}
"#;
            let raw = Self::run_powershell(script)?;
            Ok(parse_process_list_lines(&raw, |n| self.allowed(n)))
        }
    }

    async fn focus_window(&mut self, _id: &str, allow_focus_steal: bool) -> VcuResult<()> {
        if !allow_focus_steal {
            return Err(VcuError::coded(
                ErrorCode::FocusPolicyViolation,
                "refusing to steal app focus on Windows",
            ));
        }
        Err(VcuError::coded(
            ErrorCode::NotImplemented,
            "explicit Windows focus is gated",
        ))
    }

    async fn snapshot(&self, id: &str, budget: u64) -> VcuResult<AppSnapshot> {
        #[cfg(not(windows))]
        {
            let _ = budget;
            Err(VcuError::coded(
                ErrorCode::NotImplemented,
                format!("Windows snapshot unavailable on this host for {id}"),
            ))
        }
        #[cfg(windows)]
        {
            let name = id
                .strip_prefix("win:")
                .and_then(|rest| rest.split(':').next())
                .map(|s| s.replace('_', " "))
                .unwrap_or_else(|| id.to_string());
            if !self.allowed(&name) {
                return Err(VcuError::coded(
                    ErrorCode::FocusPolicyViolation,
                    format!("process '{name}' not in app allowlist"),
                ));
            }
            let pid = pid_from_win_id(id).ok_or_else(|| {
                VcuError::coded(
                    ErrorCode::InvalidInput,
                    "Windows snapshot requires win:name:pid",
                )
            })?;
            let raw = Self::run_powershell(&uia_tree_script(pid, 80))?;
            Ok(snapshot_from_uia(id, &name, Some(pid), &raw, budget))
        }
    }

    async fn invoke(&mut self, id: &str, element_ref: &str) -> VcuResult<serde_json::Value> {
        let name = id
            .strip_prefix("win:")
            .and_then(|rest| rest.split(':').next())
            .map(|s| s.replace('_', " "))
            .unwrap_or_else(|| id.to_string());
        if is_denied_app(&name) {
            return Err(VcuError::coded(
                ErrorCode::AppDenied,
                format!("app '{name}' is denied by VCU policy"),
            ));
        }
        #[cfg(not(windows))]
        {
            let _ = (id, element_ref);
            Err(VcuError::coded(
                ErrorCode::OsCursorDenied,
                "Windows app invoke is denied by default safety policy (no SendInput / no cursor)",
            ))
        }
        #[cfg(windows)]
        {
            if !self.allowed(&name) {
                return Err(VcuError::coded(
                    ErrorCode::FocusPolicyViolation,
                    format!("process '{name}' not in app allowlist"),
                ));
            }
            let pid = pid_from_win_id(id).ok_or_else(|| {
                VcuError::coded(
                    ErrorCode::InvalidInput,
                    "Windows invoke requires win:name:pid",
                )
            })?;
            let out = Self::run_powershell(&uia_invoke_script(pid, element_ref))?;
            invoke_from_uia_output(&name, element_ref, &out)
        }
    }

    async fn set_value(
        &mut self,
        id: &str,
        element_ref: &str,
        value: &str,
    ) -> VcuResult<serde_json::Value> {
        let name = id
            .strip_prefix("win:")
            .and_then(|rest| rest.split(':').next())
            .map(|s| s.replace('_', " "))
            .unwrap_or_else(|| id.to_string());
        if is_denied_app(&name) {
            return Err(VcuError::coded(
                ErrorCode::AppDenied,
                format!("app '{name}' is denied by VCU policy"),
            ));
        }
        if windows_console_app(&name) && value.chars().any(|c| c == '\n' || c == '\r') {
            return Err(VcuError::coded(
                ErrorCode::FocusPolicyViolation,
                "Windows console type refuses newline/Return so commands are not executed",
            ));
        }
        #[cfg(not(windows))]
        {
            let _ = (id, element_ref, value);
            Err(VcuError::coded(
                ErrorCode::NotImplemented,
                "Windows set_value only runs on Windows hosts",
            ))
        }
        #[cfg(windows)]
        {
            if !self.allowed(&name) {
                return Err(VcuError::coded(
                    ErrorCode::FocusPolicyViolation,
                    format!("process '{name}' not in app allowlist"),
                ));
            }
            let pid = pid_from_win_id(id).ok_or_else(|| {
                VcuError::coded(
                    ErrorCode::InvalidInput,
                    "Windows set_value requires win:name:pid",
                )
            })?;
            let out = Self::run_powershell(&uia_set_value_script_for(
                pid,
                element_ref,
                value,
                windows_console_app(&name),
            ))?;
            set_value_from_uia_output(&name, element_ref, &out)
        }
    }

    async fn open_path(&mut self, id: &str, path: &str) -> VcuResult<serde_json::Value> {
        let name = id
            .strip_prefix("win:")
            .and_then(|rest| rest.split(':').next())
            .map(|s| s.replace('_', " "))
            .unwrap_or_else(|| id.to_string());
        if is_denied_app(&name) {
            return Err(VcuError::coded(
                ErrorCode::AppDenied,
                format!("app '{name}' is denied by VCU policy"),
            ));
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Err(VcuError::coded(
                ErrorCode::NotImplemented,
                "Windows open_path only runs on Windows hosts",
            ))
        }
        #[cfg(windows)]
        {
            if path
                .chars()
                .any(|c| matches!(c, '"' | '`' | '\n' | '\r' | '{' | '}'))
            {
                return Err(VcuError::coded(
                    ErrorCode::InvalidInput,
                    "Windows open_path path has forbidden characters",
                ));
            }
            let out = Self::run_powershell(&explorer_open_script(path))?;
            if !out.contains("ok:explorer_open") {
                return Err(VcuError::with_detail(
                    ErrorCode::ActionFailed,
                    "explorer open_path failed",
                    out,
                ));
            }
            Ok(serde_json::json!({
                "ok": true,
                "process": name,
                "path": path,
                "result": out,
                "input_path": "explorer_open",
                "os_cursor_used": false,
                "hid_injected": false
            }))
        }
    }

    async fn reveal_path(&mut self, id: &str, path: &str) -> VcuResult<serde_json::Value> {
        let name = id
            .strip_prefix("win:")
            .and_then(|rest| rest.split(':').next())
            .map(|s| s.replace('_', " "))
            .unwrap_or_else(|| id.to_string());
        if is_denied_app(&name) {
            return Err(VcuError::coded(
                ErrorCode::AppDenied,
                format!("app '{name}' is denied by VCU policy"),
            ));
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Err(VcuError::coded(
                ErrorCode::NotImplemented,
                "Windows reveal_path only runs on Windows hosts",
            ))
        }
        #[cfg(windows)]
        {
            if path
                .chars()
                .any(|c| matches!(c, '"' | '`' | '\n' | '\r' | '{' | '}'))
            {
                return Err(VcuError::coded(
                    ErrorCode::InvalidInput,
                    "Windows reveal_path path has forbidden characters",
                ));
            }
            let out = Self::run_powershell(&explorer_reveal_script(path))?;
            if !out.contains("ok:explorer_reveal") {
                return Err(VcuError::with_detail(
                    ErrorCode::ActionFailed,
                    "explorer reveal failed",
                    out,
                ));
            }
            Ok(serde_json::json!({
                "ok": true,
                "process": name,
                "path": path,
                "result": out,
                "input_path": "explorer_reveal",
                "os_cursor_used": false,
                "hid_injected": false
            }))
        }
    }

    async fn scroll(
        &mut self,
        id: &str,
        element_ref: Option<&str>,
        dy: i32,
    ) -> VcuResult<serde_json::Value> {
        let name = id
            .strip_prefix("win:")
            .and_then(|rest| rest.split(':').next())
            .map(|s| s.replace('_', " "))
            .unwrap_or_else(|| id.to_string());
        if is_denied_app(&name) {
            return Err(VcuError::coded(
                ErrorCode::AppDenied,
                format!("app '{name}' is denied by VCU policy"),
            ));
        }
        #[cfg(not(windows))]
        {
            let _ = (element_ref, dy);
            Err(VcuError::coded(
                ErrorCode::NotImplemented,
                "Windows scroll only runs on Windows hosts",
            ))
        }
        #[cfg(windows)]
        {
            if !self.allowed(&name) {
                return Err(VcuError::coded(
                    ErrorCode::FocusPolicyViolation,
                    format!("process '{name}' not in app allowlist"),
                ));
            }
            let pid = pid_from_win_id(id).ok_or_else(|| {
                VcuError::coded(ErrorCode::InvalidInput, "Windows scroll requires win:name:pid")
            })?;
            let eref = element_ref.unwrap_or("");
            let out = Self::run_powershell(&uia_scroll_script(pid, eref, dy))?;
            scroll_from_uia_output(&name, &out)
        }
    }

    async fn capture_window(&self, id: &str) -> VcuResult<Option<super::AppCapture>> {
        #[cfg(not(windows))]
        {
            let _ = id;
            Ok(None)
        }
        #[cfg(windows)]
        {
            let name = id
                .strip_prefix("win:")
                .and_then(|rest| rest.split(':').next())
                .map(|s| s.replace('_', " "))
                .unwrap_or_else(|| id.to_string());
            if is_denied_app(&name) {
                return Err(VcuError::coded(
                    ErrorCode::AppDenied,
                    format!("app '{name}' is denied by VCU policy"),
                ));
            }
            if !self.allowed(&name) {
                return Err(VcuError::coded(
                    ErrorCode::FocusPolicyViolation,
                    format!("process '{name}' not in app allowlist"),
                ));
            }
            let Some(pid) = pid_from_win_id(id) else {
                return Err(VcuError::coded(
                    ErrorCode::InvalidInput,
                    "Windows screenshot requires win:name:pid",
                ));
            };
            let raw = Self::run_powershell(&uia_capture_script(pid))?;
            let Some((png, frame)) = parse_uia_capture_output(&raw) else {
                let why = if raw.contains("MISSING") {
                    "no visible HWND"
                } else {
                    "PrintWindow output was not a PNG"
                };
                return Err(VcuError::coded(
                    ErrorCode::ActionFailed,
                    format!(
                        "Windows PrintWindow did not produce a PNG ({why}). This is not a macOS screen-recording permission."
                    ),
                ));
            };
            let Some((width, height)) = super::png_ihdr_size(&png) else {
                return Ok(None);
            };
            Ok(Some(super::AppCapture {
                png,
                width,
                height,
                frame,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn windows_backend_platform_and_denials() {
        let mut b = WindowsAppBackend::new();
        assert_eq!(b.platform(), "windows");
        let err = b.focus_window("win:x:1", false).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::FocusPolicyViolation);
        let err = b.invoke("win:x:1", "e1").await.unwrap_err();
        #[cfg(not(windows))]
        {
            assert_eq!(err.code(), ErrorCode::OsCursorDenied);
            assert!(
                err.message().to_ascii_lowercase().contains("sendinput")
                    || err.message().to_ascii_lowercase().contains("cursor")
            );
        }
        #[cfg(windows)]
        {
            assert_eq!(err.code(), ErrorCode::FocusPolicyViolation);
        }
        let denied = b.invoke("win:WeChat:2", "e1").await.unwrap_err();
        assert_eq!(denied.code(), ErrorCode::AppDenied);
        let denied = b.open_path("win:WeChat:2", "C:\\Temp").await.unwrap_err();
        assert_eq!(denied.code(), ErrorCode::AppDenied);
        let err = b
            .open_path("win:notepad:1", "C:\\vcu-no-such-dir")
            .await
            .unwrap_err();
        #[cfg(not(windows))]
        assert_eq!(err.code(), ErrorCode::NotImplemented);
        #[cfg(windows)]
        assert_eq!(err.code(), ErrorCode::ActionFailed);
        let open_script = explorer_open_script(r"C:\Temp");
        assert!(open_script.contains("explorer.exe"));
        assert!(open_script.contains("ok:explorer_open"));
        assert!(!open_script.to_ascii_lowercase().contains("sendinput("));
        let denied = b
            .reveal_path("win:WeChat:2", "C:\\Temp\\x.txt")
            .await
            .unwrap_err();
        assert_eq!(denied.code(), ErrorCode::AppDenied);
        let err = b
            .reveal_path("win:notepad:1", "C:\\vcu-no-such-file.txt")
            .await
            .unwrap_err();
        #[cfg(not(windows))]
        assert_eq!(err.code(), ErrorCode::NotImplemented);
        #[cfg(windows)]
        assert_eq!(err.code(), ErrorCode::ActionFailed);
        let rev = explorer_reveal_script(r"C:\Temp\x.txt");
        assert!(rev.contains("/select,"));
        assert!(rev.contains("ok:explorer_reveal"));
        assert!(!rev.to_ascii_lowercase().contains("sendinput("));
        let err = b.set_value("win:notepad:1", "e1", "hi").await.unwrap_err();
        #[cfg(not(windows))]
        assert_eq!(err.code(), ErrorCode::NotImplemented);
        let err = b.scroll("win:notepad:1", None, 600).await.unwrap_err();
        #[cfg(not(windows))]
        assert_eq!(err.code(), ErrorCode::NotImplemented);
        let denied = b.scroll("win:WeChat:2", None, 600).await.unwrap_err();
        assert_eq!(denied.code(), ErrorCode::AppDenied);
        #[cfg(windows)]
        assert_eq!(err.code(), ErrorCode::ActionFailed);
        let nl = b
            .set_value("win:cmd:1", "e1", "echo\nhi")
            .await
            .unwrap_err();
        assert_eq!(nl.code(), ErrorCode::FocusPolicyViolation);
        let ps_nl = b
            .set_value("win:powershell:9", "e2", "line\n2")
            .await
            .unwrap_err();
        assert_eq!(ps_nl.code(), ErrorCode::FocusPolicyViolation);
        let setv = uia_set_value_script(4242, "e2", "hello");
        assert!(setv.contains("ValuePattern"));
        assert!(setv.contains("SetValue"));
        assert!(setv.contains("hello"));
        assert!(setv.contains("ok:wm_settext"));
        assert!(setv.contains("ok:combo_select"));
        assert!(setv.contains("0x014E"));
        assert!(setv.contains("ok:list_select"));
        assert!(setv.contains("0x0186"));
        assert!(setv.contains("ok:track_select"));
        assert!(setv.contains("0x0405"));
        assert!(setv.contains("ok:tab_select"));
        assert!(setv.contains("Test-VcuTabClass"));
        assert!(setv.contains("Test-VcuTrackClass"));
        let tree = uia_tree_script(1, 8);
        assert!(tree.contains("TRACKBAR"));
        assert!(tree.contains("track="));
        assert!(!setv.to_ascii_lowercase().contains("sendinput("));
        #[cfg(not(windows))]
        {
            let cap = b.capture_window("win:notepad:1").await.unwrap();
            assert!(cap.is_none());
        }
        #[cfg(windows)]
        {
            let err = b.capture_window("win:notepad:1").await.unwrap_err();
            let msg = err.message();
            assert!(!msg.contains("VCU_ALLOW_SCREENCAPTURE"), "{msg}");
            assert!(!msg.to_ascii_lowercase().contains("screen recording"), "{msg}");
        }
    }

    #[test]
    fn decode_powershell_output_keeps_utf8_chinese() {
        let raw = "文件".as_bytes();
        assert_eq!(decode_powershell_output(raw), "文件");
        let mut bom = vec![0xEF, 0xBB, 0xBF];
        bom.extend_from_slice(raw);
        assert_eq!(decode_powershell_output(&bom), "文件");
        assert!(powershell_utf8_prelude().contains("UTF8Encoding"));
    }

    #[test]
    fn parse_process_list_keeps_windowsterminal() {
        let b = WindowsAppBackend::new();
        let wins = parse_process_list_lines("WindowsTerminal\t9\tVCU", |n| b.allowed(n));
        assert_eq!(wins.len(), 1);
        assert_eq!(wins[0].bundle_or_exe, "WindowsTerminal");
        assert!(wins[0].id.starts_with("win:WindowsTerminal:"));
    }

    #[test]
    fn frame_host_calculator_aliases_without_allowing_the_host() {
        let raw = "ApplicationFrameHost\t18668\t计算器\nCalculatorApp\t6448\tCalculatorApp\nApplicationFrameHost\t99\t随机窗口\n";
        let wins = parse_process_list_lines(raw, |n| {
            let l = n.to_lowercase();
            l.contains("calculator") || l.contains("notepad")
        });
        assert_eq!(wins.len(), 1, "{wins:?}");
        assert_eq!(wins[0].id, "win:Calculator:18668");
        assert_eq!(wins[0].bundle_or_exe, "Calculator");
        assert!(wins[0].title.contains("计算器"));
        assert!(frame_host_alias("Calculator") == Some("Calculator"));
        assert!(frame_host_alias("设置").is_none());
    }

    #[test]
    fn parse_process_list_keeps_allowlist_drops_wechat() {
        let b = WindowsAppBackend::new();
        let raw = "notepad\t1001\tUntitled - Notepad\nWeChat\t2002\tWeChat\nexplorer\t3003\tDocuments\nmsedge\t4004\tMicrosoft Edge\ncmd\t5005\t\nconhost\t5006\tVCU-D-140\nCalculator\t5007\tCalculator\nSystemSettings\t5008\tSettings\n";
        let wins = parse_process_list_lines(raw, |n| b.allowed(n));
        let names: Vec<_> = wins.iter().map(|w| w.bundle_or_exe.as_str()).collect();
        assert!(names.contains(&"notepad"));
        assert!(names.contains(&"explorer"));
        assert!(names.contains(&"msedge"));
        assert!(names.contains(&"cmd"));
        assert!(names.contains(&"conhost"));
        assert!(names.contains(&"Calculator"));
        assert!(!names.iter().any(|n| n.to_lowercase().contains("wechat")));
        assert!(wins.iter().any(|w| w.id.starts_with("win:notepad:")));
        assert!(wins.iter().any(|w| w.id == "win:cmd:5005"));
        assert!(wins.iter().any(|w| w.id == "win:conhost:5006"));
        assert!(wins.iter().any(|w| w.id == "win:Calculator:5007"));
        assert!(wins.iter().any(|w| w.id == "win:SystemSettings:5008"));
    }

    #[test]
    fn parse_uia_tree_and_scripts_are_pattern_not_hid() {
        let els = parse_uia_element_lines(
            "e1|ControlType.Window|Notepad|10,10,800,600\ne2|ControlType.Button|Save|20,40,80,24\nMISSING\ne3|ControlType.Pane||10,40,780,540|Edit|VCU-D-190\ne4|ControlType.Edit||1,2,3,4|WindowsForms10.EDIT.app.0.1|\n"        );
        assert_eq!(els.len(), 4);
        assert!(els[3].role.contains("WindowsForms10.EDIT"));
        assert_eq!(els[0].r#ref, "e1");
        assert_eq!(els[1].name, "Save");
        assert_eq!(els[1].frame, Some([20.0, 40.0, 80.0, 24.0]));
        assert_eq!(els[2].role, "ControlType.Pane/Edit");
        assert_eq!(els[2].value.as_deref(), Some("VCU-D-190"));
        let tree = uia_tree_script(4242, 80);
        assert!(tree.contains("UIAutomationClient"));
        assert!(tree.contains("FromHandle"));
        assert!(tree.contains("ClassName"));
        assert!(tree.contains("AttachConsole"));
        assert!(tree.contains("VcuHwndResolve"));
        assert!(tree.contains("AutomationId"));
        assert!(tree.contains("LegacyIAccessiblePattern"));
        assert!(tree.contains("ValuePattern"));
        assert!(tree.contains("roleId -eq 44"));
        assert!(tree.contains("toggle-on"));
        assert!(tree.contains("GetWindowText"));
        assert!(!tree.to_ascii_lowercase().contains("sendinput"));
        let inv = uia_invoke_script(4242, "e2");
        assert!(inv.contains("InvokePattern"));
        assert!(inv.contains("ok:uia_invoke"));
        assert!(inv.contains("SelectionItemPattern"));
        assert!(inv.contains("ok:selection_item"));
        assert!(inv.contains("ExpandCollapsePattern"));
        assert!(inv.contains("TogglePattern"));
        assert!(inv.contains("ok:toggle"));
        assert!(inv.contains("TreeScope]::Children"));
        assert!(inv.contains("ok:expand_collapse"));
        assert!(!inv.to_ascii_lowercase().contains("sendinput"));
        assert!(!inv.to_ascii_lowercase().contains("mouse_event"));
        assert_eq!(pid_from_win_id("win:notepad:4242"), Some(4242));
        let snap = snapshot_from_uia(
            "win:notepad:4242",
            "notepad",
            Some(4242),
            "e1|ControlType.Window|Notepad|0,0,800,600\ne2|ControlType.Edit||10,40,780,540\n",
            80,
        );
        assert_eq!(snap.elements.len(), 2);
        assert!(snap.summary.contains("note=uia_tree"));
        assert_eq!(snap.target.pid, Some(4242));
        let ok = invoke_from_uia_output("notepad", "e2", "ok:uia_invoke").unwrap();
        assert_eq!(ok["input_path"], "uia_invoke");
        assert_eq!(ok["os_cursor_used"], false);
        let sel = invoke_from_uia_output("form", "e2", "ok:selection_item").unwrap();
        assert_eq!(sel["input_path"], "selection_item");
        let exp = invoke_from_uia_output("form", "e2", "ok:expand_collapse").unwrap();
        assert_eq!(exp["input_path"], "expand_collapse");
        assert_eq!(exp["os_cursor_used"], false);
        assert_eq!(sel["os_cursor_used"], false);
        let bm = invoke_from_uia_output("form", "e2", "ok:bm_click").unwrap();
        assert_eq!(bm["input_path"], "bm_click");
        assert_eq!(bm["os_cursor_used"], false);
        assert!(invoke_from_uia_output("notepad", "e2", "not-found").is_err());
        assert!(inv.contains("ok:bm_click") || inv.contains("0x00F5"));
        let typed = set_value_from_uia_output("notepad", "e2", "ok:uia_set_value").unwrap();
        assert_eq!(typed["input_path"], "uia_set_value");
        let paste = set_value_from_uia_output("cmd", "e1", "ok:clipboard_paste").unwrap();
        assert_eq!(paste["input_path"], "clipboard_paste");
        assert_eq!(paste["os_cursor_used"], false);
        let wm = set_value_from_uia_output("notepad", "e2", "ok:wm_settext").unwrap();
        assert_eq!(wm["input_path"], "wm_settext");
        assert_eq!(wm["os_cursor_used"], false);
        let setv = uia_set_value_script(4242, "e2", "hello");
        assert!(setv.contains("ok:wm_settext"));
        assert!(setv.contains("ok:clipboard_paste"));
        assert!(setv.contains("Set-Clipboard"));
        assert!(setv.contains("SendMessage"));
        assert!(setv.contains("VcuHwndResolve"));
        assert!(!setv.to_ascii_lowercase().contains("sendinput("));
        let cmdv = uia_set_value_script_for(4242, "e1", "echo-not-run", true);
        assert!(cmdv.contains("ok:uia_set_value"));
        assert!(cmdv.contains("ok:wm_settext"));
        assert!(cmdv.contains("ok:clipboard_paste"));
        assert!(cmdv.contains("Set-Clipboard"));
        assert!(cmdv.contains("0x0302"));
        assert!(cmdv.contains("AttachConsole"));
        assert!(cmdv.contains("ConsoleWindowClass"));
        assert!(cmdv.contains("$allowPaste = 1"));
        let value_at = cmdv.find("ok:uia_set_value").unwrap();
        let wm_at = cmdv.find("ok:wm_settext").unwrap();
        let paste_at = cmdv.find("ok:clipboard_paste").unwrap();
        assert!(value_at < paste_at && wm_at < paste_at);
        assert!(cmdv.find("Test-VcuConsoleClass").unwrap() < paste_at);
        assert!(!cmdv.to_ascii_lowercase().contains("sendinput("));
        let gui = uia_set_value_script_for(14124, "e2", "vcu-own-009", true);
        assert!(gui.contains("$allowPaste = 1"));
        assert!(gui.contains("windowsforms10.edit"));
        assert!(gui.contains("SendMessageGetText"));
        assert!(gui.contains("error:value-not-set"));
        assert!(gui.find("ok:uia_set_value").unwrap() < gui.find("ok:clipboard_paste").unwrap());
        let plain = uia_set_value_script(4242, "e2", "hello-plain");
        assert!(plain.contains("$allowPaste = 0"));
        assert!(plain.contains("error:value-not-set"));
        assert!(windows_console_app("powershell"));
        assert!(windows_console_app("pwsh"));
        assert!(!windows_console_app("notepad"));
        assert!(set_value_from_uia_output("notepad", "e2", "error:no-value-pattern").is_err());
        let scr = uia_scroll_script(4242, "e1", 600);
        assert!(scr.contains("ScrollPattern"));
        assert!(scr.contains("0x0115"));
        assert!(scr.contains("0x018E"));
        assert!(scr.contains("LISTBOX"));
        assert!(scr.contains("ok:uia_scroll"));
        assert!(scr.contains("ok:wm_vscroll"));
        assert!(!scr.to_ascii_lowercase().contains("sendinput("));
        assert!(!scr.to_ascii_lowercase().contains("mouse_event"));
        let ok_s = scroll_from_uia_output("notepad", "ok:uia_scroll").unwrap();
        assert_eq!(ok_s["input_path"], "uia_scroll");
        let ok_w = scroll_from_uia_output("notepad", "ok:wm_vscroll").unwrap();
        assert_eq!(ok_w["input_path"], "wm_vscroll");
        assert_eq!(ok_w["os_cursor_used"], false);
        let cap_script = uia_capture_script(4242);
        assert!(cap_script.contains("PrintWindow"));
        assert!(cap_script.contains("GetWindowRect"));
        assert!(cap_script.contains("CopyFromScreen"));
        assert!(cap_script.contains("$blank"));
        assert!(!cap_script.to_ascii_lowercase().contains("sendinput"));
        assert!(!cap_script.to_ascii_lowercase().contains("mouse_event"));
        let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let parsed = parse_uia_capture_output(&format!("FRAME|10,20,800,600\n{b64}\n"))
            .expect("parse capture");
        assert_eq!(parsed.1, [10.0, 20.0, 800.0, 600.0]);
        assert_eq!(crate::app::png_ihdr_size(&parsed.0), Some((1, 1)));
    }

    #[test]
    fn windows_live_poc_script_is_uia_not_hid() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let path = root.join("scripts/poc_desktop_windows.ps1");
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(raw.contains("UIA_OK"), "{}", path.display());
        assert!(raw.contains("PRINTWINDOW_OK"));
        assert!(raw.contains("SETVALUE_OK"));
        assert!(raw.contains("ValuePattern"));
        assert!(raw.contains("SetValue"));
        let lower = raw.to_ascii_lowercase();
        assert!(!lower.contains("sendinput("), "must not call SendInput");
        assert!(!lower.contains("[system.windows.forms.sendkeys"));
        assert!(!lower.contains("mouse_event("));
        assert!(!lower.contains("copyfromscreen("));
        let p90 = root.join("scripts/poc_cu_d_090.ps1");
        let s90 = std::fs::read_to_string(&p90).unwrap_or_default();
        assert!(s90.contains("session start"), "{}", p90.display());
        assert!(s90.contains("--surface desktop"));
        assert!(s90.contains("TYPE_OK"));
        let l90 = s90.to_ascii_lowercase();
        assert!(!l90.contains("sendinput("));
        assert!(!l90.contains("[system.windows.forms.sendkeys"));
        let p100 = root.join("scripts/poc_cu_d_100.ps1");
        let s100 = std::fs::read_to_string(&p100).unwrap_or_default();
        assert!(s100.contains("INVOKE_OK"), "{}", p100.display());
        assert!(s100.contains("VcuCount"));
        assert!(s100.contains("click"));
        let l100 = s100.to_ascii_lowercase();
        assert!(!l100.contains("sendinput("));
        assert!(!l100.contains("[system.windows.forms.sendkeys"));
        let p110 = root.join("scripts/poc_cu_d_110.ps1");
        let s110 = std::fs::read_to_string(&p110).unwrap_or_default();
        assert!(s110.contains("SHOT_OK"), "{}", p110.display());
        assert!(s110.contains("screenshot"));
        let l110 = s110.to_ascii_lowercase();
        assert!(!l110.contains("sendinput("));
        assert!(!l110.contains("copyfromscreen("));
        let p120 = root.join("scripts/poc_cu_d_120.ps1");
        let s120 = std::fs::read_to_string(&p120).unwrap_or_default();
        assert!(s120.contains("OPEN_OK"), "{}", p120.display());
        assert!(s120.contains("open_path"));
        let l120 = s120.to_ascii_lowercase();
        assert!(!l120.contains("sendinput("));
        let p130 = root.join("scripts/poc_cu_d_130.ps1");
        let s130 = std::fs::read_to_string(&p130).unwrap_or_default();
        assert!(s130.contains("REVEAL_OK"), "{}", p130.display());
        assert!(s130.contains("explorer_reveal"));
        let l130 = s130.to_ascii_lowercase();
        assert!(!l130.contains("sendinput("));
        let p140 = root.join("scripts/poc_cu_d_140.ps1");
        let s140 = std::fs::read_to_string(&p140).unwrap_or_default();
        assert!(s140.contains("TYPE_OK"), "{}", p140.display());
        assert!(s140.contains("NEWLINE_DENIED"));
        let l140 = s140.to_ascii_lowercase();
        assert!(!l140.contains("sendinput("));
        assert!(!l140.contains("[system.windows.forms.sendkeys"));
        let p160 = root.join("scripts/poc_cu_d_160.ps1");
        let s160 = std::fs::read_to_string(&p160).unwrap_or_default();
        assert!(s160.contains("INVOKE_OK"), "{}", p160.display());
        assert!(s160.contains("ProcessName") && s160.contains("win:"));
        let l160 = s160.to_ascii_lowercase();
        assert!(!l160.contains("sendinput("));
        assert!(!l160.contains("[system.windows.forms.sendkeys"));
        let p200 = root.join("scripts/poc_cu_d_200.ps1");
        let s200 = std::fs::read_to_string(&p200).unwrap_or_default();
        assert!(s200.contains("WAIT_OK"), "{}", p200.display());
        let l200 = s200.to_ascii_lowercase();
        assert!(!l200.contains("sendinput("));
        let p210 = root.join("scripts/poc_cu_d_210.ps1");
        let s210 = std::fs::read_to_string(&p210).unwrap_or_default();
        assert!(s210.contains("WAIT_MISS_OK"), "{}", p210.display());
        assert!(s210.contains("WAIT_REF_MISS_OK"));
        assert!(s210.contains("CU-D-210 OK"));
        let l210 = s210.to_ascii_lowercase();
        assert!(!l210.contains("sendinput("));
        assert!(!l210.contains("[system.windows.forms.sendkeys"));
        let p220 = root.join("scripts/poc_cu_d_220.ps1");
        let s220 = std::fs::read_to_string(&p220).unwrap_or_default();
        assert!(s220.contains("KEY_DENIED"), "{}", p220.display());
        assert!(s220.contains("CU-D-220 OK"));
        assert!(s220.contains("return"));
        let l220 = s220.to_ascii_lowercase();
        assert!(!l220.contains("sendinput("));
        assert!(!l220.contains("[system.windows.forms.sendkeys"));
        let p230 = root.join("scripts/poc_cu_d_230.ps1");
        let s230 = std::fs::read_to_string(&p230).unwrap_or_default();
        assert!(s230.contains("TYPE_OK"), "{}", p230.display());
        assert!(s230.contains("NEWLINE_DENIED"));
        assert!(s230.contains("CU-D-230 OK"));
        let l230 = s230.to_ascii_lowercase();
        assert!(!l230.contains("sendinput("));
        assert!(!l230.contains("[system.windows.forms.sendkeys"));
        let p240 = root.join("scripts/poc_cu_d_240.ps1");
        let s240 = std::fs::read_to_string(&p240).unwrap_or_default();
        assert!(s240.contains("ABORT_OK"), "{}", p240.display());
        assert!(s240.contains("HUD_GONE"));
        assert!(s240.contains("ACT_DENIED"));
        assert!(s240.contains("CU-D-240 OK"));
        let l240 = s240.to_ascii_lowercase();
        assert!(!l240.contains("sendinput("));
        assert!(!l240.contains("[system.windows.forms.sendkeys"));
        let p250 = root.join("scripts/poc_cu_d_250.ps1");
        let s250 = std::fs::read_to_string(&p250).unwrap_or_default();
        assert!(s250.contains("HOVER_OK"), "{}", p250.display());
        assert!(s250.contains("GUIDE_FILE_OK"));
        assert!(s250.contains("guide_hover"));
        assert!(s250.contains("CU-D-250 OK"));
        let l250 = s250.to_ascii_lowercase();
        assert!(!l250.contains("sendinput("));
        assert!(!l250.contains("[system.windows.forms.sendkeys"));
        assert!(!l250.contains("mouse_event"));
        let p260 = root.join("scripts/poc_cu_d_260.ps1");
        let s260 = std::fs::read_to_string(&p260).unwrap_or_default();
        assert!(s260.contains("HOVER_OK"), "{}", p260.display());
        assert!(s260.contains("INVOKE_OK"));
        assert!(s260.contains("bm_click"));
        assert!(s260.contains("CU-D-260 OK"));
        let l260 = s260.to_ascii_lowercase();
        assert!(!l260.contains("sendinput("));
        assert!(!l260.contains("[system.windows.forms.sendkeys"));
        assert!(!l260.contains("mouse_event"));
        let p270 = root.join("scripts/poc_cu_d_270.ps1");
        let s270 = std::fs::read_to_string(&p270).unwrap_or_default();
        assert!(s270.contains("SCOPE_OK"), "{}", p270.display());
        assert!(s270.contains("BACKEND_OK"));
        assert!(s270.contains("windows_desktop_scope"));
        assert!(s270.contains("CU-D-270 OK"));
        let l270 = s270.to_ascii_lowercase();
        assert!(!l270.contains("sendinput("));
        let p280 = root.join("scripts/poc_cu_d_280.ps1");
        let s280 = std::fs::read_to_string(&p280).unwrap_or_default();
        assert!(s280.contains("TOOLS_OK"), "{}", p280.display());
        assert!(s280.contains("vcu_hover"));
        assert!(s280.contains("vcu_session_abort"));
        assert!(s280.contains("CU-D-280 OK"));
        let l280 = s280.to_ascii_lowercase();
        assert!(!l280.contains("sendinput("));
        let p290 = root.join("scripts/poc_cu_d_290.ps1");
        let s290 = std::fs::read_to_string(&p290).unwrap_or_default();
        assert!(s290.contains("HOVER_OK"), "{}", p290.display());
        assert!(s290.contains("ABORT_OK"));
        assert!(s290.contains("mcp_tools_call"));
        assert!(s290.contains("vcu_hover"));
        assert!(s290.contains("CU-D-290 OK"));
        let l290 = s290.to_ascii_lowercase();
        assert!(!l290.contains("sendinput("));
        assert!(!l290.contains("[system.windows.forms.sendkeys"));
        assert!(!l290.contains("mouse_event"));
        let p300 = root.join("scripts/poc_cu_d_300.ps1");
        let s300 = std::fs::read_to_string(&p300).unwrap_or_default();
        assert!(s300.contains("INVOKE_OK"), "{}", p300.display());
        assert!(s300.contains("bm_click"));
        assert!(s300.contains("mcp_tools_call"));
        assert!(s300.contains("vcu_click"));
        assert!(s300.contains("CU-D-300 OK"));
        let l300 = s300.to_ascii_lowercase();
        assert!(!l300.contains("sendinput("));
        assert!(!l300.contains("[system.windows.forms.sendkeys"));
        assert!(!l300.contains("mouse_event"));
        let p310 = root.join("scripts/poc_cu_d_310.ps1");
        let s310 = std::fs::read_to_string(&p310).unwrap_or_default();
        assert!(s310.contains("WAIT_OK"), "{}", p310.display());
        assert!(s310.contains("scene_wait"));
        assert!(s310.contains("mcp_tools_call"));
        assert!(s310.contains("vcu_wait"));
        assert!(s310.contains("CU-D-310 OK"));
        let l310 = s310.to_ascii_lowercase();
        assert!(!l310.contains("sendinput("));
        assert!(!l310.contains("[system.windows.forms.sendkeys"));
        assert!(!l310.contains("mouse_event"));
        let p320 = root.join("scripts/poc_cu_d_320.ps1");
        let s320 = std::fs::read_to_string(&p320).unwrap_or_default();
        assert!(s320.contains("TYPE_OK"), "{}", p320.display());
        assert!(s320.contains("wm_settext") || s320.contains("clipboard_paste"));
        assert!(s320.contains("mcp_tools_call"));
        assert!(s320.contains("vcu_type"));
        assert!(s320.contains("CU-D-320 OK"));
        let l320 = s320.to_ascii_lowercase();
        assert!(!l320.contains("sendinput("));
        assert!(!l320.contains("[system.windows.forms.sendkeys"));
        assert!(!l320.contains("mouse_event"));
        let p330 = root.join("scripts/poc_cu_d_330.ps1");
        let s330 = std::fs::read_to_string(&p330).unwrap_or_default();
        assert!(s330.contains("NEWLINE_DENIED"), "{}", p330.display());
        assert!(s330.contains("FocusPolicyViolation"));
        assert!(s330.contains("mcp_tools_call"));
        assert!(s330.contains("vcu_type"));
        assert!(s330.contains("CU-D-330 OK"));
        let l330 = s330.to_ascii_lowercase();
        assert!(!l330.contains("sendinput("));
        assert!(!l330.contains("[system.windows.forms.sendkeys"));
        assert!(!l330.contains("mouse_event"));
        let p340 = root.join("scripts/poc_cu_d_340.ps1");
        let s340 = std::fs::read_to_string(&p340).unwrap_or_default();
        assert!(s340.contains("SCROLL_OK"), "{}", p340.display());
        assert!(s340.contains("wm_vscroll") || s340.contains("uia_scroll"));
        assert!(s340.contains("mcp_tools_call"));
        assert!(s340.contains("vcu_scroll"));
        assert!(s340.contains("CU-D-340 OK"));
        let l340 = s340.to_ascii_lowercase();
        assert!(!l340.contains("sendinput("));
        assert!(!l340.contains("[system.windows.forms.sendkeys"));
        assert!(!l340.contains("mouse_event"));
        let p350 = root.join("scripts/poc_cu_d_350.ps1");
        let s350 = std::fs::read_to_string(&p350).unwrap_or_default();
        assert!(s350.contains("EXTRACT_OK"), "{}", p350.display());
        assert!(s350.contains("mcp_tools_call"));
        assert!(s350.contains("vcu_extract"));
        assert!(s350.contains("CU-D-350 OK"));
        let l350 = s350.to_ascii_lowercase();
        assert!(!l350.contains("sendinput("));
        assert!(!l350.contains("[system.windows.forms.sendkeys"));
        assert!(!l350.contains("mouse_event"));
        let p360 = root.join("scripts/poc_cu_d_360.ps1");
        let s360 = std::fs::read_to_string(&p360).unwrap_or_default();
        assert!(s360.contains("SHOT_OK"), "{}", p360.display());
        assert!(s360.contains("image/png"));
        assert!(s360.contains("mcp_tools_call"));
        assert!(s360.contains("vcu_screenshot"));
        assert!(s360.contains("CU-D-360 OK"));
        let l360 = s360.to_ascii_lowercase();
        assert!(!l360.contains("sendinput("));
        assert!(!l360.contains("[system.windows.forms.sendkeys"));
        assert!(!l360.contains("mouse_event"));
        assert!(!l360.contains("copyfromscreen"));
        let p370 = root.join("scripts/poc_cu_d_370.ps1");
        let s370 = std::fs::read_to_string(&p370).unwrap_or_default();
        assert!(s370.contains("KEY_DENIED"), "{}", p370.display());
        assert!(s370.contains("FocusPolicyViolation"));
        assert!(s370.contains("mcp_tools_call"));
        assert!(s370.contains("vcu_act"));
        assert!(s370.contains("CU-D-370 OK"));
        let l370 = s370.to_ascii_lowercase();
        assert!(!l370.contains("sendinput("));
        assert!(!l370.contains("[system.windows.forms.sendkeys"));
        assert!(!l370.contains("mouse_event"));
        let p380 = root.join("scripts/poc_cu_d_380.ps1");
        let s380 = std::fs::read_to_string(&p380).unwrap_or_default();
        assert!(s380.contains("WAIT_MISS_OK"), "{}", p380.display());
        assert!(s380.contains("WAIT_REF_MISS_OK"));
        assert!(s380.contains("ActionFailed"));
        assert!(s380.contains("mcp_tools_call"));
        assert!(s380.contains("vcu_wait"));
        assert!(s380.contains("CU-D-380 OK"));
        let l380 = s380.to_ascii_lowercase();
        assert!(!l380.contains("sendinput("));
        assert!(!l380.contains("[system.windows.forms.sendkeys"));
        assert!(!l380.contains("mouse_event"));
        let p390 = root.join("scripts/poc_cu_d_390.ps1");
        let s390 = std::fs::read_to_string(&p390).unwrap_or_default();
        assert!(s390.contains("SCOPE_OK"), "{}", p390.display());
        assert!(s390.contains("windows_desktop_scope"));
        assert!(s390.contains("mcp_tools_call"));
        assert!(s390.contains("vcu_doctor"));
        assert!(s390.contains("CU-D-390 OK"));
        let l390 = s390.to_ascii_lowercase();
        assert!(!l390.contains("sendinput("));
        assert!(!l390.contains("[system.windows.forms.sendkeys"));
        assert!(!l390.contains("mouse_event"));
        let p190 = root.join("scripts/poc_cu_d_190.ps1");
        let s190 = std::fs::read_to_string(&p190).unwrap_or_default();
        assert!(s190.contains("EXTRACT_OK"), "{}", p190.display());
        let l190 = s190.to_ascii_lowercase();
        assert!(!l190.contains("sendinput("));
        let p180 = root.join("scripts/poc_cu_d_180.ps1");
        let s180 = std::fs::read_to_string(&p180).unwrap_or_default();
        assert!(s180.contains("SCROLL_OK"), "{}", p180.display());
        let l180 = s180.to_ascii_lowercase();
        assert!(!l180.contains("sendinput("));
        assert!(!l180.contains("mouse_event"));
        let p170 = root.join("scripts/poc_cu_d_170.ps1");
        let s170 = std::fs::read_to_string(&p170).unwrap_or_default();
        assert!(s170.contains("CLICK_DENIED"), "{}", p170.display());
        assert!(s170.contains("TYPE_DENIED"));
        assert!(s170.contains("ms-settings:"));
        let l170 = s170.to_ascii_lowercase();
        assert!(!l170.contains("sendinput("));
        assert!(!l170.contains("[system.windows.forms.sendkeys"));
        let book = root.join("playbooks/desktop.md");
        let play = std::fs::read_to_string(&book).unwrap_or_default();
        assert!(play.contains("clipboard_paste"), "{}", book.display());
        assert!(play.contains("win:cmd:"));
        assert!(play.contains("FocusPolicyViolation"));
        let lplay = play.to_ascii_lowercase();
        assert!(!lplay.contains("sendinput("));
        assert!(!lplay.contains("[system.windows.forms.sendkeys"));
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn windows_list_and_snapshot_are_host_gated_off_windows() {
        let b = WindowsAppBackend::new();
        let list = b.list_windows().await.unwrap_err();
        assert_eq!(list.code(), ErrorCode::NotImplemented);
        let snap = b.snapshot("win:notepad:1", 1000).await.unwrap_err();
        assert_eq!(snap.code(), ErrorCode::NotImplemented);
    }
}
