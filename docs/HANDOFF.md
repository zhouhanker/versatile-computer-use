# VCU 会话交接

更新：2026-09-20。CU-D-420 真机已过。

## 本轮

- 420：macOS 默认 allowlist 含 `Google Chrome` / `Chrome`；`vcu browser observe` 遍历全部 `user_browsers`，失败快照不吞错误；真机 `app_id=proc:Chrome:*` `allowed=true` 无 AppDenied。`poc_login_state.sh` PASS；`make check` 0。组 1/3 未动。无 SendInput。

## 下一刀

停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
