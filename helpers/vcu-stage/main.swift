import AppKit
import Foundation

/// Compact macOS HUD: system hudWindow material, content-sized capsule.
/// Control JSON: { "stop": false, "hud": false, "guide": { "x": 12.0, "y": 34.0, "visible": true } }
/// x/y are AX points (primary top-left, y down).

let hudTitle = "VCU 正在使用这台 Mac"
let hudSub = "Esc 取消"
let hudHeight: CGFloat = 28
// Keep enough transparent margin for the halo while anchoring the arrow tip
// itself to the requested AX point.
let guideSize: CGFloat = 84

struct ControlFile: Decodable {
    var stop: Bool?
    var hud: Bool?
    var guide: GuideSpec?
}

struct GuideSpec: Decodable {
    var x: Double
    var y: Double
    var visible: Bool?
}

final class HudRoot: NSView {
    let effect = NSVisualEffectView()
    let hairline = NSView()

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        layer?.cornerRadius = hudHeight / 2
        layer?.masksToBounds = false
        effect.material = .hudWindow
        effect.blendingMode = .behindWindow
        effect.state = .active
        effect.wantsLayer = true
        effect.layer?.cornerRadius = hudHeight / 2
        effect.layer?.masksToBounds = true
        effect.autoresizingMask = [.width, .height]
        addSubview(effect)
        hairline.wantsLayer = true
        hairline.layer?.cornerRadius = hudHeight / 2
        hairline.layer?.borderWidth = 0.5
        hairline.layer?.borderColor = NSColor.separatorColor.withAlphaComponent(0.45).cgColor
        hairline.autoresizingMask = [.width, .height]
        addSubview(hairline)
    }

    required init?(coder: NSCoder) { fatalError("init(coder:)") }
    override var isOpaque: Bool { false }

    override func layout() {
        super.layout()
        effect.frame = bounds
        hairline.frame = bounds
        let r = bounds.height / 2
        layer?.cornerRadius = r
        effect.layer?.cornerRadius = r
        hairline.layer?.cornerRadius = r
    }
}

final class GuideView: NSView {
    // NSView coordinates grow upward. This point is the arrow hotspot and is
    // intentionally not the center of the guide window.
    static let hotspot = NSPoint(x: 32, y: 50)

    override var isOpaque: Bool { false }

    override func draw(_ dirtyRect: NSRect) {
        let tip = Self.hotspot
        // PARITY-004: match native Codex CU crop and public ~66px circular fog.
        // Compact dart, no hard ring, no long stem.
        let fogCenter = NSPoint(x: tip.x + 6, y: tip.y - 6)
        NSGraphicsContext.saveGraphicsState()
        let fogShadow = NSShadow()
        fogShadow.shadowBlurRadius = 16
        fogShadow.shadowOffset = .zero
        fogShadow.shadowColor = NSColor(calibratedRed: 0.62, green: 0.70, blue: 0.78, alpha: 0.55)
        fogShadow.set()
        NSColor(calibratedRed: 0.70, green: 0.76, blue: 0.82, alpha: 0.22).setFill()
        NSBezierPath(ovalIn: NSRect(x: fogCenter.x - 22, y: fogCenter.y - 22, width: 44, height: 44)).fill()
        NSGraphicsContext.restoreGraphicsState()
        if let ctx = NSGraphicsContext.current?.cgContext {
            let colors = [
                NSColor(calibratedRed: 0.58, green: 0.66, blue: 0.74, alpha: 0.42).cgColor,
                NSColor(calibratedRed: 0.67, green: 0.72, blue: 0.78, alpha: 0.24).cgColor,
                NSColor(calibratedRed: 0.81, green: 0.83, blue: 0.87, alpha: 0.10).cgColor,
                NSColor(calibratedWhite: 0.90, alpha: 0).cgColor,
            ] as CFArray
            let locations: [CGFloat] = [0, 0.40, 0.68, 1]
            if let gradient = CGGradient(
                colorsSpace: CGColorSpaceCreateDeviceRGB(),
                colors: colors,
                locations: locations
            ) {
                ctx.drawRadialGradient(
                    gradient,
                    startCenter: CGPoint(x: fogCenter.x, y: fogCenter.y),
                    startRadius: 0,
                    endCenter: CGPoint(x: fogCenter.x, y: fogCenter.y),
                    endRadius: 36,
                    options: [.drawsAfterEndLocation]
                )
            }
        }
        let arrow = NSBezierPath()
        arrow.move(to: tip)
        arrow.line(to: NSPoint(x: tip.x + 18, y: tip.y - 10))
        arrow.line(to: NSPoint(x: tip.x + 10, y: tip.y - 12))
        arrow.line(to: NSPoint(x: tip.x + 4, y: tip.y - 20))
        arrow.close()
        NSColor(calibratedRed: 0.353, green: 0.376, blue: 0.408, alpha: 0.96).setFill()
        arrow.fill()
        arrow.lineWidth = 1.5
        arrow.lineJoinStyle = .round
        NSColor.white.withAlphaComponent(0.92).setStroke()
        arrow.stroke()
    }
}

final class StageController: NSObject {
    let controlURL: URL
    let abortURL: URL
    var hud: NSPanel!
    var guide: NSWindow!
    var timer: Timer?
    var globalMon: Any?
    var localMon: Any?
    var abortSent = false
    var hudEnabled = true
    var hudSize = NSSize(width: 300, height: hudHeight)

    init(controlPath: String) {
        self.controlURL = URL(fileURLWithPath: controlPath)
        self.abortURL = self.controlURL.deletingPathExtension().appendingPathExtension("abort")
        super.init()
    }

    func start() {
        let app = NSApplication.shared
        app.setActivationPolicy(.accessory)
        ProcessInfo.processInfo.processName = "vcu-stage"
        buildHud()
        buildGuide()
        placeHud(on: NSScreen.main ?? NSScreen.screens[0])
        installEscapeMonitors()
        timer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
            self?.poll()
        }
        if let timer {
            RunLoop.main.add(timer, forMode: .common)
        }
        app.finishLaunching()
        app.run()
    }

    func decorate(_ win: NSWindow) {
        win.level = NSWindow.Level(rawValue: Int(CGWindowLevelForKey(.statusWindow)))
        win.isOpaque = false
        win.hasShadow = true
        win.ignoresMouseEvents = true
        win.backgroundColor = .clear
        win.collectionBehavior = [.canJoinAllSpaces, .stationary, .ignoresCycle, .fullScreenAuxiliary]
        win.isReleasedWhenClosed = false
        win.appearance = NSAppearance(named: .vibrantDark)
    }

    func buildHud() {
        let root = HudRoot(frame: .zero)
        let stack = NSStackView()
        stack.orientation = .horizontal
        stack.alignment = .centerY
        stack.spacing = 6
        stack.edgeInsets = NSEdgeInsets(top: 0, left: 10, bottom: 0, right: 12)
        stack.translatesAutoresizingMaskIntoConstraints = false

        let dot = NSImageView()
        let cfg = NSImage.SymbolConfiguration(pointSize: 7, weight: .bold)
        dot.image = NSImage(systemSymbolName: "circle.fill", accessibilityDescription: nil)?.withSymbolConfiguration(cfg)
        dot.contentTintColor = NSColor.controlAccentColor
        dot.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            dot.widthAnchor.constraint(equalToConstant: 10),
            dot.heightAnchor.constraint(equalToConstant: 10),
        ])

        let title = NSTextField(labelWithString: hudTitle)
        title.textColor = .labelColor
        title.font = NSFont.systemFont(ofSize: 11, weight: .semibold)
        title.lineBreakMode = .byTruncatingTail
        title.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

        let sub = NSTextField(labelWithString: hudSub)
        sub.textColor = .secondaryLabelColor
        sub.font = NSFont.systemFont(ofSize: 11, weight: .regular)
        sub.setContentCompressionResistancePriority(.required, for: .horizontal)

        stack.addArrangedSubview(dot)
        stack.addArrangedSubview(title)
        stack.addArrangedSubview(sub)
        root.effect.addSubview(stack)

        // Apple HIG HUD: keep small, don't obscure content. Size to fitting width, 28pt tall.
        stack.layoutSubtreeIfNeeded()
        let fit = stack.fittingSize
        let w = min(max(ceil(fit.width), 220), 320)
        hudSize = NSSize(width: w, height: hudHeight)
        root.frame = NSRect(origin: .zero, size: hudSize)

        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: root.effect.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: root.effect.trailingAnchor),
            stack.topAnchor.constraint(equalTo: root.effect.topAnchor),
            stack.bottomAnchor.constraint(equalTo: root.effect.bottomAnchor),
        ])

        let win = NSPanel(
            contentRect: root.frame,
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        decorate(win)
        win.becomesKeyOnlyIfNeeded = true
        win.contentView = root
        hud = win
        hud.ignoresMouseEvents = false
        let click = NSClickGestureRecognizer(target: self, action: #selector(hudClicked(_:)))
        root.addGestureRecognizer(click)
        win.orderFrontRegardless()
    }

    func buildGuide() {
        let rect = NSRect(x: 0, y: 0, width: guideSize, height: guideSize)
        let win = NSWindow(contentRect: rect, styleMask: .borderless, backing: .buffered, defer: false)
        decorate(win)
        win.appearance = nil
        win.level = NSWindow.Level(rawValue: Int(CGWindowLevelForKey(.statusWindow)) + 1)
        win.hasShadow = false
        win.acceptsMouseMovedEvents = false
        win.contentView = GuideView(frame: rect)
        win.orderOut(nil)
        guide = win
    }

    func primaryScreen() -> NSScreen {
        NSScreen.screens.first(where: { $0.frame.origin == .zero }) ?? NSScreen.main ?? NSScreen.screens[0]
    }

    func cocoaCenter(axX: CGFloat, axY: CGFloat) -> NSPoint {
        let primary = primaryScreen()
        return NSPoint(x: axX, y: primary.frame.maxY - axY)
    }

    func screenContaining(cocoa: NSPoint) -> NSScreen {
        NSScreen.screens.first(where: { $0.frame.contains(cocoa) }) ?? primaryScreen()
    }

    func placeHud(on screen: NSScreen) {
        let vis = screen.visibleFrame
        let x = vis.midX - hudSize.width / 2
        let y = vis.maxY - hudSize.height - 8
        hud.setFrame(NSRect(x: x, y: y, width: hudSize.width, height: hudSize.height), display: true)
        hud.orderFrontRegardless()
    }

    func applyGuide(x: Double, y: Double, visible: Bool) {
        let center = cocoaCenter(axX: CGFloat(x), axY: CGFloat(y))
        // `center` is the target point, while the arrow hotspot is offset
        // inside the transparent guide window. Centering the window here
        // would place the arrow tip away from the requested point.
        let origin = NSPoint(
            x: center.x - GuideView.hotspot.x,
            y: center.y - GuideView.hotspot.y
        )
        guide.setFrameOrigin(origin)
        if hudEnabled {
            placeHud(on: screenContaining(cocoa: center))
        } else {
            hud.orderOut(nil)
        }
        if visible {
            guide.orderFrontRegardless()
        } else {
            guide.orderOut(nil)
        }
    }

    func installEscapeMonitors() {
        globalMon = NSEvent.addGlobalMonitorForEvents(matching: .keyDown) { [weak self] event in
            if event.keyCode == 53 { self?.requestAbort() }
        }
        localMon = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            if event.keyCode == 53 {
                self?.requestAbort()
                return nil
            }
            return event
        }
    }

    @objc func hudClicked(_ sender: NSClickGestureRecognizer) {
        requestAbort()
    }

    func requestAbort() {
        if abortSent { return }
        abortSent = true
        let tmp = abortURL.appendingPathExtension("tmp")
        try? Data("1".utf8).write(to: tmp)
        let _ = try? FileManager.default.replaceItemAt(abortURL, withItemAt: tmp)
        if !FileManager.default.fileExists(atPath: abortURL.path) {
            try? Data("1".utf8).write(to: abortURL)
        }
        stop()
    }

    func poll() {
        guard let data = try? Data(contentsOf: controlURL) else { return }
        guard let ctl = try? JSONDecoder().decode(ControlFile.self, from: data) else { return }
        if ctl.stop == true {
            stop()
            return
        }
        if let h = ctl.hud {
            hudEnabled = h
            if !h { hud.orderOut(nil) }
        }
        if let g = ctl.guide {
            applyGuide(x: g.x, y: g.y, visible: g.visible ?? true)
        }
    }

    func stop() {
        timer?.invalidate()
        hud.orderOut(nil)
        guide.orderOut(nil)
        NSApp.terminate(nil)
    }
}

func parseControlPath() -> String? {
    let args = CommandLine.arguments
    if let i = args.firstIndex(of: "--control"), i + 1 < args.count {
        return args[i + 1]
    }
    if args.count >= 2, !args[1].hasPrefix("-") {
        return args[1]
    }
    return nil
}

// Read-only CG window list for Scene when System Events has no AX windows
// (Finder folder windows on current macOS). Do not start a HUD.
if CommandLine.arguments.count == 3 && CommandLine.arguments[1] == "--list-windows" {
    guard let pid = Int(CommandLine.arguments[2]),
          let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] else { exit(2) }
    var out: [[String: Any]] = []
    for win in windows {
        guard (win[kCGWindowOwnerPID as String] as? Int) == pid,
              (win[kCGWindowLayer as String] as? Int) == 0,
              let bounds = win[kCGWindowBounds as String] as? [String: Double],
              let wx = bounds["X"], let wy = bounds["Y"], let ww = bounds["Width"], let wh = bounds["Height"],
              ww >= 120, wh >= 80,
              let id = win[kCGWindowNumber as String] as? Int else { continue }
        let title = win[kCGWindowName as String] as? String ?? ""
        out.append(["window_id": id, "title": title, "frame": [wx, wy, ww, wh]])
    }
    let data = try JSONSerialization.data(withJSONObject: out)
    print(String(data: data, encoding: .utf8)!)
    exit(0)
}

// Read-only native window lookup used by the browser screenshot path. Do not
// initialize AppKit windows or a HUD for this command.
if CommandLine.arguments.count == 7 && ["--window-id", "--window-info"].contains(CommandLine.arguments[1]) {
    let args = CommandLine.arguments
    guard let pid = Int(args[2]), let x = Double(args[3]), let y = Double(args[4]),
          let w = Double(args[5]), let h = Double(args[6]),
          let windows = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as? [[String: Any]] else { exit(2) }
    var candidates: [(Int, [Double])] = []
    for win in windows {
        guard (win[kCGWindowOwnerPID as String] as? Int) == pid,
              (win[kCGWindowLayer as String] as? Int) == 0,
              let bounds = win[kCGWindowBounds as String] as? [String: Double],
              let wx = bounds["X"], let wy = bounds["Y"], let ww = bounds["Width"], let wh = bounds["Height"],
              ww >= 200, wh >= 150,
              let id = win[kCGWindowNumber as String] as? Int else { continue }
        candidates.append((id, [wx, wy, ww, wh]))
    }
    let selected = candidates.first { _, frame in
        abs(frame[0]-x) <= 2 && abs(frame[1]-y) <= 2 && abs(frame[2]-w) <= 2 && abs(frame[3]-h) <= 2
    } ?? ((w < 200 || h < 150) ? candidates.first : nil)
    guard let (id, frame) = selected else { exit(1) }
    if args[1] == "--window-id" { print(id) }
    else {
        let data = try JSONSerialization.data(withJSONObject: ["window_id": id, "frame": frame])
        print(String(data: data, encoding: .utf8)!)
    }
    exit(0)
}

guard let path = parseControlPath() else {
    fputs("vcu-stage: pass --control <json-path>\n", stderr)
    exit(2)
}

let controller = StageController(controlPath: path)
controller.start()
