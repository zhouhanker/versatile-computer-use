//! Stage presenter: Banner + Guide overlay.
//! Guide is drawn in the overlay; the OS cursor is never warped.
//!
//! Live presenter prefers the native AppKit helper `vcu-stage` (LSUIElement-style
//! accessory process). JXA/osascript remains a fallback only.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use vcu_core::{ErrorCode, VcuError, VcuResult};

const BANNER: &str = "VCU 正在使用这台 Mac    按 Escape 取消";
#[allow(dead_code)]
const BANNER_WINDOWS: &str = "VCU 正在使用这台 PC    按 Escape 取消";

/// WinForms HUD + Guide. No SendInput / cursor warp. Escape writes the abort file.
#[allow(dead_code)]
const STAGE_WINPS: &str = r#"
Add-Type -AssemblyName System.Windows.Forms | Out-Null
Add-Type -AssemblyName System.Drawing | Out-Null
$controlPath = $args[0]
if (-not $controlPath) { exit 1 }
$abortPath = [System.IO.Path]::ChangeExtension($controlPath, 'abort')
function Read-Control {
  if (-not (Test-Path -LiteralPath $controlPath)) { return $null }
  try {
    return (Get-Content -LiteralPath $controlPath -Raw -ErrorAction Stop | ConvertFrom-Json)
  } catch { return $null }
}
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
$form = New-Object System.Windows.Forms.Form
$form.Text = 'VCU'
$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None
$form.TopMost = $true
$form.ShowInTaskbar = $false
$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual
$form.Width = 320
$form.Height = 32
$form.Left = [int]($screen.Left + ($screen.Width - 320) / 2)
$form.Top = [int]($screen.Top + 8)
$form.BackColor = [System.Drawing.Color]::FromArgb(18, 46, 107)
$form.KeyPreview = $true
$form.Add_KeyDown({
  if ($_.KeyCode -eq [System.Windows.Forms.Keys]::Escape) {
    [System.IO.File]::WriteAllText($abortPath, '1')
  }
})
$label = New-Object System.Windows.Forms.Label
$label.Text = 'VCU 正在使用这台 PC    Esc 取消'
$label.ForeColor = [System.Drawing.Color]::White
$label.AutoSize = $false
$label.Width = 320
$label.Height = 32
$label.TextAlign = [System.Drawing.ContentAlignment]::MiddleCenter
$form.Controls.Add($label)
$guide = New-Object System.Windows.Forms.Form
$guide.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None
$guide.TopMost = $true
$guide.ShowInTaskbar = $false
$guide.Width = 48
$guide.Height = 48
$guide.BackColor = [System.Drawing.Color]::FromArgb(255, 107, 56)
$guide.Opacity = 0.85
$guide.Visible = $false
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 80
$timer.Add_Tick({
  $c = Read-Control
  if ($null -eq $c) { return }
  if ($c.stop) { $timer.Stop(); $form.Close(); return }
  if ($c.PSObject.Properties.Name -contains 'hud') {
    if ($c.hud -eq $false) { $form.Hide() } else { $form.Show() }
  }
  if ($c.guide) {
    $gx = 0; $gy = 0
    try { $gx = [int]$c.guide.x } catch {}
    try { $gy = [int]$c.guide.y } catch {}
    $guide.Left = $gx - 24
    $guide.Top = $gy - 24
    if ($c.guide.visible -eq $false) { $guide.Hide() } else { $guide.Show() }
  }
})
$timer.Start()
$form.Add_FormClosed({ $timer.Stop(); try { $guide.Close() } catch {} })
[System.Windows.Forms.Application]::Run($form)
"#;

const STAGE_JXA: &str = r#"
ObjC.import('Cocoa');
ObjC.import('Foundation');

function readControl(path) {
  const str = $.NSString.stringWithContentsOfFileEncodingError(path, $.NSUTF8StringEncoding, null);
  if (!str) return null;
  try { return JSON.parse(ObjC.unwrap(str)); } catch (e) { return null; }
}

function run(argv) {
  const app = $.NSApplication.sharedApplication;
  try { app.setActivationPolicy(1); } catch (e) {}
  const controlPath = (argv && argv.length) ? argv[0] : null;
  const vis = $.NSScreen.mainScreen.visibleFrame;
  const width = 280;
  const height = 28;
  const x = vis.origin.x + (vis.size.width - width) / 2;
  const y = vis.origin.y + vis.size.height - height - 8;
  const win = $.NSWindow.alloc.initWithContentRectStyleMaskBackingDefer(
    $.NSMakeRect(x, y, width, height),
    0,
    2,
    false
  );
  win.level = 25;
  win.opaque = false;
  win.hasShadow = true;
  win.ignoresMouseEvents = false;
  win.collectionBehavior = 337;
  win.backgroundColor = $.NSColor.clearColor;
  try {
    win.contentView.wantsLayer = true;
    win.contentView.layer.cornerRadius = 16;
    win.contentView.layer.backgroundColor = $.NSColor.colorWithCalibratedRedGreenBlueAlpha(0.07, 0.18, 0.42, 0.96).CGColor;
  } catch (e) {
    win.backgroundColor = $.NSColor.colorWithCalibratedRedGreenBlueAlpha(0.07, 0.18, 0.42, 0.96);
  }
  const label = $.NSTextField.alloc.initWithFrame($.NSMakeRect(12, 8, 188, 16));
  label.stringValue = 'VCU 正在使用这台 Mac';
  label.bezeled = false;
  label.drawsBackground = false;
  label.editable = false;
  label.selectable = false;
  label.textColor = $.NSColor.whiteColor;
  try { label.font = $.NSFont.systemFontOfSize(11); } catch (e) {}
  const sub = $.NSTextField.alloc.initWithFrame($.NSMakeRect(200, 8, 88, 16));
  sub.stringValue = 'Esc 取消';
  sub.bezeled = false;
  sub.drawsBackground = false;
  sub.editable = false;
  sub.selectable = false;
  sub.textColor = $.NSColor.colorWithCalibratedWhiteAlpha(0.82, 1);
  try { sub.alignment = 2; } catch (e) {}
  win.contentView.addSubview(label);
  win.contentView.addSubview(sub);
  win.orderFront(null);

  const gsize = 48;
  const guide = $.NSWindow.alloc.initWithContentRectStyleMaskBackingDefer(
    $.NSMakeRect(0, 0, gsize, gsize),
    0,
    2,
    false
  );
  guide.level = 26;
  guide.opaque = false;
  guide.hasShadow = false;
  guide.ignoresMouseEvents = true;
  guide.collectionBehavior = 337;
  guide.backgroundColor = $.NSColor.clearColor;
  try {
    guide.contentView.wantsLayer = true;
    guide.contentView.layer.cornerRadius = gsize / 2;
    guide.contentView.layer.borderWidth = 3;
    guide.contentView.layer.borderColor = $.NSColor.colorWithCalibratedRedGreenBlueAlpha(0.25, 0.55, 1, 0.95).CGColor;
    guide.contentView.layer.backgroundColor = $.NSColor.colorWithCalibratedRedGreenBlueAlpha(1, 0.42, 0.22, 0.35).CGColor;
  } catch (e) {
    guide.backgroundColor = $.NSColor.colorWithCalibratedRedGreenBlueAlpha(1, 0.42, 0.22, 0.9);
  }

  function applyGuide(gx, gy, visible) {
    const cocoaY = vis.origin.y + vis.size.height - gy - (gsize / 2);
    const cocoaX = vis.origin.x + gx - (gsize / 2);
    guide.setFrameOrigin($.NSMakePoint(cocoaX, cocoaY));
    if (visible) { guide.orderFront(null); }
    else { guide.orderOut(null); }
  }

  let abortSent = false;
  function abortPath() {
    if (!controlPath) return null;
    return String(controlPath).replace(/\.json$/i, '.abort');
  }
  function requestAbort() {
    const p = abortPath();
    if (!p || abortSent) return;
    abortSent = true;
    const s = $.NSString.alloc.initWithUTF8String('1');
    s.writeToFileAtomicallyEncodingError(p, true, $.NSUTF8StringEncoding, null);
  }
  try {
    $.NSEvent.addGlobalMonitorForEventsMatchingMaskHandler($.NSEventMaskKeyDown, function(ev) {
      if (ev && ev.keyCode === 53) requestAbort();
    });
  } catch (e) {}
  try {
    $.NSEvent.addLocalMonitorForEventsMatchingMaskHandler($.NSEventMaskKeyDown, function(ev) {
      if (ev && ev.keyCode === 53) requestAbort();
      return ev;
    });
  } catch (e) {}

  while (true) {
    if (controlPath) {
      const c = readControl(controlPath);
      if (c) {
        if (c.stop) break;
        if (c.hud === false) { try { win.orderOut(null); } catch (e) {} }
        else if (c.hud === true) { try { win.orderFront(null); } catch (e) {} }
        if (c.guide) {
          applyGuide(Number(c.guide.x) || 0, Number(c.guide.y) || 0, c.guide.visible !== false);
        }
      }
    }
    $.NSRunLoop.currentRunLoop.runUntilDate($.NSDate.dateWithTimeIntervalSinceNow(0.05));
  }
  try { win.orderOut(null); } catch (e) {}
  try { guide.orderOut(null); } catch (e) {}
}
"#;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuidePos {
    pub x: f64,
    pub y: f64,
    pub visible: bool,
}

pub struct StageHandle {
    child: Mutex<Option<Child>>,
    script_path: Option<PathBuf>,
    control_path: Option<PathBuf>,
    abort_path: Option<PathBuf>,
    last_guide: Mutex<Option<GuidePos>>,
    pub shown: bool,
    pub mock: bool,
    pub presenter: &'static str,
}

impl StageHandle {
    pub fn noop() -> Self {
        Self {
            child: Mutex::new(None),
            script_path: None,
            control_path: None,
            abort_path: None,
            last_guide: Mutex::new(None),
            shown: true,
            mock: true,
            presenter: "noop",
        }
    }

    /// CU-D-010: a Stage that never became visible must not start a desktop session.
    pub fn hidden() -> Self {
        let mut s = Self::noop();
        s.shown = false;
        s.presenter = "hidden";
        s
    }

    pub fn noop_with_abort(abort_path: PathBuf) -> Self {
        let mut s = Self::noop();
        s.abort_path = Some(abort_path);
        s
    }

    pub fn raise_for_platform(platform: &str) -> VcuResult<Self> {
        if platform == "mock-app" {
            let abort = std::env::temp_dir().join(format!(
                "vcu-stage-mock-{}.abort",
                ulid::Ulid::new()
            ));
            return Ok(Self::noop_with_abort(abort));
        }
        Self::raise_live()
    }

    pub fn raise_live() -> VcuResult<Self> {
        #[cfg(target_os = "macos")]
        {
            let token = ulid::Ulid::new().to_string();
            let control_path = std::env::temp_dir().join(format!("vcu-stage-{token}.json"));
            write_control(&control_path, false, None, Some(true))?;
            if let Some(handle) = try_spawn_native(&token, &control_path) {
                return Ok(handle);
            }
            spawn_jxa_fallback(&token, control_path)
        }
        #[cfg(windows)]
        {
            spawn_winforms_hud()
        }
        #[cfg(not(any(target_os = "macos", windows)))]
        {
            Err(VcuError::coded(
                ErrorCode::StageRequired,
                "desktop Stage HUD is macOS/Windows in this slice",
            ))
        }
    }

    pub fn move_guide(&self, x: f64, y: f64) -> VcuResult<GuidePos> {
        let pos = GuidePos {
            x,
            y,
            visible: true,
        };
        if let Ok(mut g) = self.last_guide.lock() {
            *g = Some(pos);
        }
        if let Some(path) = &self.control_path {
            write_control(path, false, Some(pos), None)?;
        }
        Ok(pos)
    }

    pub fn last_guide(&self) -> Option<GuidePos> {
        self.last_guide.lock().ok().and_then(|g| *g)
    }

    /// Show Guide at an AX point without the HUD capsule, then tear down.
    /// Login-state click uses this so retina mapping is visible without blocking the menu bar.
    pub fn flash_guide(x: f64, y: f64, hold_ms: u64) -> VcuResult<GuidePos> {
        let pos = GuidePos {
            x,
            y,
            visible: true,
        };
        if cfg!(not(target_os = "macos")) {
            return Ok(pos);
        }
        let mut stage = Self::raise_live()?;
        if let Some(path) = &stage.control_path {
            write_control(path, false, Some(pos), Some(false))?;
        }
        if let Ok(mut g) = stage.last_guide.lock() {
            *g = Some(pos);
        }
        std::thread::sleep(std::time::Duration::from_millis(hold_ms.max(80)));
        stage.teardown();
        Ok(pos)
    }

    pub fn abort_watch_path(&self) -> Option<PathBuf> {
        self.abort_path.clone()
    }

    pub fn abort_requested(&self) -> bool {
        self.abort_path.as_ref().is_some_and(|p| p.is_file())
    }

    pub fn write_abort_signal(&self) -> VcuResult<()> {
        let path = self.abort_path.as_ref().ok_or_else(|| {
            VcuError::coded(ErrorCode::Internal, "stage abort path missing")
        })?;
        let tmp = path.with_extension("abort.tmp");
        std::fs::write(&tmp, b"1").map_err(|e| {
            VcuError::with_detail(ErrorCode::Internal, "write stage abort", e.to_string())
        })?;
        std::fs::rename(&tmp, path).map_err(|e| {
            VcuError::with_detail(ErrorCode::Internal, "publish stage abort", e.to_string())
        })?;
        Ok(())
    }

    pub fn teardown(&mut self) {
        if let Some(path) = &self.control_path {
            let _ = write_control(path, true, self.last_guide(), None);
        }
        if let Ok(mut g) = self.child.lock() {
            if let Some(mut child) = g.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        if let Some(path) = self.script_path.take() {
            let _ = std::fs::remove_file(path);
        }
        if let Some(path) = self.control_path.take() {
            let _ = std::fs::remove_file(path);
        }
        if let Some(path) = self.abort_path.take() {
            let _ = std::fs::remove_file(path);
        }
        if !self.mock {
            self.shown = false;
        }
    }
}

impl Drop for StageHandle {
    fn drop(&mut self) {
        self.teardown();
    }
}

pub fn banner_text() -> &'static str {
    #[cfg(windows)]
    {
        BANNER_WINDOWS
    }
    #[cfg(not(windows))]
    {
        BANNER
    }
}

pub fn resolve_stage_bin() -> Option<PathBuf> {
    resolve_stage_bin_from(
        std::env::var_os("VCU_STAGE_BIN").map(PathBuf::from),
        std::env::current_exe().ok(),
        std::env::var("PATH").ok(),
    )
}

fn resolve_stage_bin_from(
    env_bin: Option<PathBuf>,
    current_exe: Option<PathBuf>,
    path_var: Option<String>,
) -> Option<PathBuf> {
    if let Some(p) = env_bin {
        if p.is_file() {
            return Some(p);
        }
    }
    if let Some(exe) = current_exe {
        if let Some(dir) = exe.parent() {
            let cand = dir.join("vcu-stage");
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    if let Some(path) = path_var {
        for dir in path.split(':') {
            if dir.is_empty() {
                continue;
            }
            let cand = Path::new(dir).join("vcu-stage");
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

fn spawn_logged(bin: &Path, args: &[&str], log_path: &Path) -> std::io::Result<Child> {
    let log = std::fs::File::create(log_path)?;
    Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(std::process::Stdio::from(log))
        .spawn()
}

fn child_still_running(child: &mut Child, log_path: &Path) -> bool {
    std::thread::sleep(std::time::Duration::from_millis(250));
    match child.try_wait() {
        Ok(None) => true,
        Ok(Some(status)) => {
            let err = std::fs::read_to_string(log_path).unwrap_or_default();
            let _ = status;
            let _ = err;
            false
        }
        Err(_) => false,
    }
}

#[cfg(target_os = "macos")]
fn try_spawn_native(token: &str, control_path: &Path) -> Option<StageHandle> {
    let bin = resolve_stage_bin()?;
    let log_path = std::env::temp_dir().join(format!("vcu-stage-{token}.log"));
    let control = control_path.to_string_lossy();
    let mut child = spawn_logged(
        &bin,
        &["--control", control.as_ref()],
        &log_path,
    )
    .ok()?;
    if !child_still_running(&mut child, &log_path) {
        return None;
    }
    Some(StageHandle {
        child: Mutex::new(Some(child)),
        script_path: None,
        control_path: Some(control_path.to_path_buf()),
        abort_path: Some(control_path.with_extension("abort")),
        last_guide: Mutex::new(None),
        shown: true,
        mock: false,
        presenter: "native",
    })
}

#[cfg(target_os = "macos")]
fn spawn_jxa_fallback(token: &str, control_path: PathBuf) -> VcuResult<StageHandle> {
    let script_path = std::env::temp_dir().join(format!("vcu-stage-{token}.jxa"));
    {
        let mut f = std::fs::File::create(&script_path).map_err(|e| {
            VcuError::with_detail(ErrorCode::Internal, "write stage script", e.to_string())
        })?;
        f.write_all(STAGE_JXA.as_bytes()).map_err(|e| {
            VcuError::with_detail(ErrorCode::Internal, "write stage script", e.to_string())
        })?;
    }
    let log_path = std::env::temp_dir().join(format!("vcu-stage-{token}.log"));
    let mut child = spawn_logged(
        Path::new("osascript"),
        &[
            "-l",
            "JavaScript",
            script_path.to_string_lossy().as_ref(),
            control_path.to_string_lossy().as_ref(),
        ],
        &log_path,
    )
    .map_err(|e| {
        let _ = std::fs::remove_file(&script_path);
        let _ = std::fs::remove_file(&control_path);
        VcuError::with_detail(
            ErrorCode::Internal,
            "failed to raise Stage banner",
            e.to_string(),
        )
    })?;
    if !child_still_running(&mut child, &log_path) {
        let err = std::fs::read_to_string(&log_path).unwrap_or_default();
        let _ = std::fs::remove_file(&script_path);
        let _ = std::fs::remove_file(&control_path);
        return Err(VcuError::with_detail(
            ErrorCode::Internal,
            "Stage banner process exited",
            err,
        ));
    }
    Ok(StageHandle {
        child: Mutex::new(Some(child)),
        script_path: Some(script_path),
        control_path: Some(control_path.clone()),
        abort_path: Some(control_path.with_extension("abort")),
        last_guide: Mutex::new(None),
        shown: true,
        mock: false,
        presenter: "jxa",
    })
}

#[cfg(windows)]
fn windows_powershell() -> PathBuf {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    PathBuf::from(root).join(r"System32\WindowsPowerShell\v1.0\powershell.exe")
}

#[cfg(windows)]
fn spawn_winforms_hud() -> VcuResult<StageHandle> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let token = ulid::Ulid::new().to_string();
    let control_path = std::env::temp_dir().join(format!("vcu-stage-{token}.json"));
    write_control(&control_path, false, None, Some(true))?;
    let script_path = std::env::temp_dir().join(format!("vcu-stage-{token}.ps1"));
    {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(STAGE_WINPS.as_bytes());
        std::fs::write(&script_path, bytes).map_err(|e| {
            VcuError::with_detail(ErrorCode::Internal, "write windows stage script", e.to_string())
        })?;
    }
    let log_path = std::env::temp_dir().join(format!("vcu-stage-{token}.log"));
    let log = std::fs::File::create(&log_path).map_err(|e| {
        VcuError::with_detail(ErrorCode::Internal, "windows stage log", e.to_string())
    })?;
    let mut cmd = Command::new(windows_powershell());
    cmd.args([
        "-NoProfile",
        "-STA",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        script_path.to_string_lossy().as_ref(),
        control_path.to_string_lossy().as_ref(),
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(std::process::Stdio::from(log))
    .creation_flags(CREATE_NO_WINDOW);
    let mut child = cmd.spawn().map_err(|e| {
        let _ = std::fs::remove_file(&script_path);
        let _ = std::fs::remove_file(&control_path);
        VcuError::with_detail(
            ErrorCode::StageRequired,
            "failed to raise Windows Stage HUD",
            e.to_string(),
        )
    })?;
    std::thread::sleep(std::time::Duration::from_millis(600));
    match child.try_wait() {
        Ok(None) => {}
        Ok(Some(status)) => {
            let err = std::fs::read_to_string(&log_path).unwrap_or_default();
            let _ = std::fs::remove_file(&script_path);
            let _ = std::fs::remove_file(&control_path);
            return Err(VcuError::with_detail(
                ErrorCode::StageRequired,
                "Windows Stage HUD exited",
                format!("status={status:?} log={err}"),
            ));
        }
        Err(e) => {
            return Err(VcuError::with_detail(
                ErrorCode::StageRequired,
                "Windows Stage HUD wait failed",
                e.to_string(),
            ));
        }
    }
    Ok(StageHandle {
        child: Mutex::new(Some(child)),
        script_path: Some(script_path),
        control_path: Some(control_path.clone()),
        abort_path: Some(control_path.with_extension("abort")),
        last_guide: Mutex::new(None),
        shown: true,
        mock: false,
        presenter: "winforms",
    })
}

fn write_control(
    path: &PathBuf,
    stop: bool,
    guide: Option<GuidePos>,
    hud: Option<bool>,
) -> VcuResult<()> {
    let mut body = match guide {
        Some(g) => serde_json::json!({
            "stop": stop,
            "guide": {"x": g.x, "y": g.y, "visible": g.visible}
        }),
        None => serde_json::json!({"stop": stop}),
    };
    if let Some(h) = hud {
        body["hud"] = serde_json::json!(h);
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(&body).unwrap_or_default()).map_err(|e| {
        VcuError::with_detail(ErrorCode::Internal, "write stage control", e.to_string())
    })?;
    std::fs::rename(&tmp, path).map_err(|e| {
        VcuError::with_detail(ErrorCode::Internal, "publish stage control", e.to_string())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_stage_is_not_shown() {
        let s = StageHandle::hidden();
        assert!(!s.shown);
        assert_eq!(s.presenter, "hidden");
    }

    #[test]
    fn noop_stage_is_shown_without_process() {
        let mut s = StageHandle::noop();
        assert!(s.shown);
        assert!(s.mock);
        assert_eq!(s.presenter, "noop");
        let g = s.move_guide(60.0, 32.0).unwrap();
        assert_eq!(g.x, 60.0);
        assert_eq!(s.last_guide().unwrap().y, 32.0);
        s.teardown();
        assert!(s.shown);
        assert!(banner_text().contains("VCU 正在使用这台 Mac"));
    }

    #[test]
    fn abort_signal_is_detected_and_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let control = dir.path().join("stage.json");
        let abort = dir.path().join("stage.abort");
        std::fs::write(&control, b"{\"stop\":false}").unwrap();
        let mut s = StageHandle {
            child: Mutex::new(None),
            script_path: None,
            control_path: Some(control),
            abort_path: Some(abort.clone()),
            last_guide: Mutex::new(None),
            shown: true,
            mock: true,
            presenter: "noop",
        };
        assert!(!s.abort_requested());
        s.write_abort_signal().unwrap();
        assert!(abort.is_file());
        assert!(s.abort_requested());
        s.teardown();
        assert!(!abort.exists());
    }

    #[test]
    fn write_control_can_show_hud() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("c.json");
        write_control(&p, false, None, Some(true)).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
        assert_eq!(v["hud"], true);
        assert_eq!(v["stop"], false);
    }

    #[test]
    fn write_control_can_hide_hud() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("c.json");
        write_control(
            &p,
            false,
            Some(GuidePos {
                x: 1732.0,
                y: 195.0,
                visible: true,
            }),
            Some(false),
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
        assert_eq!(v["hud"], false);
        assert_eq!(v["guide"]["x"], 1732.0);
        assert_eq!(v["stop"], false);
    }

    #[test]
    fn jxa_fallback_is_capsule_not_full_width() {
        assert!(STAGE_JXA.contains("const width = 280;"));
        assert!(STAGE_JXA.contains("c.hud === false"));
        assert!(STAGE_JXA.contains("Esc 取消"));
        assert!(STAGE_JXA.contains("keyCode === 53"));
        assert!(STAGE_JXA.contains(".abort"));
        assert!(STAGE_JXA.contains("vis.origin"));
        assert!(!STAGE_JXA.contains("NSScreen.screens"));
        assert!(!STAGE_JXA.contains("screen.size.width;"));
        assert!(!STAGE_JXA.contains("screen.origin"));
    }

    #[test]
    fn hud_copy_is_vcu_not_chatgpt_or_codex() {
        assert!(BANNER.contains("VCU 正在使用这台 Mac"));
        assert!(BANNER.contains("Escape") || BANNER.contains("Esc"));
        for banned in ["ChatGPT", "Using your Mac", "Codex is using"] {
            assert!(!BANNER.contains(banned), "{BANNER}");
            assert!(!STAGE_JXA.contains(banned), "jxa");
        }
        assert!(STAGE_JXA.contains("VCU 正在使用这台 Mac"));
        assert!(STAGE_JXA.contains("Esc 取消"));
        assert!(BANNER_WINDOWS.contains("VCU 正在使用这台 PC"));
        assert!(BANNER_WINDOWS.contains("Escape") || BANNER_WINDOWS.contains("Esc"));
        assert!(STAGE_WINPS.contains("VCU 正在使用这台 PC"));
        assert!(STAGE_WINPS.contains("Esc 取消"));
        assert!(STAGE_WINPS.contains("Keys]::Escape"));
        assert!(STAGE_WINPS.contains("abort"));
        assert!(STAGE_WINPS.contains("TopMost"));
        assert!(!STAGE_WINPS.to_ascii_lowercase().contains("sendinput("));
        assert!(!STAGE_WINPS.to_ascii_lowercase().contains("[system.windows.forms.sendkeys"));
        for banned in ["ChatGPT", "Using your Mac", "Codex is using"] {
            assert!(!BANNER_WINDOWS.contains(banned), "{BANNER_WINDOWS}");
            assert!(!STAGE_WINPS.contains(banned), "winps");
        }
        let swift = include_str!("../../../helpers/vcu-stage/main.swift");
        assert!(swift.contains(r#"let hudTitle = "VCU 正在使用这台 Mac""#));
        assert!(swift.contains(r#"let hudSub = "Esc 取消""#));
        assert!(!swift.contains(r#"let hudTitle = "ChatGPT"#));
        assert!(!swift.contains("Codex is using"));
        assert!(swift.contains("Compact dart, no hard ring, no long stem"));
        assert!(swift.contains("fogCenter"));
    }

    #[test]
    fn guide_overlay_is_short_dart_with_fog() {
        let swift = include_str!("../../../helpers/vcu-stage/main.swift");
        assert!(swift.contains("Compact dart, no hard ring, no long stem"));
        assert!(swift.contains("fogCenter"));
        assert!(swift.contains("endRadius: 36"));
    }

    #[test]
    fn resolve_stage_bin_prefers_explicit_file() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("vcu-stage");
        std::fs::write(&bin, b"#!/bin/sh\n").unwrap();
        let got = resolve_stage_bin_from(Some(bin.clone()), None, None);
        assert_eq!(got.as_deref(), Some(bin.as_path()));
        let empty = tempfile::tempdir().unwrap();
        let missing = resolve_stage_bin_from(
            Some(empty.path().join("nope")),
            Some(empty.path().join("vcu-daemon")),
            None,
        );
        assert!(missing.is_none());
        let sibling_dir = dir.path().join("bin");
        std::fs::create_dir(&sibling_dir).unwrap();
        let sibling = sibling_dir.join("vcu-stage");
        std::fs::write(&sibling, b"x").unwrap();
        let exe = sibling_dir.join("vcu-daemon");
        std::fs::write(&exe, b"x").unwrap();
        let got = resolve_stage_bin_from(None, Some(exe), None);
        assert_eq!(got.as_deref(), Some(sibling.as_path()));
    }
}
