# VCU 会话交接

更新：2026-09-20。CU-D-400 未完成。下一刀仍是 CU-D-400。

## 本轮

- 390：仍有效 run `35483037087` @ `8304b65`。
- 400：live 观察脚本 `scripts/poc_cu_d_400.py`。Chrome 与 Edge 进程都在；lens 同一时刻只 poll 其中一个。`install-lens --reload` 只 reload 已连接的那一个。打开 Edge 127.0.0.1 唤醒页未能让 Edge hello。未宣称 MERGE_OK。未动组 1/3。
- 扩展：content.js 页面加载发 `vcu_wake`；background 收到后 `bootstrapAndStart`。Node 42 项仍绿。

## 下一刀

CU-D-400：双浏览器 live Chrome+Edge 同时 hello 且 tabs 合并。不要动用户组 1/3。不要 claim 完成直到脚本打出 `CHROME_HELLO_OK` / `EDGE_HELLO_OK` / `MERGE_OK` / `CU-D-400 OK`。不要 claim MAC-NEXT / FEISHU-001。
