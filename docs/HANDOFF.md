# VCU 会话交接

更新：2026-09-20。macOS 第一版门禁已齐。Windows 真机待 CI。

## 本轮

- 修 Windows `daemon.lock`：独占 `share_mode(0)`，PID 写进已打开的锁句柄（不再 `fs::write` 二次打开）。
- 同进程二次 `start_daemon` / `acquire_daemon_lock` 必须 `DaemonAlreadyRunning`。这是 CI run `35461855794` 挡住 UIA smoke 的原因。
- 单测 `second_lock_is_denied_then_released`；本地 `cargo test --workspace --offline` **121 passed**。

## 下一刀

看 Windows CI：`cargo test` 必须先绿，随后 `windows UIA notepad smoke` 打印 `UIA_OK` 与 `PRINTWINDOW_OK`。未绿不得宣称 Windows CU。
