# VCU 会话交接

更新时间：2026-09-20。作者zhouhanker。

**先读 [PLAN.md](PLAN.md) 与 [ROADMAP-CU.md](ROADMAP-CU.md)。** Bridge 0.2.8 浏览器冻结有效。桌面史诗进行中。

## 本轮完成

- **CU-D-010**（单测）：无可见 Stage 则 `ErrorCode::StageRequired`；`session start --surface desktop` 返回 `stage_hud` / `stage_presenter`；raise 控制文件 `hud: true`。
- **CU-D-011**（单测）：abort 文件删除会话；mock Stage 武装 abort watch。真机 TextEdit Escape 未跑。
- **CU-D-012**（单测）：微信 denylist + covering point。

证据：`cargo test --workspace --offline` 绿；`node --test extension/tests/*.test.cjs` 42 绿。

## 下一刀

CU-D-011 真机 Abort，或 CU-D-013/014 观察路径（窗口截帧 / Scene scale）按 ROADMAP 继续。不要对用户 Edge 组 1/3 做动作。

## 红线

不点 Allow；不自动化微信；不 warp OS 光标；不把 `/reference/` 官方包提交进 git。
