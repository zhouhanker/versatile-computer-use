# VCU 会话交接

更新：2026-09-20。CU-D-630 真机已过。

## 本轮

- 630：`observe --browser chrome|edge` 不依赖 OS 前台，定向该浏览器 focused tab；`--tab` 撞号必须带 `--browser`。真机 Chrome `#who=chrome`、Edge `#who=edge`，extract 无 `--tab` 绑 last observe。`scripts/poc_cu_d_630.py` CU-D-630 OK。组 1/3 未动。无 SendInput。`make check` 0。

## 测试边界

- 允许：`127.0.0.1` 抛页、dry-run、自己的 tab
- 禁止：组 1/3、用户站点真点、微信动作、CDP Allow、OS cursor、SendInput
- 无 `--tab`/`--browser` 的 observe 仅当前台是 USER Chrome/Edge
- tab_id 在 Chrome/Edge 间可能撞号，observe 须 `--browser`

## 下一刀

CU-D-640 候选：`close`/`select` 同样接受 `--browser` 以免撞号关错标签。停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
