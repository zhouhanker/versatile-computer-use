# VCU 会话交接

更新时间：2026-09-20。作者zhouhanker。

**先读 [当前计划](PLAN.md)。** 已发布 Bridge **0.2.8**（浏览器）。用户已要求对标 Codex CU **含桌面**；执行总览是 [ROADMAP-CU.md](ROADMAP-CU.md)，测试是 [testing/DESKTOP_CU_TEST_PLAN.md](testing/DESKTOP_CU_TEST_PLAN.md)。桌面切片尚未开工。

## 硬约束（桌面史诗也不破）

不点 Allow；不自动化微信；不 warp OS 光标；不修改 `~/.codex/computer-use/`；不碰用户 Edge 组 `1` / `3`。网页细操作仍走 extension_dom。

## 已发布

| 项 | 值 |
| --- | --- |
| Git | `e960014` README；功能 `98997b7` Bridge 0.2.8 |
| 热更新 | `vcu browser install-lens --reload` |
| 默认 open | 现有窗口新标签 |

## 下一刀

CU-D-010：desktop 会话必须升起 Stage HUD，否则拒绝。先单测，不要先对用户真窗口开Actuator。
