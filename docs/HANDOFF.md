# VCU 会话交接

更新：2026-09-20（中断恢复后重写）。上一提交 `7a7a7c8` CU-D-670 真机已过。

## 现场（2026-09-20 收尾：CU-D-680/690/700 全绿）

- CU-D-680（`group-update --browser`）**真机通过**：`scripts/poc_cu_d_680.py` 打印 `CU-D-680 OK`，报告在 `.local/desktop-cu/cu-d-680.json`。真机覆盖：组列表带 `browser` 标记；`--browser chrome|edge` 只改对应组（title/color/collapsed），另一浏览器组不动；id 唯一时无 `--browser` 正确解析；另一浏览器真实 id + 错 `--browser` → 诚实失败且不误改；未知 id → `No group with id`；组清理干净，组 1/3 未动。撞号分支由 `cargo test -p vcu-server --test app_http`（`upd_amb`）覆盖（真机构造不出同号 `group_id`）。无 SendInput。
- CU-D-690（extension DOM 原生 `<select>`）**真机通过**：`type --selector` 接受 `<select>`，按 option value/可见文本匹配，设置后派发 `input`+`change`，回执 `input_path=dom_select`；匹配不到报 `select option not found` 且不改值；文本输入仍 `dom_type`。`scripts/poc_cu_d_690.py` CU-D-690 OK。真机验收：用 VCU 在 GitHub Support 工单页选中「Type of Issue」并**成功提交**（页面回执「您的信息已成功提交」，证据 `.local/desktop-cu/cu-d-690-github-ticket.png`）。
- CU-D-700（Release 托管 + `vcu self update`）**完成**：发布 Release `v0.2.8`（Latest，16 资产）；`curl -fsSL .../releases/latest/download/install.sh | sh` 装到临时前缀成功；不带 `VCU_BASE_URL` 的 `vcu self update` → `updated: true`，装出的 vcu/vcu-daemon/vcu-mcp 与 Release 包 sha256 完全一致。失败路径输出 installer stderr + 本地 `file://` 提示（单测 `crates/vcu-cli/tests/self_update_failure.rs`，真机 404 复现）。修了两个发布管线缺陷：0 字节 `dist/.gitkeep` 被当资产（publish 失败）与 Windows checkout 的 CRLF `install.sh` 覆盖 LF（`curl \| sh` 失败）；已重传修复后的 `install.sh`，工作流修复（显式资产列表 + `tr` 归一化 + `.gitattributes`）**待下一次打 tag 端到端验证**。
- 门禁：`cargo test --workspace` 全绿、`node --test extension/tests/*.test.cjs` 46 通过、`make check` 0（含打包与 curl 安装 POC）。
- 仓库残留 `extension/background.js.bak`（2460B，2026-09-17 旧 service worker）已删除。
- 本机安装的 CLI 用 `VCU_BASE_URL=file://$PWD/dist vcu self update` 重建（含 CU-D-680 代码）；daemon 已重启、lens 已重连。Chrome 由本轮真机需要而启动（此前只有 Edge 在线）。

## 两个真实问题（2026-09-20 现场发现）

1. **原生 `<select>` 无法设置（CU-D-690）**：GitHub Support 工单页「Type of Issue」是原生 select。证据：`type` → `target is not an editable text element`；点 `option` → `target has no visible bounds`；`key` 只出策略 plan（`pressed=false`，不注入按键）；直接 Submit 被校验「需要问题类型」拒绝。根因：`extension/content.js` 的 `validateTarget(el,{editable:true})` 只认 input/textarea/contenteditable；弹层是原生菜单，DOM 点不到。
2. **`vcu self update` 无 Release 可用（CU-D-700，对应台账 MAC-NEXT）**：`GET /repos/.../releases` 为空 → installer 退出非零，CLI 只报 `update installer exited non-zero`（不透传 stderr）。本地通路 `VCU_BASE_URL=file://$PWD/dist` 已实测 `updated: true`；`vcu --version` 恒为 crate 版本 0.1.0，浏览器桥才是 0.2.8。

## 测试边界

- 允许：`127.0.0.1` 抛页、dry-run、自己的 tab
- 禁止：组 1/3、用户站点真点、微信动作、CDP Allow、OS cursor、SendInput
- 原生标签组只属于一个浏览器窗口

## 下一刀

台账里已无进行中的 CU-D 切片。可选后续（按价值）：

1. **下一次打 tag 时验证发布管线修复**：确认 publish job 绿、Release 资产无 0 字节文件、`install.sh` 为 LF。
2. **已知问题：CI `test (windows-latest)` 持续红**（最近多次 push 都失败，与本轮切片无关）——修好后 Windows 侧才有可信门禁。
3. AWR 完成度登记：`work complete`/`evidence add` 的报告 schema 仍未摸清（模板未公开），当前用台账 `status: completed` + docs 证据指针，和仓库既有做法一致。

停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU / MAC-NEXT 深 AX / FEISHU-001。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
