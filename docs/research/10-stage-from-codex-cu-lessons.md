# 10 — 从 Codex CU 结构抽出的教训（VCU 用自己的名字）

日期：2026-09-18  
政策：只读调研；禁止修改或卸载 `~/.codex/computer-use/`。

## 结构（借鉴） vs 名字（禁止照搬）

| Codex CU 上看到的结构 | VCU 采用 | VCU 不采用的名字 |
| --- | --- | --- |
| 本机长驻 helper，宿主只发指令 | **Steward**（`vcu-daemon` 演进） | SkyComputerUseService |
| 辅助功能一次授权 | Steward 启动检查 TCC | — |
| AX + 截图观察 | **Scene** | Skysight |
| 自有 click/type，不是每次 CDP Allow | **Actuator** | — |
| 可见 overlay + 软件指针 | **Stage** Banner + **Guide** | FogCursor / LensSequence / ComputerUseCursor |
| 真窗口（已登录浏览器/IM 客户端） | `surface=desktop` | — |

## 为什么独立 Agent Edge 对不齐这条路

空 `--user-data-dir` 没有用户 Cookie。Codex 能点已登录 Edge / 飞书，是因为它在 **系统 UI** 上工作。VCU 要对齐，就必须把默认 surface 改成 desktop，而不是把空 profile 当主路径。

## 浏览器 WebArea 的诚实限制

Chromium 的网页 AX 经常是一块 WebArea。Scene 在这种窗口上必须带截帧；可选把**用户 Edge 里的扩展**当 DOM lens。这与「再开一个空 Agent Edge」不是一回事。

## CDP Allow 为什么不是 Codex 的日常

Codex 不每步去连 `127.0.0.1:9222`。VCU 的日常也不应该。CDP 留作 CI / 无 UI / doctor 降级。

## 硬边界

- 微信：VCU denylist（即便其它工具允许）
- Codex CU 安装：不动
- 用户物理鼠标：不当主路径；Guide 只画 overlay
