# VCU 会话交接

更新：2026-09-20。CU-D-090 实现已提交，待 Windows CI。

## 本轮

- Windows Stage：WinForms HUD「VCU 正在使用这台 PC」，presenter=`winforms`，Escape 写 abort。无 SendInput。
- UIA 树带 ClassName；`ControlType.Pane/Edit` 视为可输入。
- desktop snapshot 在 Windows 上 `source=uia_scene`。
- `scripts/poc_cu_d_090.ps1`：`vcu session start --surface desktop` → snapshot → type。

## 下一刀

看 CI 是否打印 `STAGE_OK` / `SNAP_OK` / `TYPE_OK`。未绿不得宣称 090 完成。
