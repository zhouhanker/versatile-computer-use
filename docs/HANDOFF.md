# VCU 会话交接

更新：2026-09-20（中断恢复后重写）。上一提交 `7a7a7c8` CU-D-670 真机已过。

## 现场（中断恢复 → CU-D-680 收尾）

- CU-D-680（`group-update --browser`）**真机通过**：`scripts/poc_cu_d_680.py` 打印 `CU-D-680 OK`，报告在 `.local/desktop-cu/cu-d-680.json`。真机覆盖：组列表带 `browser` 标记；`--browser chrome|edge` 只改对应组（title/color/collapsed），另一浏览器组不动；id 唯一时无 `--browser` 正确解析；另一浏览器真实 id + 错 `--browser` → 诚实失败且不误改；未知 id → `No group with id`；组清理干净，组 1/3 未动。撞号分支由 `cargo test -p vcu-server --test app_http`（`upd_amb`）覆盖（真机构造不出同号 `group_id`）。无 SendInput。
- 门禁：`cargo test --workspace` 全绿、`node --test extension/tests/*.test.cjs` 45 通过、`make check` 0（含打包与 curl 安装 POC）。
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

按此顺序：

1. **CU-D-690**：extension DOM 支持原生 `<select>`（设 value + 派发 `input`/`change`，`input_path=dom_select`，`trusted=false`）；单测 + 抛页 POC；真机验收＝用 VCU 完成 GitHub Support 工单「Type of Issue」选择并提交。
2. **CU-D-700**：发布 Release 资产（`dist/*.tar.gz` + `install.sh/ps1`）使 `curl | sh` 与 `vcu self update` 开箱可用；同时让 `self update` 失败时透传 installer stderr 并提示 `VCU_BASE_URL=file://...`。

停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU / MAC-NEXT 深 AX。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
