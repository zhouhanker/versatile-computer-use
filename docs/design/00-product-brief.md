# VCU 产品简报

版本：0.2-design  
日期：2026-09-18  
状态：**Stage+Steward 主路径待用户确认**；确认前不写 overlay/AX 执行器产品代码。

## 一句话

**Versatile Computer Use (VCU)** 是与 Agent 宿主、模型厂商解耦的本机 Computer Use 运行时。主路径：**Steward 长驻 + 一次辅助功能授权 + 真实窗口上的 Scene/Actuator + Stage/Guide 可见层**。独立空浏览器只是旁路。

## 目标用户

- 使用 Codex / Claude Code / Cursor / Pi 等、但希望 **自选模型** 仍能 Computer Use 的开发者
- 需要 **已登录的真实窗口**（Edge、飞书客户端等），而不是空 profile
- 需要把能力以 **CLI/MCP/Skill** 接到自建 Agent 的团队

## 核心价值主张

1. **解耦**：Computer Use ≠ 某家模型的附赠功能
2. **真窗口**：默认操作用户已打开的 App，带登录态
3. **一次授权**：macOS 辅助功能一次；不再把 CDP Allow 当日常
4. **可看见**：Stage 横幅「VCU 正在使用这台 Mac」+ Guide 虚拟指针；不搬用户物理鼠标
5. **可补视觉**：`vcu init model`；主模型可纯文本，Scene 以 AX 为先
6. **易接入**：CLI / MCP；Steward 对所有宿主相同

## 与 Codex CU 的关系

借鉴其结构（长驻服务、AX、叠加层、虚拟指针、真窗口）。**名称、协议、宿主全部自有。** 禁止改/卸 `~/.codex/computer-use/`。详见 `docs/design/06-stage-steward.md` 与 `docs/research/10-stage-from-codex-cu-lessons.md`。

## 与独立 Agent Edge 的关系

`browser_agent`（空 profile + 扩展）保留：公开页、CI、不想碰用户窗。日常默认改为 `desktop`。

## 成功指标（本设计）

- [ ] Stage+Steward 文档被接受
- [ ] 名称不与 Codex 产品撞车
- [ ] 微信 denylist、不碰 Codex CU 写进边界
- [ ] 用户确认后台账才切开实现
