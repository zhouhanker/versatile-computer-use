# 浏览器架构调研（不打断 · 可接管 · Chrome/Edge）

日期：2026-09-17

## 1. 硬性交互约束

来自用户强制需求，转化为架构不变量：

| ID | 不变量 | 含义 |
| --- | --- | --- |
| B1 | 不打断用户操作 | Agent 浏览器工作默认在**独立窗口/标签容器**；不抢用户当前前台页的输入焦点 |
| B2 | 不抢占系统光标 | 浏览器期**禁止**把 OS 级鼠标挪到用户屏幕当主路径；点击/输入走 **CDP Input 或扩展 DOM 事件** |
| B3 | 可接管已打开浏览器 | 能附着用户已运行的 Chrome/Edge，复用登录态；接管用户标签必须**显式** |
| B4 | 首批 Chrome + Edge | 以 Chromium 扩展 + CDP 为公约数 |
| B5 | 可抓取 | 提供 DOM/a11y/文本/截图等只读与受控抽取 |

## 2. 推荐默认拓扑

```text
[Agent: Codex/Claude/Pi/...]
        |  shell or MCP
        v
   vcu CLI  ----IPC----  vcu-daemon
                              |
                              | authenticated channel
                              v
                     Browser Extension (Chrome/Edge)
                              |
              +---------------+---------------+
              |                               |
      Agent Window (工作面)            User Windows (日常)
      - 导航/执行/抓取                  - 默认只读列举
      - 默认焦点隔离                    - 仅 borrow 后可写
```

### 2.1 Agent Window

- 扩展创建**独立窗口**作为 Agent 工作面（对齐 BrowserSkill）
- 用户可看见 Agent 在做什么（可观测、可中止）
- 用户自己的窗口继续用来浏览；二者输入焦点分离

### 2.2 Borrow / Return（接管）

- `tabs.list`：列举用户窗与 Agent 窗标签（元数据：title/url/id，可配置是否暴露）
- `tabs.borrow --tab <id>`：将用户标签暂时交 Agent 会话；**单一借用人**；UI 明确标记
- `tabs.return`：归还；会话结束必须归还（daemon 负责）
- 未借用的用户标签：**默认不可 click/type**，只可对已授权范围做只读 snapshot（可再收紧为需额外 grant）

### 2.3 输入路径

| 动作 | 允许路径 | 禁止路径（浏览器期） |
| --- | --- | --- |
| click/type/scroll | 扩展或 CDP → 目标 page target | `CGEvent` / `SendInput` 移动用户物理光标 |
| 切标签 | 扩展 API 在 Agent 窗内 | 强制把用户前台窗拉到前台（除非用户设置允许） |
| 截图 | 扩展 capture 或 CDP `Page.captureScreenshot` | 全屏 OS 截屏作为默认（隐私面过大；可作高级选项） |

## 3. 附着已运行浏览器

### 3.1 主路径：扩展

用户安装 VCU 扩展 → 扩展连本地 daemon（loopback + pairing）→ 无需用户改启动参数。

优点：商店分发、权限可见、不开裸调试端口。  
成本：要过 Chrome/Edge 扩展审核与企业策略。

### 3.2 降级路径：受控 CDP

适用：CI、无 UI 服务器、扩展被禁。

- 使用**专用 user-data-dir** 或用户显式开启的 debugging
- daemon 只连接 **pairing 过的端口**，不扫描全机盲连
- 文档说明 Chrome 高版本对**默认 profile + remote debugging** 的限制

### 3.3 与“光标设计 / Codex”的对齐

Codex 公开路径包括 in-app browser 与真实 Chrome 扩展/CDP。VCU **不内嵌完整 Electron 浏览器产品**（避免再做一个 Agent App），而是：

- 扩展 + daemon 提供**同等“真实浏览器”能力**
- 用 CLI/MCP 暴露给**任何** Agent，而不是只给一个桌面壳

## 4. 会话与并发

- 每个 Agent 任务：`vcu session start` → `session_id`
- 多 Agent 并行：多 session；**同一用户 tab 同时只允许一个写借用**
- 焦点策略：`activationPolicy=never` 类行为（具体 API 按 Chromium 扩展能力）——目标是**不 steal focus**
- 命令带 `--session`；支持 `--json` 稳定字段

## 5. 抓取（Scrape）模型

```text
snapshot
  ├─ a11y tree (优先给无视觉模型)
  ├─ dom simplified
  ├─ text / markdown
  └─ screenshot (可选；可触发 vision)

extract
  ├─ css / role / testid
  ├─ table
  └─ jsonld / meta

network (二期)
  └─ 过滤后的响应正文
```

安全默认：

- 截图与 DOM 默认**本地**，只经 Agent 上下文返回摘要或受控片段
- 禁止工具主动导出 Cookie/LocalStorage 密钥；`doctor` 检查扩展权限最小化

## 6. 人机协同

对齐 BrowserSkill 的 human-in-the-loop：

- `vcu session request-help --reason captcha|login|payment|other`
- 用户在 Agent Window 完成 → `continue`
- 超时策略与会话挂起写入可恢复状态（借鉴 AWR checkpoint 语义）

## 7. 风险与缓释

| 风险 | 缓释 |
| --- | --- |
| 扩展被企业禁用 | CDP 降级 + 清晰 doctor |
| 借用标签导致用户丢失上下文 | 强 UI 标记 + 自动 return + 超时 |
| 本机恶意进程连 daemon | 配对 token、socket 权限、可选 OS keychain |
| 站点反自动化 | 真实 profile + 人类节奏可选；不承诺绕过风控 |
| 与 BrowserSkill 功能重叠 | 产品上强调 CU 协议/视觉/桌面演进；技术上可先适配再自研关键路径 |

## 8. 结论

浏览器 MVP 架构锁定为：**扩展 + daemon + 独立 Agent Window + 显式 borrow + 页内输入（无 OS 光标）+ Chrome/Edge**。  
CDP 直连为降级。抓取为一等只读工具面。
