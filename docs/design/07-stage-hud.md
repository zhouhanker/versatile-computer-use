# Stage HUD — 紧凑浮层（STEW-003）

版本：0.1-plan  
日期：2026-09-18  
状态：**已实现（2026-09-18 13:30 CST）**  
权威：本文修正 `06-stage-steward.md` 里把 Banner 做成「整屏顶栏」的实现偏差。  
政策：只借鉴 Codex Computer Use **结构**；禁止抄对方类名、禁止把 `~/.codex/computer-use/` 的 Lens/Cursor PNG 提交进仓库。

## 0. 为什么要单独开这一刀

真机 `desktop` 会话把「VCU 正在使用这台 Mac」画成 **每块屏幕一条 36px 通栏**，挡住菜单栏。这不是 Codex CU 的形态，用户会中断会话，长任务就停。

Codex CU 只读结论（不改对方文件）：

| 对方结构 | 形态 | VCU 对应 |
| --- | --- | --- |
| `RecordAndReplayOverlayPanel` | 可拖的浮动小面板，不是通栏 | Stage **HUD**（胶囊） |
| `usingComputer` / `escToCancel` | 一句主文案 + 取消提示 | 「VCU 正在使用这台 Mac」+「Esc 取消」 |
| `ComputerUseCursor` / SoftwareCursor | 软件指针，不搬 OS 光标 | **Guide** |
| `LensSequence` 48×48×45 帧 | 指针旁一圈透镜动画 | Guide 自绘圆环（不复制 PNG） |

## 1. HUD（必须）

- **一块** borderless 窗口，不是 `NSScreen.screens` 循环铺满。
- 尺寸：内容自适应（fitting width，约 220–320pt），高度 **28pt**（接近菜单栏）。背景用 AppKit `NSVisualEffectView.Material.hudWindow`（[HIG HUD](https://developer.apple.com/design/human-interface-guidelines/macos/windows-and-views/panels/)：keep HUDs small, don't obscure content）。
- 位置：Guide 所在屏（尚无 Guide 时用主屏）**顶部居中**，只占中间一段；左右菜单栏必须仍可用。
- 不抢焦点、`activationPolicy accessory`、默认 `ignoresMouseEvents`（第一刀不做拖拽/停止按钮点击）。
- 会话 `stop` / Drop：**立刻**拆掉 HUD 和 Guide。测试同一回合必须 `session stop`，禁止留下 overlay。

禁止：

- 全宽顶栏
- 每块屏幕一条
- 覆盖整个菜单栏

## 2. Guide（必须）

- 独立 overlay 窗，约 48–64pt。
- 自绘：外环（透镜）+ 内点/指针；VCU 自己的蓝/珊瑚，不引用对方 `Software Cursor.png`。
- 坐标：AX 左上原点，y 向下；**禁止** `CGWarpMouseCursorPosition`。
- click 前移到目标 frame 中心；`detail.guide.overlay=true`。

## 3. 实现落点

1. `helpers/vcu-stage/main.swift` — 原生 AppKit presenter（主路径）
2. `crates/vcu-server/src/stage.rs` — JXA 回退改成同样的单屏胶囊，禁止再画通栏
3. `scripts/build-stage.sh` → `~/.local/bin/vcu-stage`
4. 不 `vcu service install` KeepAlive

## 4. 验收

- `cargo test --workspace` 通过
- 真机 `session start --surface desktop` 后 `pgrep vcu-stage` 活着
- 截屏：胶囊在顶部中间，**不是**通栏
- Finder 安全 click（如「最近使用」）Guide 坐标匹配 AX 中心
- WeChat → `AppDenied`；`os_click` → `OsCursorDenied`
- `session stop` 后 helper 消失；本回合不留会话

## 5. 本刀明确不做

- 不复制 Lens PNG / SoftwareCursor / AppInstructions
- 不接对方 socket / XPC
- 不把 Esc 接到 Steward abort（文案先写上，接线下一刀）
- 不自动化微信；不改 Codex CU 安装
- 不在本刀做飞书发送 / Etherscan L3

## 6. 防停滞（写进计划，不只是口头）

长任务停掉的原因已经定位：一轮 abort 后没有可续跑交接，且 overlay 挡屏幕会迫使中断。

每刀结束前必须：

1. 更新 `docs/HANDOFF.md` 第 0 节（时间、pid、下一步一句）
2. 更新台账对应 `STEW-*`
3. 若开过 desktop 会话：先 `vcu session stop all`，再结束回合
