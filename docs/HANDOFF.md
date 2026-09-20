# VCU 会话交接

更新：2026-09-20 本轮收尾。HEAD `b627506`（main 已推送）。上一轮结尾为 `7a7a7c8`（CU-D-670）。

## 本轮完成（CU-D-680 → CU-D-690 → CU-D-700，全部提交并推送）

- **CU-D-680 `group-update --browser`（真机通过）**：`scripts/poc_cu_d_680.py` → `CU-D-680 OK`，报告 `.local/desktop-cu/cu-d-680.json`。覆盖：合并列表带 `browser` 标记；`--browser chrome|edge` 只改对应组（title/color/collapsed），另一浏览器组不动；id 唯一时无 `--browser` 正确解析；另一浏览器真实 id + 错 `--browser`、以及未知 id 都是诚实失败且不误改；组清理干净、组 1/3 未动。撞号分支由 `cargo test -p vcu-server --test app_http`（`upd_amb`）覆盖（真机构造不出同号 `group_id`）。提交 `03db63b`、`1ad02ca`、`84e48ec`。
- **CU-D-690 extension DOM 原生 `<select>`（真机通过）**：`type --selector` 接受 `<select>`，按 option 的 value 或可见文本匹配，设置后派发 `input`+`change`，回执 `input_path=dom_select`；匹配不到报 `select option not found` 且不改值；文本输入仍是 `dom_type`。单测 46 项（`extension/tests/content.test.cjs`）；真机 `scripts/poc_cu_d_690.py` → `CU-D-690 OK`。真机验收：用 VCU 在 GitHub Support 工单页选中「Type of Issue」并**成功提交**（页面回执「您的信息已成功提交」，证据 `.local/desktop-cu/cu-d-690-github-ticket.png`）。提交 `ee66ae7`、`1af8764`。
- **CU-D-700 Release 托管 + `vcu self update`（完成）**：发布 GitHub Release `v0.2.8`（Latest，16 资产）。`curl -fsSL .../releases/latest/download/install.sh | sh` 装到临时前缀成功；不带 `VCU_BASE_URL` 的 `vcu self update` → `updated: true`，装出的 vcu/vcu-daemon/vcu-mcp 与 Release 包 sha256 完全一致。失败路径输出 installer stderr + 本地 `file://` 提示（单测 `crates/vcu-cli/tests/self_update_failure.rs`，并用真机 404 复现）。提交 `98737ca`、`f9a94c7`。
- **顺手清理 / 修复**：删除被 git 跟踪的残留 `extension/background.js.bak`（`fdd76f9`）；修发布管线两个缺陷——0 字节 `dist/.gitkeep` 被当资产导致 publish job 失败（改显式资产列表）、Windows checkout 的 CRLF `install.sh` 覆盖 LF 导致 `curl | sh` 报 `set: pipefail: invalid option name`（`.gitattributes` + publish 步骤 `tr` 归一化）；提交 `c0c49dd`，并用临时 tag 端到端验证（run `35511683467` 绿：14 资产、无 0 字节、install.sh 为 LF），验证后删除该 tag，记录于 `b627506`。

## 门禁与证据

- `cargo test --workspace` 全绿；`node --test extension/tests/*.test.cjs` **46** 通过；`make check` **0**（mock 流程 / 额外动作 / 打包 / curl 安装 POC）。
- 真机 POC：`scripts/poc_cu_d_680.py`、`scripts/poc_cu_d_690.py`（报告 `.local/desktop-cu/`）。
- 发布冒烟：`curl | sh` 临时前缀安装 + `vcu self update` 二进制 sha256 对照（对照对象是线上 Release 资产）。

## 本机环境状态

- `~/.local/bin` 的 `vcu` / `vcu-daemon` / `vcu-mcp` 是 **Release v0.2.8 包内二进制**（sha256 与 release tarball 逐字节一致），`vcu-stage` 也在。
- `vcu --version` 仍打印 crate 版本 `0.1.0`（浏览器桥版本为 0.2.8）——版本号不变不代表没更新。
- daemon 在跑（`127.0.0.1:17890`），lens 已重连，`extension_browsers=["chrome","edge"]`。本轮为真机需要启动了 Chrome（此前只有 Edge 在线）。
- 本轮排查发布问题时下载的临时文件在 `/private/tmp`（可忽略）。AWR 无未关闭会话/claim。

## 本轮解决的两个真实问题（保留现场记录，均已修复）

1. **原生 `<select>` 无法设置** → CU-D-690。现场证据：`type` → `target is not an editable text element`；点 `option` → `target has no visible bounds`；`key` 只出策略 plan（`pressed=false`）。根因：`validateTarget(el,{editable:true})` 只认 input/textarea/contenteditable，且原生弹层点不到。
2. **`vcu self update` 不可用且不解释** → CU-D-700。现场证据：GitHub Releases 为空 → installer 非零退出，CLI 只报 `update installer exited non-zero`（stderr 被 `Stdio::null()` 吞掉）。

## 测试边界（不变）

- 允许：`127.0.0.1` 抛页、dry-run、自己的 tab
- 禁止：组 1/3、用户站点真点、微信动作、CDP Allow、OS cursor、SendInput
- 原生标签组只属于一个浏览器窗口

## 下次会话

1. 台账 `.awr/intake/work-ledger.yaml` 已无进行中切片；可领取只剩 `MAC-NEXT`（深 AX，停放）与 `FEISHU-001`（停放）。**不要 claim 这两项**，除非用户明确要开。
2. 已知问题：CI `test (windows-latest)` 最近多次 push 持续失败（与本轮切片无关）；修好后 Windows 侧门禁才可信。
3. AWR 完成登记：`work complete` / `evidence add` 的报告 schema 未摸清（模板未公开，报错只有通用提示），现用台账 `status: completed` + docs 证据指针，与仓库既有 62 条 completed 的做法一致。
4. 停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU / MAC-NEXT 深 AX / FEISHU-001。不要 claim 完整 Codex CU 或完整 Windows 产品 CU。
