# 视觉模型与主/子 Agent 协同

日期：2026-09-17

## 1. 问题

主 Agent 使用的模型可能：

- 纯文本，无多模态
- 有多模态但贵/慢/不允许传图
- 与 Computer Use 内置视觉强绑定（换模型即失效）

VCU 必须在**不绑架主模型**的前提下仍能完成“看见 UI”。

## 2. 业界常见模式

| 模式 | 做法 | 优点 | 缺点 |
| --- | --- | --- | --- |
| M1 主模型自带视觉 | 截图直接进主模型 | 简单 | 换模型即挂；用户原问题场景 |
| M2 工具内隐式视觉 | Runtime 调视觉 API，把**结构化描述**回主模型 | 主模型可纯文本；集成简单 | 要管理密钥与成本；描述可能失真 |
| M3 子 Agent 视觉专家 | 主 Agent 派生子任务“看图点哪里” | 角色清晰；可不同模型 | 编排复杂；状态同步难 |
| M4 纯 DOM/a11y 无视觉 | 只走可访问性树 | 无视觉成本 | 自定义 Canvas/游戏/图标按钮失败 |
| M5 混合 grounding | DOM 优先，失败升级截图视觉 | 成本与成功率平衡 | 策略工程 |

成熟自动化（Playwright 系、BrowserSkill 技能说明、local-browser-agent 等）普遍：**DOM/a11y 优先，视觉兜底**。  
Computer Use 演示则常 **vision-first**。VCU 作为“可插拔 CU”应 **可配置策略**，默认 **M5**。

## 3. VCU 推荐：可插拔 Vision Provider + 混合感知

```text
Main Agent (any model)
    | vcu snapshot / act
    v
vcu-daemon
    |- Perception pipeline
    |    |- a11y/dom (always local)
    |    |- optional screenshot
    |    |- if need_vision && provider configured:
    |         call vision model → elements/coords/caption
    |- return packet to Main Agent (text-first)
```

### 3.1 CLI 配置（用户要求形态）

```sh
vcu init                 # 初始化 ~/.vcu 与 doctor
vcu init model           # 向导：配置视觉（或默认）模型
vcu model list
vcu model set vision --provider openai-compatible \
  --base-url https://api.example.com/v1 \
  --model qwen-vl-plus \
  --api-key-env VCU_VISION_API_KEY
vcu model test vision    # 发最小测图，验证连通
vcu model set policy --mode dom_first|vision_first|dom_only|vision_always
```

说明：

- 密钥进环境变量或 OS 密钥链，**不进项目 Git**
- `openai-compatible` 覆盖多数网关；后续加 `anthropic`/`local-ollama` 等 provider
- 主 Agent 无视觉时，**不必**改主模型，只要 VCU 配了 vision

### 3.2 是否要“主 Agent + 子 Agent”？

**要有能力，但默认不强迫用户感知两套对话。**

| 协同级别 | 表现 | 何时用 |
| --- | --- | --- |
| L0 工具内视觉 | daemon 内部调用 vision，主 Agent 只看见工具结果 | **默认** |
| L1 共享会话黑板 | `~/.vcu/sessions/<id>/blackboard.json`：截图句柄、候选元素、最后动作；主/子读写 | 多工具并行、可恢复 |
| L2 显式子 Agent | `vcu agent spawn vision --session <id>` 或 MCP 资源；子 Agent 专用视觉模型 | 宿主支持多 Agent（Codex/Pi 等）且任务长 |

信息共享最小集（对齐 AWR checkpoint 思想，但更轻）：

```json
{
  "session_id": "...",
  "revision": 3,
  "observation_id": "...",
  "dom_summary": "...",
  "vision_summary": "...",
  "candidates": [{"ref": "e12", "role": "button", "name": "Submit"}],
  "next_action_hint": null,
  "open_loops": []
}
```

主 Agent 与视觉子 Agent **不共享完整聊天历史**，只共享 **session blackboard + 证据句柄**（截图存本地路径/content-addressed，不默认回传 base64 进主上下文）。

## 4. 无视觉模型时的行为

若 `vision` 未配置且策略需要视觉：

1. `vcu doctor` / 工具返回 **明确错误码** `VisionProviderRequired`
2. 提示运行 `vcu init model`
3. 对纯 DOM 可完成的任务，策略 `dom_only` 仍可工作

**不会**静默调用某家默认云模型（避免数据与费用惊吓）。

## 5. 与宿主多 Agent 的关系

- Codex/Claude 等若提供子 Agent：VCU 暴露 `session` 与 `blackboard` 供其挂载
- 若宿主只有单 Agent：L0 足够
- VCU **不实现**通用多 Agent 框架；只提供 **会话级共享状态与 vision 调用**

## 6. 结论

- **必须**支持 `vcu init model` 配置独立视觉模型  
- **默认** daemon 内混合感知（DOM 优先 + 可选视觉）  
- **可选** 主/子 Agent 通过 session blackboard 协同  
- 主模型无多模态 **不是** 阻断条件
