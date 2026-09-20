# VCU 会话交接

更新：2026-09-20。作者 zhouhanker。  
代码：`main` @ `a21df6c`（`feat: CU-D-590 observe --tab live viewport click`）。

本文是会话连续性说明，**不是**计划本身。计划以 [`PLAN.md`](PLAN.md)、[`ROADMAP-CU.md`](ROADMAP-CU.md) 为准。

## 当前任务

**已完成并提交：CU-D-590。**

`vcu browser observe --tab <id>` 指定 USER 标签：后台标签只 `tabs.update(active)`，不 `windows.update(focused)`，不抢 OS 前台。observe 写入 `capture_id` 与 viewport sidecar；对该 capture 做 **live** viewport click，抛页 `#hit` 计数 0→1 后关闭该标签。

| 项 | 结果 |
| --- | --- |
| 真机 | `scripts/poc_cu_d_590.py` → `CU-D-590 OK` tab `1934572749`，0→1 |
| 证据 | `.local/desktop-cu/cu-d-590.json`（不入库） |
| HTTP | `snapshot_merges_extension_tabs_for_empty_ax_edge` 对 `tab_id=99` observe + dry-run click |
| 扩展 | `select_tab` + `focus_window:false` 不改 `focusedWindowId` |
| 门禁 | `make check` 0 |
| 组 1 / 3 | 未动 |
| 宣称 | 不是 TC-B-040；不是完整 Codex CU；无 SendInput |

## 测试边界

### 允许的真机

- USER Chrome / Edge + 已加载 lens（`~/.vcu/lens-extension`）
- 脚本自己打开的 `127.0.0.1` 抛页；用完必须 `close --tab`
- dry-run 先过，再 live
- macOS 允许名单上的一次性窗口：TextEdit / Notes / 脚本自建 Finder 文件夹 / Terminal（无 Return）
- Windows 仅 CI 真机切片（Notepad / Explorer / cmd 等），不是本机产品 `vcu session`

### 禁止的真机

- 用户 Edge 标签组标题 `1` / `3`
- 用户正在看的站点（B 站、飞书网页等）
- 微信窗口上的任何动作
- 点击 Edge「允许调试」/ CDP
- warp 系统光标、HID、SendInput 当主路径
- 飞书客户端自动发送
- 修改 `~/.codex/computer-use/` 或把 `/reference/computer-use/` 提交进 git

### 网页路径（产品）

- 网页动作：`source=extension_dom`，`trusted=false`
- 标签管理：`source=extension_tabs`
- 网页截图：`source=extension_viewport`
- 登录态循环：`ping` → `observe`（PNG + `tab_id` + `capture_id`）→ 60s last observe 绑定 click/type/hover/scroll/extract/screenshot/open
- 指定 `--tab` 的 observe 不依赖 OS 前台是浏览器
- capture 60s 过期；真实动作消费一次；失败后先重新 observe，不盲着重放

### 明确未覆盖（不得写成已通过）

| 项 | 说明 |
| --- | --- |
| TC-B-040 | 通用 AX 网页像素真点。网页真点走 extension viewport |
| 跨源 iframe / `isTrusted` | 明确拒绝；不能替代原生用户手势 |
| 产品 Windows `vcu session` | 只有 CI 切片，没有产品桌面会话 |
| FEISHU-001 | 飞书 App 真发送，停放 |
| MAC-NEXT | 深 AX + GitHub Releases 托管，停放 |
| Ghostty | CU-D-041 只做了 Terminal.app |
| 完整 Codex CU | 无微信、无 HID、无官方动画、无可信手势 |

### 门禁命令

每个切片结束：

```sh
make check                          # 打正在跑的 daemon
node --test extension/tests/*.test.cjs
```

改 daemon 后先重建并重启：

```sh
cargo build -p vcu-cli -p vcu-daemon
./target/debug/vcu daemon stop
./target/debug/vcu daemon start
./target/debug/vcu browser install-lens --reload   # 仅扩展有改动时
```

使用 `target/debug/vcu`，不要用可能过期的 `~/.local/bin/vcu`。

## 本机环境

- daemon：`http://127.0.0.1:17890`，user-dir `~/.vcu`
- lens：`~/.vcu/lens-extension`，Chrome/Edge unpacked 共用 runtime id `cedlbclnijpladccmmfpihhgkeeldfhc`
- client key：`browser:id`；poll 必须带 `browser=`
- Browser Bridge **0.2.8 冻结**：扩展协议与 Node 测试保持绿
- `/reference/computer-use/` 已 gitignore，可本地反编译对照；`.app` / 官方素材不入库
- `dist/install.sh`、`dist/install.ps1` 可能被 `pack-release` 改脏，不要当产品提交

## 未来任务

同一时间只 claim **一个** CU-D 主切片。不要 claim 完整 Codex CU、MAC-NEXT、FEISHU-001、TC-B-040、产品 Windows CU。

| ID | 内容 | 优先级 |
| --- | --- | --- |
| **CU-D-600** | 无 `--tab` 的 observe：前台不是 USER Chrome/Edge 时诚实失败（或要求 `--tab`），禁止静默绑到另一浏览器的 active tab | **下一刀** |
| CU-D-610 左右 | 同一 observe+capture 环上的 live type / scroll（仍只用抛页） | 其后 |
| 双浏览器定向 | 前台非浏览器时的错误码与文案与 CLI/MCP help 对齐 | 可并入 600 |
| Ghostty | Terminal 兄弟应用；禁止执行命令、禁止 Return | 可选 |
| HUD 真机目视 | Stage 胶囊在用户屏上的目视；会升起横幅 | 需明确同意 |
| 产品 Windows session | 真 `vcu session --surface desktop` | 停放 |
| P2 | TC-B-040、跨源 iframe、`isTrusted` | 停放 |

## 下一步

领取 **CU-D-600**：

1. 把切片写进 `ROADMAP-CU.md` / `PLAN.md` / `DESKTOP_CU_TEST_PLAN.md`（立项）
2. 无 `--tab` 且前台不是 USER Chrome/Edge 时，`observe` 失败并说明需 `--tab`，不绑错浏览器
3. HTTP 单测 + 真机 poc（可用 `--tab` 对照；不要动组 1/3）
4. `make check` 0，更新本文与 README 能力描述（若有用户可见变化），再 push `main`

## 操作入口

```sh
vcu browser ping --json
vcu browser observe --json
vcu browser observe --tab <id> --json
vcu browser click --space viewport --capture <capture_id> --pixel-x <x> --pixel-y <y>
vcu browser close --tab <id>
```
