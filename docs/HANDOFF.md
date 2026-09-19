# VCU 会话交接

更新时间：2026-09-19。作者zhouhanker。

**先读 [当前计划](PLAN.md)，再读本文。** 当前定版是浏览器 Bridge **0.2.8**。总体目标未宣布全部完成。

## 1. 本轮做了什么

实现并修复未完成项：

- 已 Load unpacked 的 Edge/Chrome lens 热更新：`vcu browser install-lens --reload`
- ping `--reload` 向每个已连接客户端发 `reload_self`
- 默认 `browser open` 钉在 last-focused USER 窗口的新标签；只有 `--new-window` 才开新窗口
- 本机安装新 CLI/daemon 并 Reload lens 到 0.2.8

硬约束未变：browser-only；不点 Allow；不 warp OS 光标；不改 `~/.codex/computer-use/`；不碰用户 Edge 组 `1` / `3`。

## 2. 状态

| 项目 | 状态 |
| --- | --- |
| Bridge | **0.2.8** |
| runtime package | **0.1.0** |
| 热更新 | `vcu browser install-lens --reload`；已打开旧页仍可能 stale content.js |

## 3. 仍不是完成项

- 跨源 iframe / trusted 手势 / TC-B-040 AX 网页像素
- 双浏览器 `tab_id` 撞号
- 飞书 / 微信 / CDP / OS 光标

## 4. 验证

```sh
node --test extension/tests/*.test.cjs
cargo test --workspace --offline
vcu browser ping --json
vcu browser install-lens --reload
```
