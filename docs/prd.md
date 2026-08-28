# Grok Build Desktop — 产品需求文档（PRD）

> 状态：草案（来自产品愿景输入，待评审）  
> 产品定位：AI 编程工作台（非完整 IDE）  
> 首版平台：Windows、macOS

## 1. 产品愿景

开发一款基于 **Grok Build CLI** 的跨平台桌面应用，为用户提供**可视化、可控制、可审计**的 AI 编程工作流。

### 职责边界

| 层级 | 职责 |
|------|------|
| **Grok Build CLI** | 模型调用、上下文管理、工具执行、任务规划、Agent Loop |
| **Desktop** | 项目管理、会话交互、执行过程展示、权限审批、代码审查、Git 操作、CLI 生命周期 |

### 架构原则（硬约束）

1. Desktop **必须**通过 `grok agent stdio` 提供的 **ACP（Agent Client Protocol）** 与 Grok Build 通信。
2. Desktop **不得**解析终端 TUI 内容。
3. Desktop **不得**重新实现 Grok Agent Loop。
4. 通信形态为 JSON-RPC over stdio；扩展能力优先使用 Grok 的 `x.ai/*` 方法。

## 2. 目标用户

- 使用 Grok Build 进行日常编程的开发者
- 希望对 AI 改文件 / 跑命令有可视化审批与审计能力的用户
- 需要本地项目级会话管理、Git Diff 审查、多模式协作的用户

## 3. 首版核心能力（P0–P1）

### 3.1 CLI 生命周期

- 自动检测本机是否已安装 Grok Build CLI
- 引导安装、配置、版本校验
- CLI 进程启动 / 重启 / 健康检查
- ACP 断线检测与自动恢复
- Desktop 与 CLI **独立**版本检测与自动更新

### 3.2 身份认证

- xAI OAuth 登录
- API Key 登录
- 登录态展示、登出、重新认证
- 凭证由 CLI / `~/.grok` 管理；Desktop 不自建平行鉴权后端

### 3.3 项目管理

- 打开本地项目目录
- 最近项目列表
- 打开 / 克隆 Git 仓库
- 项目级工作目录绑定到 ACP `session/new.cwd`

### 3.4 会话与对话

- 流式 AI 对话（`agent_message_chunk`）
- 思考流展示（`agent_thought_chunk`）
- 文件引用（@file / 路径引用）
- 多轮会话
- 会话恢复、继续、Fork、搜索
- 会话列表与历史浏览

### 3.5 权限与执行模式

支持以下模式（映射到 CLI permission / always-approve 能力）：

| 模式 | 说明 |
|------|------|
| **Plan** | 规划优先，限制破坏性执行 |
| **Ask** | 默认逐项询问 |
| **Accept Edits** | 自动接受文件编辑，其他仍需审批 |
| **Always Approve** | 自动批准工具执行（高风险，需明确 UI 提示） |

高风险命令与文件修改提供**逐项审批** UI。

### 3.6 执行过程可视化

实时展示：

- 文件读取
- 文件修改
- Shell 命令
- MCP 调用
- Agent 状态 / Plan 条目
- 工具调用状态流转（pending → running → completed / failed）

数据源：ACP `session/update`（`tool_call`、`tool_call_update`、`plan` 等）。

### 3.7 代码审查与 Diff

- Side-by-side Diff
- Inline Diff
- 按文件接受 / 拒绝
- 按修改块（hunk）接受 / 拒绝 / 撤销
- 优先对接 Grok `x.ai/git/*` 与 session diff 通知

### 3.8 内置工具面

- 代码查看器（只读为主，首版不做完整编辑器）
- 内置终端（对接 `x.ai/terminal/*` 或本地 PTY）

### 3.9 模型与配置

- 模型选择
- Reasoning Effort
- 自定义模型配置（与 CLI `config.toml` / custom models 对齐）

### 3.10 Git 与任务空间

- 分支查看 / 切换
- Commit
- Worktree
- Agent 独立任务空间（worktree 隔离）

### 3.11 扩展生态可视化管理

- MCP Servers
- Skills
- Plugins
- Hooks
- Marketplace

Desktop 提供可视化管理 UI；实际配置读写优先委托 CLI / 现有 `~/.grok` 与项目配置，避免分叉两套配置源。

### 3.12 安全与隐私

- 隐私与数据保留设置展示 / 同步
- 权限规则编辑
- Sandbox 配置
- 敏感文件保护提示与拦截展示

### 3.13 诊断与可观测

- 应用日志
- 环境诊断（CLI 路径、版本、auth、网络、ACP 状态）
- CLI 进程重启
- ACP 断线恢复

## 4. 非目标（首版明确不做）

以下作为后续版本，不进入 v1 范围：

- 完整代码编辑器 / LSP 智能补全
- 多人实时协作
- 多 Agent 编排编排台
- 远程开发（Remote SSH / Dev Container 完整方案）
- Linux 首发支持（可预留，但不承诺 v1）
- 重新实现 Agent 规划 / 工具执行引擎

## 5. 成功标准（产品级）

1. 用户可在 5 分钟内完成：安装/检测 CLI → 登录 → 打开项目 → 发出第一条流式对话。
2. 任意工具调用均可在 UI 中看到类型、输入摘要、状态与结果。
3. 文件修改可在 Diff 视图中逐文件/逐 hunk 审查并接受或拒绝。
4. 高风险 Shell 命令在 Ask 模式下必须经用户明确批准后才执行。
5. CLI 进程崩溃后，Desktop 能检测、提示并在用户确认或自动策略下恢复会话。
6. 不解析 TUI；所有 Agent 交互均走 ACP。

## 6. 里程碑建议

| 里程碑 | 目标 |
|--------|------|
| **M0 骨架** | 应用壳、CLI 检测、ACP 连接、最小会话流式对话 |
| **M1 工作台** | 项目打开、权限审批、工具可视化、基础代码查看 |
| **M2 审查** | Diff 接受/拒绝、模式切换、会话恢复/Fork |
| **M3 Git/空间** | 分支/Commit/Worktree、内置终端 |
| **M4 生态与安全** | MCP/Skills/Plugins/Hooks/Marketplace UI、隐私与 Sandbox |
| **M5 发布** | 自动更新、诊断、安装包、Windows + macOS 发布流水线 |

## 7. 参考资料（本机已验证）

- `grok agent stdio` — ACP stdio transport
- `~/.grok/docs/user-guide/15-agent-mode.md` — Agent Mode / ACP
- `~/.grok/docs/user-guide/02-authentication.md` — OAuth / API Key
- `~/.grok/README.md` — ACP SDK 示例与协议概览
- ACP 规范：https://agentclientprotocol.com
- TS SDK：`@agentclientprotocol/sdk`
- Rust crate：`agent-client-protocol`
