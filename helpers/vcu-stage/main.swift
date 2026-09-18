import AppKit
import Foundation

/// Compact macOS HUD: system hudWindow material, content-sized capsule.
/// Control JSON: { "stop": false, "hud": false, "guide": { "x": 12.0, "y": 34.0, "visible": true } }
/// x/y are AX points (primary top-left, y down).

let hudTitle = "VCU 正在使用这台 Mac"
let hudSub = "Esc 取消"
let hudHeight: CGFloat = 28
let guideSize: CGFloat = 40

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
    override var isOpaque: Bool { false }

    override func draw(_ dirtyRect: NSRect) {
        let ring = NSBezierPath(ovalIn: bounds.insetBy(dx: 3.5, dy: 3.5))
        ring.lineWidth = 2
        NSColor.controlAccentColor.withAlphaComponent(0.95).setStroke()
        ring.stroke()
        let inner = NSBezierPath(ovalIn: bounds.insetBy(dx: 14, dy: 14))
        NSColor(calibratedRed: 1.0, green: 0.42, blue: 0.22, alpha: 0.95).setFill()
        inner.fill()
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
        let origin = NSPoint(x: center.x - guideSize / 2, y: center.y - guideSize / 2)
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

guard let path = parseControlPath() else {
    fputs("vcu-stage: pass --control <json-path>\n", stderr)
    exit(2)
}

let controller = StageController(controlPath: path)
controller.start()
