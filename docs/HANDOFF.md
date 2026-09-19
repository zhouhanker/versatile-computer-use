# VCU 会话交接

更新：2026-09-20。CU-B-010：tabs/health 报告已连接浏览器。

## 本轮

- `list_tabs_merged` 单客户端也返回 `browser_count`/`browsers`。
- health：`extension_browsers` / `extension_browser_count`。
- 真机：当前只有 Edge 在轮询；Chrome 进程未连上 lens（reload 只打到 1 个 client）。**双浏览器 tabs 复检未过**。
- 跨源 iframe / trusted / TC-B-040 仍明确不在范围内（已有 Node 拒绝测试）。

## 下一刀

双浏览器需 Chrome SW 轮询后再复检。不要宣称 P2 完成。禁止改用户组 1/3。
