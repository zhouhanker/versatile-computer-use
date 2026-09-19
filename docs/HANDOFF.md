# VCU 会话交接

更新时间：2026-09-19。作者zhouhanker。

**先读 [当前计划](PLAN.md)，再读本文。** 当前定版是浏览器 Bridge **0.2.7**。总体目标未宣布全部完成。

## 1. 用户本轮要求

确定当前测试还剩什么问题、Computer Use 边界在哪，更新 README，推送到 GitHub。

约束未变：browser-only；不要再向用户要截图；不要退回长箭尾/硬圆环；有 mismatch 才改代码；不改 `~/.codex/computer-use/`；不点 Allow；不 warp OS 光标；不碰用户 Edge 组 `1` / `3`。

## 2. 当前工程状态

| 项目 | 状态 |
| --- | --- |
| Git | main，上一发布 `11fc9a3`（0.2.6）；本轮 0.2.7 待提交推送 |
| Bridge | **0.2.7**（仓库）；本机已装 lens 仍可能 ping **0.2.5**，需 install-lens + Reload |
| runtime package | **0.1.0** |
| 自动化 | Node 扩展测试 40 通过；`extension_bridge` 含双客户端合并与 tab 所有者路由 |
| daemon | 本机在跑；不要用旧 PID 当证据 |

## 3. 0.2.7 相对 0.2.6 的实现

- 光标：移动 >8px 朝向旋转，点击 squash + halo pulse；截图 `keep_cursor=true`
- `GET /v1/browser/tabs` 合并多个 polling 扩展；tab 带 `browser`
- 已知且不冲突的 `tab_id` 路由到所属 client
- CLI/MCP/HTTP `hover`，合成 mouseover，`trusted=false`
- 同源 iframe 点内层；canvas 合成 pointer 序列；跨源 iframe / object 仍拒绝

## 4. 边界（不要写成已完成）

- 不是 Codex 官方桌面 CU，不是 OS 键鼠
- TC-B-040 通用 AX 网页像素真点：未通过；禁止用 warp/CDP 假绿
- 跨源 iframe、trusted 手势、支付/游戏类 canvas：不在范围内
- 双浏览器 `tab_id` 可能撞号；撞号不保证路由
- 双浏览器合并目前是桥接单测，不是本机 Edge+Chrome 真机复检
- DOM 一律 `source=extension_dom`，`trusted=false`

## 5. 验证

```sh
node --test extension/tests/*.test.cjs
cargo test --workspace --offline
vcu browser ping --json    # version 应与仓库一致；落后则 Reload 扩展
```

真机更多场景：`python3 scripts/poc_browser_more_scenarios.py`（独立窗口，不碰用户 1/3 组）。本轮未重跑该脚本，因为本机 lens 仍是 0.2.5。

## 6. 下一阶段

1. 本机 Reload 0.2.7 lens，确认 ping 版本。
2. 需要时再跑 more-scenarios / 双浏览器真机 tabs。
3. 不要把 AX 网页真点或跨源 iframe 排进下一步，除非产品范围改变。

历史 0.2.5 节点证据仍以 `d8ee9ad` 和 [BROWSER_PARITY_RESULTS](testing/BROWSER_PARITY_RESULTS.md) 为准。
