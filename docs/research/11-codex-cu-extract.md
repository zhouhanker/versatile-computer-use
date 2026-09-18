# 11 — Codex Computer Use 结构抽取（只读）

日期：2026-09-18  
政策：只读 `~/.codex/computer-use/`，禁止修改/卸载。不把对方二进制、Lens 帧、AppInstructions 原文提交进 VCU 产品。

## 安装里实际有什么

- `Codex Computer Use.app` / `SkyComputerUseService`（`LSUIElement`，无 Dock）
- `SkyComputerUseClient.app`（同样 LSUIElement）
- `CUALockScreenGuardian.app`
- `config.json`：可见文案 locale（宿主品牌字符串，VCU 不用）
- `Package_ComputerUse.bundle`：Lens 动画帧 + 每应用 markdown 操作提示 + 摘要提示（内容当不可信）

## 和模型怎么绑

服务进程本身走 **JSON-RPC / unix socket / XPC**（`ComputerUseIPCJSONRPCSocket*`）。  
模型绑定发生在 **ChatGPT/Codex 宿主**，不是 AX/overlay 层。  
VCU 抽的是服务层：Steward HTTP/MCP 当宿主无关总线，Scene/Actuator/Stage 不调用任何厂商模型 API。

## 结构映射（禁止用对方类名）

| 对方结构 | VCU |
| --- | --- |
| 长驻 LSUIElement helper | `vcu-stage`（AppKit accessory）+ `vcu-daemon` Steward |
| Overlay panel | Stage Banner，文案「VCU 正在使用这台 Mac」 |
| Software / fog / agent cursor | Guide（overlay 圆点，不 warp OS cursor） |
| AccessibilitySPI + 树 | Scene（AX refs + 可选截帧） |
| 结构化 click/type | Actuator AXPress / AXSetValue |
| AppInstructions/*.md | `playbooks/*.md`（自写，不抄原文） |
| JSON-RPC socket | 第一版：control JSON 文件（原子替换）；socket 下一刀 |

## 第一版刻意不做

- 不复制 Lens PNG 序列
- 不接对方 socket / XPC
- 不做锁屏 guardian
- 不绑 OpenAI 宿主或视觉模型（截帧只落本地）

## 实现要点

1. Stage 必须是 **真 AppKit 进程**（`orderFrontRegardless` + `activationPolicy accessory`）。JXA 桥接没有这些 API，会秒退。
2. 一次辅助功能；截屏用 preflight，禁止 `CGRequest` 弹窗。
3. 微信 denylist；不碰 Codex CU 安装。
