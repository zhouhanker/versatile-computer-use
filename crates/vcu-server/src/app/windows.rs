//! Windows app adapter (PowerShell list on Windows hosts).
use async_trait::async_trait;
use vcu_core::{ErrorCode, VcuError, VcuResult};
use super::{is_denied_app, AppBackend, AppElement, AppSnapshot, AppTarget};

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
                "powershell".into(),
                "cmd".into(),
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
        self.allowlist.iter().any(|a| lower.contains(&a.to_lowercase()))
    }

    #[cfg(windows)]
    fn run_powershell(script: &str) -> VcuResult<String> {
        use std::process::Command;
        let path = std::env::temp_dir().join(format!("vcu-ps-{}.ps1", ulid::Ulid::new()));
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(script.as_bytes());
        std::fs::write(&path, &bytes).map_err(|e| {
            VcuError::with_detail(ErrorCode::Internal, "write powershell script", e.to_string())
        })?;
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-STA",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                path.to_string_lossy().as_ref(),
            ])
            .output();
        let _ = std::fs::remove_file(&path);
        let output = output.map_err(|e| {
            VcuError::with_detail(ErrorCode::Internal, "powershell spawn failed", e.to_string())
        })?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(VcuError::with_detail(ErrorCode::ActionFailed, "powershell failed", err));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

/// Parse `name\tpid\ttitle` lines from the Windows process listing script.
pub fn parse_process_list_lines(raw: &str, allowed: impl Fn(&str) -> bool) -> Vec<AppTarget> {
    let mut out = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let mut parts = line.split('\t');
        let name = parts.next().unwrap_or("").trim();
        let pid = parts.next().and_then(|s| s.trim().parse().ok());
        let title = parts.next().unwrap_or(name).trim();
        if name.is_empty() {
            continue;
        }
        if is_denied_app(name) || is_denied_app(title) {
            continue;
        }
        if !allowed(name) && !allowed(title) {
            continue;
        }
        out.push(AppTarget {
            id: format!("win:{}:{}", name.replace(' ', "_"), pid.unwrap_or(idx as i32)),
            title: if title.is_empty() {
                name.to_string()
            } else {
                title.to_string()
            },
            bundle_or_exe: name.to_string(),
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

pub fn uia_tree_script(pid: i32, max_nodes: i32) -> String {
    format!(
        r#"
Add-Type -AssemblyName UIAutomationClient | Out-Null
$pid = {pid}
$max = {max}
$root = [System.Windows.Automation.AutomationElement]::RootElement
$cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $pid)
$win = $root.FindFirst([System.Windows.Automation.TreeScope]::Children, $cond)
if ($null -eq $win) {{ 'MISSING'; exit 0 }}
$q = New-Object System.Collections.Queue
$q.Enqueue($win)
$n = 0
while ($q.Count -gt 0 -and $n -lt $max) {{
  $el = $q.Dequeue()
  $n++
  $ct = $el.Current.ControlType.ProgrammaticName
  $nm = ($el.Current.Name -replace '[\r\n\|]', ' ')
  $cls = ($el.Current.ClassName -replace '[\r\n\|]', ' ')
  $r = $el.Current.BoundingRectangle
  '{{0}}|{{1}}|{{2}}|{{3}},{{4}},{{5}},{{6}}|{{7}}' -f ("e$n"), $ct, $nm, [int]$r.X, [int]$r.Y, [int]$r.Width, [int]$r.Height, $cls
  if ($n -ge $max) {{ break }}
  $kids = $el.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
  foreach ($k in $kids) {{ $q.Enqueue($k) }}
}}
"#,
        pid = pid,
        max = max_nodes
    )
}

/// PowerShell: InvokePattern on the Nth UIA node (eN). No SendInput.
pub fn uia_invoke_script(pid: i32, eref: &str) -> String {
    let n = eref.trim_start_matches('e').parse::<i32>().unwrap_or(0);
    format!(
        r#"
Add-Type -AssemblyName UIAutomationClient | Out-Null
$pid = {pid}
$want = {n}
$root = [System.Windows.Automation.AutomationElement]::RootElement
$cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $pid)
$win = $root.FindFirst([System.Windows.Automation.TreeScope]::Children, $cond)
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
    }} catch {{
      'error:no-invoke-pattern'
    }}
    exit 0
  }}
  $kids = $el.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
  foreach ($k in $kids) {{ $q.Enqueue($k) }}
}}
'not-found'
"#,
        pid = pid,
        n = n
    )
}

/// PowerShell: ValuePattern.SetValue on eN, else WM_SETTEXT. No SendInput.
pub fn uia_set_value_script(pid: i32, eref: &str, value: &str) -> String {
    let n = eref.trim_start_matches('e').parse::<i32>().unwrap_or(0);
    let val = value.replace('\'', "''");
    format!(
        r#"
Add-Type -AssemblyName UIAutomationClient | Out-Null
if (-not ("Vcu.VcuSetValue070" -as [type])) {{
  $sig = @'
[DllImport("user32.dll", CharSet=CharSet.Unicode)]
public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, string lParam);
'@
  Add-Type -MemberDefinition $sig -Name VcuSetValue070 -Namespace Vcu | Out-Null
}}
$pid = {pid}
$want = {n}
$val = '{val}'
$root = [System.Windows.Automation.AutomationElement]::RootElement
$cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $pid)
$win = $root.FindFirst([System.Windows.Automation.TreeScope]::Children, $cond)
if ($null -eq $win) {{ 'not-found'; exit 0 }}
$q = New-Object System.Collections.Queue
$q.Enqueue($win)
$i = 0
while ($q.Count -gt 0) {{
  $el = $q.Dequeue()
  $i++
  if ($i -eq $want) {{
    $pat = [System.Windows.Automation.ValuePattern]::Pattern
    try {{
      $vp = $el.GetCurrentPattern($pat)
      $vp.SetValue($val)
      'ok:uia_set_value'
      exit 0
    }} catch {{}}
    $nh = [int64]$el.Current.NativeWindowHandle
    if ($nh -ne 0) {{
      [void][Vcu.VcuSetValue070]::SendMessage([IntPtr]$nh, 12, [IntPtr]::Zero, $val)
      'ok:wm_settext'
      exit 0
    }}
    'error:no-value-pattern'
    exit 0
  }}
  $kids = $el.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
  foreach ($k in $kids) {{ $q.Enqueue($k) }}
}}
'not-found'
"#,
        pid = pid,
        n = n,
        val = val,
    )
}

/// PowerShell: PrintWindow of the process main HWND to PNG (base64). Not CopyFromScreen of an occluded desktop, not SendInput.
pub fn uia_capture_script(pid: i32) -> String {
    format!(
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
$proc = Get-Process -Id {pid} -ErrorAction SilentlyContinue
if ($null -eq $proc -or $proc.MainWindowHandle -eq [IntPtr]::Zero) {{ 'MISSING'; exit 0 }}
$hwnd = $proc.MainWindowHandle
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
$b64 = [Convert]::ToBase64String($ms.ToArray())
$ms.Dispose()
'FRAME|{{0}},{{1}},{{2}},{{3}}' -f $rect.Left, $rect.Top, $w, $h
$b64
"#,
        pid = pid
    )
}

pub fn parse_uia_capture_output(raw: &str) -> Option<(Vec<u8>, [f64; 4])> {
    let mut lines = raw.lines().map(|l| l.trim()).filter(|l| !l.is_empty());
    let header = lines.next()?;
    let rest = header.strip_prefix("FRAME|")?;
    let nums: Vec<f64> = rest.split(',').filter_map(|s| s.trim().parse().ok()).collect();
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
        if eref.is_empty() {
            continue;
        }
        if !class.is_empty() {
            let cl = class.to_ascii_lowercase();
            if cl == "edit" || cl == "document" || cl.contains("richedit") {
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
            value: None,
            frame,
        });
    }
    out
}

impl Default for WindowsAppBackend {
    fn default() -> Self { Self::new() }
}


pub fn snapshot_from_uia(id: &str, name: &str, pid: Option<i32>, raw: &str, budget: u64) -> AppSnapshot {
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
        summary: format!("process=\"{name}\" elements={} note=uia_tree", elements.len()),
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

pub fn invoke_from_uia_output(name: &str, element_ref: &str, out: &str) -> VcuResult<serde_json::Value> {
    if out.contains("ok:uia_invoke") {
        Ok(serde_json::json!({
            "ok": true,
            "process": name,
            "ref": element_ref,
            "result": out,
            "input_path": "uia_invoke",
            "os_cursor_used": false,
            "hid_injected": false
        }))
    } else {
        Err(VcuError::with_detail(ErrorCode::ActionFailed, "uia invoke failed", out))
    }
}

pub fn set_value_from_uia_output(name: &str, element_ref: &str, out: &str) -> VcuResult<serde_json::Value> {
    if out.contains("ok:uia_set_value") || out.contains("ok:wm_settext") {
        let path = if out.contains("ok:uia_set_value") {
            "uia_set_value"
        } else {
            "wm_settext"
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
        Err(VcuError::with_detail(ErrorCode::ActionFailed, "uia set_value failed", out))
    }
}

#[async_trait]
impl AppBackend for WindowsAppBackend {
    fn platform(&self) -> &str { "windows" }

    async fn list_windows(&self) -> VcuResult<Vec<AppTarget>> {
        #[cfg(not(windows))]
        {
            Err(VcuError::coded(ErrorCode::NotImplemented, "WindowsAppBackend::list_windows only runs on Windows hosts"))
        }
        #[cfg(windows)]
        {
            let script = r#"
Get-Process | Where-Object { $_.MainWindowTitle -ne '' } |
  ForEach-Object { '{0}`t{1}`t{2}' -f $_.ProcessName, $_.Id, ($_.MainWindowTitle -replace '[\r\n\t]',' ') }
"#;
            let raw = Self::run_powershell(script)?;
            Ok(parse_process_list_lines(&raw, |n| self.allowed(n)))
        }
    }

    async fn focus_window(&mut self, _id: &str, allow_focus_steal: bool) -> VcuResult<()> {
        if !allow_focus_steal {
            return Err(VcuError::coded(ErrorCode::FocusPolicyViolation, "refusing to steal app focus on Windows"));
        }
        Err(VcuError::coded(ErrorCode::NotImplemented, "explicit Windows focus is gated"))
    }

    async fn snapshot(&self, id: &str, budget: u64) -> VcuResult<AppSnapshot> {
        #[cfg(not(windows))]
        {
            let _ = budget;
            Err(VcuError::coded(ErrorCode::NotImplemented, format!("Windows snapshot unavailable on this host for {id}")))
        }
        #[cfg(windows)]
        {
            let name = id.strip_prefix("win:").and_then(|rest| rest.split(':').next()).map(|s| s.replace('_', " ")).unwrap_or_else(|| id.to_string());
            if !self.allowed(&name) {
                return Err(VcuError::coded(ErrorCode::FocusPolicyViolation, format!("process '{name}' not in app allowlist")));
            }
            let pid = pid_from_win_id(id).ok_or_else(|| {
                VcuError::coded(ErrorCode::InvalidInput, "Windows snapshot requires win:name:pid")
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
            return Err(VcuError::coded(ErrorCode::AppDenied, format!("app '{name}' is denied by VCU policy")));
        }
        #[cfg(not(windows))]
        {
            let _ = (id, element_ref);
            Err(VcuError::coded(ErrorCode::OsCursorDenied, "Windows app invoke is denied by default safety policy (no SendInput / no cursor)"))
        }
        #[cfg(windows)]
        {
            if !self.allowed(&name) {
                return Err(VcuError::coded(ErrorCode::FocusPolicyViolation, format!("process '{name}' not in app allowlist")));
            }
            let pid = pid_from_win_id(id).ok_or_else(|| {
                VcuError::coded(ErrorCode::InvalidInput, "Windows invoke requires win:name:pid")
            })?;
            let out = Self::run_powershell(&uia_invoke_script(pid, element_ref))?;
            invoke_from_uia_output(&name, element_ref, &out)
        }
    }

    async fn set_value(&mut self, id: &str, element_ref: &str, value: &str) -> VcuResult<serde_json::Value> {
        #[cfg(not(windows))]
        {
            let _ = (id, element_ref, value);
            Err(VcuError::coded(ErrorCode::NotImplemented, "Windows set_value only runs on Windows hosts"))
        }
        #[cfg(windows)]
        {
            let name = id
                .strip_prefix("win:")
                .and_then(|rest| rest.split(':').next())
                .map(|s| s.replace('_', " "))
                .unwrap_or_else(|| id.to_string());
            if is_denied_app(&name) {
                return Err(VcuError::coded(ErrorCode::AppDenied, format!("app '{name}' is denied by VCU policy")));
            }
            if !self.allowed(&name) {
                return Err(VcuError::coded(ErrorCode::FocusPolicyViolation, format!("process '{name}' not in app allowlist")));
            }
            let pid = pid_from_win_id(id).ok_or_else(|| {
                VcuError::coded(ErrorCode::InvalidInput, "Windows set_value requires win:name:pid")
            })?;
            let out = Self::run_powershell(&uia_set_value_script(pid, element_ref, value))?;
            set_value_from_uia_output(&name, element_ref, &out)
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
            if is_denied_app(&name) || !self.allowed(&name) {
                return Ok(None);
            }
            let Some(pid) = pid_from_win_id(id) else {
                return Ok(None);
            };
            let raw = Self::run_powershell(&uia_capture_script(pid))?;
            let Some((png, frame)) = parse_uia_capture_output(&raw) else {
                return Ok(None);
            };
            let Some((width, height)) = super::png_ihdr_size(&png) else {
                return Ok(None);
            };
            Ok(Some(super::AppCapture { png, width, height, frame }))
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
            assert!(err.message().to_ascii_lowercase().contains("sendinput") || err.message().to_ascii_lowercase().contains("cursor"));
        }
        #[cfg(windows)]
        {
            assert_eq!(err.code(), ErrorCode::FocusPolicyViolation);
        }
        let denied = b.invoke("win:WeChat:2", "e1").await.unwrap_err();
        assert_eq!(denied.code(), ErrorCode::AppDenied);
        let err = b.set_value("win:notepad:1", "e1", "hi").await.unwrap_err();
        #[cfg(not(windows))]
        assert_eq!(err.code(), ErrorCode::NotImplemented);
        #[cfg(windows)]
        assert_eq!(err.code(), ErrorCode::ActionFailed);
        let setv = uia_set_value_script(4242, "e2", "hello");
        assert!(setv.contains("ValuePattern"));
        assert!(setv.contains("SetValue"));
        assert!(setv.contains("hello"));
        assert!(setv.contains("ok:wm_settext"));
        assert!(!setv.to_ascii_lowercase().contains("sendinput("));
        let cap = b.capture_window("win:notepad:1").await.unwrap();
        assert!(cap.is_none());
    }

    #[test]
    fn parse_process_list_keeps_allowlist_drops_wechat() {
        let b = WindowsAppBackend::new();
        let raw = "notepad\t1001\tUntitled - Notepad\nWeChat\t2002\tWeChat\nexplorer\t3003\tDocuments\nmsedge\t4004\tMicrosoft Edge\n";
        let wins = parse_process_list_lines(raw, |n| b.allowed(n));
        let names: Vec<_> = wins.iter().map(|w| w.bundle_or_exe.as_str()).collect();
        assert!(names.contains(&"notepad"));
        assert!(names.contains(&"explorer"));
        assert!(names.contains(&"msedge"));
        assert!(!names.iter().any(|n| n.to_lowercase().contains("wechat")));
        assert!(wins.iter().any(|w| w.id.starts_with("win:notepad:")));
    }

    #[test]
    fn parse_uia_tree_and_scripts_are_pattern_not_hid() {
        let els = parse_uia_element_lines(
            "e1|ControlType.Window|Notepad|10,10,800,600\ne2|ControlType.Button|Save|20,40,80,24\nMISSING\ne3|ControlType.Pane||10,40,780,540|Edit\n"        );
        assert_eq!(els.len(), 3);
        assert_eq!(els[0].r#ref, "e1");
        assert_eq!(els[1].name, "Save");
        assert_eq!(els[1].frame, Some([20.0, 40.0, 80.0, 24.0]));
        assert_eq!(els[2].role, "ControlType.Pane/Edit");
        let tree = uia_tree_script(4242, 80);
        assert!(tree.contains("UIAutomationClient"));
        assert!(tree.contains("ProcessIdProperty"));
        assert!(tree.contains("ClassName"));
        assert!(!tree.to_ascii_lowercase().contains("sendinput"));
        let inv = uia_invoke_script(4242, "e2");
        assert!(inv.contains("InvokePattern"));
        assert!(inv.contains("ok:uia_invoke"));
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
        assert!(invoke_from_uia_output("notepad", "e2", "not-found").is_err());
        let typed = set_value_from_uia_output("notepad", "e2", "ok:uia_set_value").unwrap();
        assert_eq!(typed["input_path"], "uia_set_value");
        let wm = set_value_from_uia_output("notepad", "e2", "ok:wm_settext").unwrap();
        assert_eq!(wm["input_path"], "wm_settext");
        assert_eq!(wm["os_cursor_used"], false);
        let setv = uia_set_value_script(4242, "e2", "hello");
        assert!(setv.contains("ok:wm_settext"));
        assert!(setv.contains("SendMessage"));
        assert!(!setv.to_ascii_lowercase().contains("sendinput("));
        assert!(set_value_from_uia_output("notepad", "e2", "error:no-value-pattern").is_err());
        let cap_script = uia_capture_script(4242);
        assert!(cap_script.contains("PrintWindow"));
        assert!(cap_script.contains("GetWindowRect"));
        assert!(!cap_script.to_ascii_lowercase().contains("sendinput"));
        assert!(!cap_script.to_ascii_lowercase().contains("mouse_event"));
        assert!(!cap_script.to_ascii_lowercase().contains("copyfromscreen"));
        let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let parsed = parse_uia_capture_output(&format!("FRAME|10,20,800,600\n{b64}\n")).expect("parse capture");
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
