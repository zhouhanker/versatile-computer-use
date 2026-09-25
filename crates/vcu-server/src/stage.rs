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
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class VcuDpiBoot {
  [DllImport("shcore.dll")] public static extern int SetProcessDpiAwareness(int awareness);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
  [DllImport("user32.dll")] public static extern uint GetDpiForSystem();
}
"@
$script:DpiAware = $false
try { $script:DpiAware = [VcuDpiBoot]::SetProcessDpiAwarenessContext([IntPtr](-4)) } catch {}
if (-not $script:DpiAware) {
  try { if ([VcuDpiBoot]::SetProcessDpiAwareness(2) -eq 0) { $script:DpiAware = $true } } catch {}
}
if (-not $script:DpiAware) { try { $script:DpiAware = [VcuDpiBoot]::SetProcessDPIAware() } catch {} }
$script:Dpi = 96
try { $script:Dpi = [int][VcuDpiBoot]::GetDpiForSystem() } catch {}
if ($script:Dpi -lt 96) { $script:Dpi = 96 }
$script:DpiScale = $script:Dpi / 96.0
Add-Type -AssemblyName System.Windows.Forms | Out-Null
Add-Type -AssemblyName System.Drawing | Out-Null
Add-Type -TypeDefinition @"
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
public static class VcuStageWin {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x; public int y; }
  [StructLayout(LayoutKind.Sequential)] public struct SIZE { public int cx; public int cy; }
  [StructLayout(LayoutKind.Sequential, Pack=1)]
  public struct BLENDFUNCTION { public byte BlendOp; public byte BlendFlags; public byte SourceConstantAlpha; public byte AlphaFormat; }
  [StructLayout(LayoutKind.Sequential)] public struct BITMAPINFOHEADER {
    public int biSize; public int biWidth; public int biHeight; public short biPlanes; public short biBitCount;
    public int biCompression; public int biSizeImage; public int biXPelsPerMeter; public int biYPelsPerMeter;
    public int biClrUsed; public int biClrImportant;
  }
  [StructLayout(LayoutKind.Sequential)] public struct BITMAPINFO { public BITMAPINFOHEADER bmiHeader; }
  [DllImport("user32.dll", SetLastError=true)] public static extern bool UpdateLayeredWindow(IntPtr hwnd, IntPtr hdcDst, ref POINT pptDst, ref SIZE psize, IntPtr hdcSrc, ref POINT pptSrc, int crKey, ref BLENDFUNCTION pblend, int dwFlags);
  [DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr hwnd, IntPtr hdc);
  [DllImport("gdi32.dll")] public static extern IntPtr CreateCompatibleDC(IntPtr hdc);
  [DllImport("gdi32.dll")] public static extern bool DeleteDC(IntPtr hdc);
  [DllImport("gdi32.dll")] public static extern IntPtr SelectObject(IntPtr hdc, IntPtr obj);
  [DllImport("gdi32.dll")] public static extern bool DeleteObject(IntPtr obj);
  [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr hWnd, int nIndex);
  [DllImport("user32.dll")] public static extern int SetWindowLong(IntPtr hWnd, int nIndex, int dwNewLong);
  [DllImport("gdi32.dll")] public static extern IntPtr CreateDIBSection(IntPtr hdc, ref BITMAPINFO bmi, uint usage, out IntPtr bits, IntPtr section, uint offset);
  public static bool ShowBitmap(IntPtr hwnd, Bitmap bitmap, int x, int y, bool clickThrough) {
    int ex = GetWindowLong(hwnd, -20);
    int style = ex | 0x80000 | 0x08000000;
    if (clickThrough) style |= 0x20;
    SetWindowLong(hwnd, -20, style);
    IntPtr screenDc = GetDC(IntPtr.Zero);
    IntPtr memDc = CreateCompatibleDC(screenDc);
    BITMAPINFO bmi = new BITMAPINFO();
    bmi.bmiHeader.biSize = 40;
    bmi.bmiHeader.biWidth = bitmap.Width;
    bmi.bmiHeader.biHeight = -bitmap.Height;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    IntPtr bits;
    IntPtr dib = CreateDIBSection(screenDc, ref bmi, 0, out bits, IntPtr.Zero, 0);
    BitmapData data = bitmap.LockBits(new Rectangle(0, 0, bitmap.Width, bitmap.Height), ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
    int bytes = Math.Abs(data.Stride) * bitmap.Height;
    byte[] raw = new byte[bytes];
    Marshal.Copy(data.Scan0, raw, 0, bytes);
    bitmap.UnlockBits(data);
    for (int i = 0; i < raw.Length; i += 4) {
      byte a = raw[i + 3];
      raw[i] = (byte)(raw[i] * a / 255);
      raw[i + 1] = (byte)(raw[i + 1] * a / 255);
      raw[i + 2] = (byte)(raw[i + 2] * a / 255);
    }
    Marshal.Copy(raw, 0, bits, raw.Length);
    IntPtr old = SelectObject(memDc, dib);
    SIZE size = new SIZE(); size.cx = bitmap.Width; size.cy = bitmap.Height;
    POINT dst = new POINT(); dst.x = x; dst.y = y;
    POINT src = new POINT();
    BLENDFUNCTION blend = new BLENDFUNCTION();
    blend.BlendOp = 0; blend.SourceConstantAlpha = 255; blend.AlphaFormat = 1;
    bool ok = UpdateLayeredWindow(hwnd, screenDc, ref dst, ref size, memDc, ref src, 0, ref blend, 2);
    SelectObject(memDc, old);
    DeleteObject(dib);
    DeleteDC(memDc);
    ReleaseDC(IntPtr.Zero, screenDc);
    return ok;
  }
}
"@ -ReferencedAssemblies System.Drawing -Language CSharp

$script:HudW = 280
$script:HudH = 28
$script:GuideW = 84
$script:GuideH = 84
$script:HotX = 32
$script:HotY = 34
$script:HudW = [int][Math]::Round(280 * $script:DpiScale)
$script:HudH = [int][Math]::Round(28 * $script:DpiScale)
$script:GuideW = [int][Math]::Round(84 * $script:DpiScale)
$script:GuideH = $script:GuideW
$script:HotX = [int][Math]::Round(32 * $script:DpiScale)
$script:HotY = [int][Math]::Round(34 * $script:DpiScale)

function New-HudBitmap {
  $bmp = New-Object System.Drawing.Bitmap $script:HudW, $script:HudH, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
  $g.Clear([System.Drawing.Color]::Transparent)
  $path = New-Object System.Drawing.Drawing2D.GraphicsPath
  $d = $script:HudH
  $path.AddArc(0, 0, $d, $d, 180, 90)
  $path.AddArc(($script:HudW - $d), 0, $d, $d, 270, 90)
  $path.AddArc(($script:HudW - $d), 0, $d, $d, 0, 90)
  $path.AddArc(0, 0, $d, $d, 90, 90)
  $path.CloseFigure()
  $fill = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(245, 18, 46, 107))
  $g.FillPath($fill, $path)
  $pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(115, 255, 255, 255)), 1
  $g.DrawPath($pen, $path)
  $fontPx = [single](12 * $script:DpiScale)
  $font = New-Object System.Drawing.Font "Segoe UI", $fontPx, ([System.Drawing.FontStyle]::Regular), ([System.Drawing.GraphicsUnit]::Pixel)
  $textY = [single](($script:HudH - $fontPx) / 2)
  $pad = 12 * $script:DpiScale
  $white = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::White)
  $dim = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(210, 255, 255, 255))
  $g.DrawString("VCU 正在使用这台 PC", $font, $white, $pad, $textY)
  $measured = $g.MeasureString("Esc 取消", $font)
  $g.DrawString("Esc 取消", $font, $dim, ($script:HudW - $measured.Width - $pad), $textY)
  $g.Dispose()
  return $bmp
}

function New-GuideBitmap {
  # PARITY-004: Compact dart, no hard ring, no long stem.
  # Hotspot is the arrow tip, matching helpers/vcu-stage GuideView.
  $bmp = New-Object System.Drawing.Bitmap $script:GuideW, $script:GuideH, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $g.Clear([System.Drawing.Color]::Transparent)
  $tipX = $script:HotX
  $tipY = $script:HotY
  $fogCenterX = $tipX + 6
  $fogCenterY = $tipY + 6
  # Soft fog, endRadius 36, no hard ring.
  $fogMax = [int][Math]::Round(36 * $script:DpiScale)
  for ($r = $fogMax; $r -ge 2; $r -= 2) {
    $alpha = [int](6 + (36 - ($r / $script:DpiScale)) * 2.4)
    if ($alpha -gt 96) { $alpha = 96 }
    $haze = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb($alpha, 168, 182, 196))
    $g.FillEllipse($haze, ($fogCenterX - $r), ($fogCenterY - $r), ($r * 2), ($r * 2))
    $haze.Dispose()
  }
  $pts = @(
    (New-Object System.Drawing.Point $tipX, $tipY),
    (New-Object System.Drawing.Point ($tipX + [int](18 * $script:DpiScale)), ($tipY + [int](10 * $script:DpiScale))),
    (New-Object System.Drawing.Point ($tipX + [int](10 * $script:DpiScale)), ($tipY + [int](12 * $script:DpiScale))),
    (New-Object System.Drawing.Point ($tipX + [int](4 * $script:DpiScale)), ($tipY + [int](20 * $script:DpiScale)))
  )
  $fill = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(245, 90, 96, 104))
  $g.FillPolygon($fill, $pts)
  $edge = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(235, 255, 255, 255)), 1.5
  $edge.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
  $g.DrawPolygon($edge, $pts)
  $g.Dispose()
  return $bmp
}

if ($env:VCU_STAGE_RENDER) {
  $dir = $env:VCU_STAGE_RENDER
  New-Item -ItemType Directory -Force -Path $dir | Out-Null
  $hud = New-HudBitmap
  $guide = New-GuideBitmap
  $hud.Save((Join-Path $dir "hud.png"), [System.Drawing.Imaging.ImageFormat]::Png)
  $guide.Save((Join-Path $dir "guide.png"), [System.Drawing.Imaging.ImageFormat]::Png)
  "RENDER_OK $($script:HudW)x$($script:HudH) guide=$($script:GuideW) hotspot=$($script:HotX),$($script:HotY) fogCenter endRadius: 36" | Set-Content -Encoding utf8 (Join-Path $dir "render.txt")
  exit 0
}

$controlPath = $args[0]
if (-not $controlPath) { exit 1 }
$abortPath = [System.IO.Path]::ChangeExtension($controlPath, "abort")
function Read-Control {
  if (-not (Test-Path -LiteralPath $controlPath)) { return $null }
  try { return (Get-Content -LiteralPath $controlPath -Raw -ErrorAction Stop | ConvertFrom-Json) } catch { return $null }
}
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
$hudBmp = New-HudBitmap
$guideBmp = New-GuideBitmap
$hudLeft = [int]($screen.Left + ($screen.Width - $script:HudW) / 2)
$hudTop = [int]($screen.Top + 8)
$hud = New-Object System.Windows.Forms.Form
$hud.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None
$hud.ShowInTaskbar = $false
$hud.TopMost = $true
$hud.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual
$hud.ClientSize = New-Object System.Drawing.Size $script:HudW, $script:HudH
$hud.Left = $hudLeft
$hud.Top = $hudTop
$hud.KeyPreview = $true
$hud.Add_KeyDown({ if ($_.KeyCode -eq [System.Windows.Forms.Keys]::Escape) { [System.IO.File]::WriteAllText($abortPath, "1") } })
$null = $hud.Handle
[void][VcuStageWin]::ShowBitmap($hud.Handle, $hudBmp, $hudLeft, $hudTop, $false)
$guide = New-Object System.Windows.Forms.Form
$guide.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None
$guide.ShowInTaskbar = $false
$guide.TopMost = $true
$guide.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual
$guide.ClientSize = New-Object System.Drawing.Size $script:GuideW, $script:GuideH
$guide.Visible = $false
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 80
$timer.Add_Tick({
  $c = Read-Control
  if ($null -eq $c) { return }
  if ($c.stop) { $timer.Stop(); $hud.Close(); return }
  if ($c.PSObject.Properties.Name -contains "hud") {
    if ($c.hud -eq $false) { $hud.Hide() } else {
      if (-not $hud.Visible) { $hud.Show() }
      [void][VcuStageWin]::ShowBitmap($hud.Handle, $hudBmp, $hud.Left, $hud.Top, $false)
    }
  }
  if ($c.guide) {
    $gx = 0; $gy = 0
    try { $gx = [int]$c.guide.x } catch {}
    try { $gy = [int]$c.guide.y } catch {}
    $left = $gx - $script:HotX
    $top = $gy - $script:HotY
    $guide.Left = $left
    $guide.Top = $top
    if ($c.guide.visible -eq $false) { $guide.Hide() } else {
      if (-not $guide.Visible) { $guide.Show() }
      [void][VcuStageWin]::ShowBitmap($guide.Handle, $guideBmp, $left, $top, $true)
    }
  }
})
$hud.Add_Shown({
  [void][VcuStageWin]::ShowBitmap($hud.Handle, $hudBmp, $hud.Left, $hud.Top, $false)
})
$timer.Start()
$hud.Add_FormClosed({ $timer.Stop(); try { $guide.Close() } catch {} })
[System.Windows.Forms.Application]::Run($hud)
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
        assert!(banner_text().contains("VCU 正在使用这台"));
        assert!(banner_text().contains("Mac") || banner_text().contains("PC"));
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
    fn windows_stage_matches_macos_capsule_and_dart() {
        assert!(STAGE_WINPS.contains("$script:HudW = 280"));
        assert!(STAGE_WINPS.contains("$script:HudH = 28"));
        assert!(STAGE_WINPS.contains("$script:GuideW = 84"));
        assert!(STAGE_WINPS.contains("$script:HotX = 32"));
        assert!(STAGE_WINPS.contains("$script:HotY = 34"));
        assert!(STAGE_WINPS.contains("Compact dart, no hard ring, no long stem"));
        assert!(STAGE_WINPS.contains("endRadius 36"));
        assert!(STAGE_WINPS.contains("Keys]::Escape"));
        assert!(STAGE_WINPS.contains("TopMost"));
        assert!(STAGE_WINPS.contains("0x20"));
        assert!(STAGE_WINPS.contains("ShowBitmap"));
        assert!(STAGE_WINPS.contains("UpdateLayeredWindow"));
        assert!(STAGE_WINPS.contains("SetProcessDpiAwareness"));
        assert!(STAGE_WINPS.contains("GetDpiForSystem"));
        assert!(STAGE_WINPS.contains("GraphicsUnit]::Pixel"));
        assert!(!STAGE_WINPS.contains("New-PillRegion"));
        assert!(!STAGE_WINPS.contains("FromArgb(255, 107, 56)"));
        assert!(!STAGE_WINPS.to_ascii_lowercase().contains("sendinput("));
        assert!(!STAGE_WINPS.to_ascii_lowercase().contains("[system.windows.forms.sendkeys"));
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
