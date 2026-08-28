# Spec: Grok Build Desktop

## Objective

构建 **Grok Build Desktop**：一款 Windows / macOS 桌面应用，作为 Grok Build CLI 的可视化客户端（AI 编程工作台）。

用户通过 Desktop 打开本地项目、与 Agent 流式对话、审批工具执行、审查代码 Diff、管理会话与扩展配置；所有智能能力由本机 `grok agent stdio`（ACP）提供。

**成功定义：** 见 [docs/prd.md](./prd.md) §5；工程上以 M0→M5 里程碑可演示、可测试为完成标准。

## Tech Stack（推荐，待确认）

| 层 | 选择 | 理由 |
|----|------|------|
| 应用壳 | **Tauri 2** | 原生窗口、体积小、系统 WebView、Rust 适合进程/IPC |
| 前端 | **React 19 + TypeScript + Vite** | 组件生态成熟，适合对话流、Diff、面板布局 |
| UI | **Tailwind CSS + 自研设计系统** | 快速迭代；避免重型 UI 库锁定 |
| 状态 | **Zustand**（UI）+ 事件总线（ACP 流） | 简单可控；流式更新不适合重 Redux |
| ACP 客户端 | **Rust `agent-client-protocol`**（主）或 TS `@agentclientprotocol/sdk`（备） | 进程管理放 Rust 更稳；UI 只消费结构化事件 |
| Git Diff UI | `diff` 库 + 自研 Side-by-side/Inline | 优先消费 CLI `x.ai/git/*` 与 session diff |
| 终端 | xterm.js + 后端 PTY（或 `x.ai/terminal/*`） | 内置终端标准方案 |
| 打包 | Tauri bundler（msi/nsis + dmg） | 平台安装包 |
| 更新 | Tauri updater + CLI 自有 `grok update` | Desktop / CLI 版本通道分离 |
| 测试 | Vitest（前端单元）+ Rust tests（ACP 适配层）+ Playwright（可选 E2E） | 分层验证 |

### 备选方案

| 方案 | 何时采用 |
|------|----------|
| **Electron + React** | 若团队更熟 Node、需要完整 Chromium、可接受更大安装包 |
| **纯 TS ACP（Node sidecar）** | 若 Rust ACP crate 能力不足，可用 Node 子进程跑 `@agentclientprotocol/sdk` |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Desktop UI (React)                                         │
│  Chat | Tool Timeline | Diff Review | File Viewer | Terminal│
│  Project | Sessions | Settings | Diagnostics                │
└───────────────────────────┬─────────────────────────────────┘
                            │ Tauri commands / events
┌───────────────────────────▼─────────────────────────────────┐
│  Desktop Core (Rust)                                        │
│  - CliManager: detect / install / spawn / restart grok      │
│  - AcpClient: JSON-RPC over stdio                           │
│  - SessionStore: session metadata index                     │
│  - PermissionBroker: tool permission UI bridge              │
│  - GitFacade: thin wrapper over x.ai/git/* + local git      │
│  - UpdateService: app + CLI version checks                  │
│  - Diagnostics: env report, logs                            │
└───────────────────────────┬─────────────────────────────────┘
                            │ stdin/stdout JSON-RPC
┌───────────────────────────▼─────────────────────────────────┐
│  grok agent stdio  (Grok Build CLI)                         │
│  Model | Context | Tools | MCP | Plan | Permissions         │
└─────────────────────────────────────────────────────────────┘
```

### 硬边界

- **Always via ACP**：对话、工具、权限、plan、thought 一律 ACP。
- **Never parse TUI**：禁止屏幕抓取 / ANSI 解析作为功能路径。
- **Never reimplement Agent Loop**：Desktop 不做自主 tool-calling 循环。
- **配置单一事实源**：优先 `~/.grok/config.toml`、项目规则与 CLI 管理命令；Desktop 只提供 UI。

## Commands

```bash
# 开发
pnpm install
pnpm tauri dev

# 前端 only
pnpm dev

# 类型检查 / lint / test
pnpm typecheck
pnpm lint
pnpm test
cargo test --manifest-path src-tauri/Cargo.toml

# 构建
pnpm tauri build

# 诊断（运行时）
# UI: Settings → Diagnostics
# 或 CLI: grok version && grok inspect
```

## Project Structure

```
grokdesktop/
├── docs/
│   ├── prd.md                 # 产品需求
│   ├── spec.md                # 本工程规格（本文）
│   ├── architecture.md        # 架构细节（后续）
│   └── acp-mapping.md         # UI 功能 ↔ ACP 方法映射
├── src/                       # React 前端
│   ├── app/                   # 路由 / 布局 / 壳
│   ├── features/
│   │   ├── chat/              # 对话与流式渲染
│   │   ├── tools/             # 工具时间线
│   │   ├── permissions/       # 审批 UI
│   │   ├── diff/              # Diff 审查
│   │   ├── project/           # 项目 / 最近打开
│   │   ├── sessions/          # 会话列表 / 恢复 / Fork
│   │   ├── terminal/          # 内置终端
│   │   ├── viewer/            # 代码查看器
│   │   ├── git/               # 分支 / commit / worktree
│   │   ├── extensions/        # MCP / Skills / Plugins / Hooks
│   │   ├── settings/          # 模型 / 隐私 / 权限规则
│   │   └── diagnostics/       # 日志与环境诊断
│   ├── shared/                # UI 组件、hooks、types
│   └── main.tsx
├── src-tauri/                 # Tauri / Rust 核心
│   ├── src/
│   │   ├── main.rs
│   │   ├── cli/
│   │   ├── acp/
│   │   ├── session/
│   │   ├── permission/
│   │   ├── update/
│   │   └── diagnostics/
│   └── Cargo.toml
├── tests/
├── package.json
├── README.md
└── AGENTS.md                  # Agent 协作约定
```

## Code Style

- TypeScript：严格模式，`type` 优先于 `interface`（除非需声明合并）。
- 组件：功能切片（feature folder），避免巨型 `components/` 垃圾场。
- 命名：React 组件 PascalCase；hooks `useX`；Rust 模块 snake_case。
- ACP 事件：前端只消费 **已规范化的领域事件**，不直接绑 JSON-RPC 原始字段到 UI。

```typescript
// Good: domain event for UI
type AgentStreamEvent =
  | { type: "message_delta"; text: string }
  | { type: "thought_delta"; text: string }
  | { type: "tool_call"; id: string; kind: ToolKind; title: string; input: unknown }
  | { type: "tool_update"; id: string; status: ToolStatus; output?: unknown }
  | { type: "plan"; entries: PlanEntry[] }
  | { type: "permission_request"; requestId: string; toolCallId: string; summary: string };

// Bad: UI components parsing raw JSON-RPC
// <Chat rawRpc={line} />
```

## Testing Strategy

| 层级 | 范围 | 工具 |
|------|------|------|
| 单元 | 事件规范化、Diff hunk 操作、权限模式映射 | Vitest / Rust tests |
| 集成 | 伪 ACP 进程（fixture JSON-RPC）↔ AcpClient | Rust tests |
| 组件 | Chat 流式渲染、审批弹窗、Diff 接受/拒绝 | Vitest + Testing Library |
| E2E（后期） | 打开项目 → 发消息 → 看到工具卡片 | Playwright + mock ACP |
| 手工 | 真机连真实 `grok agent stdio` | 检查清单 |

覆盖率目标：核心适配层（ACP 映射、权限、CLI 生命周期）> 80%；UI 不强制全局覆盖率。

## Boundaries

### Always

- 通过 ACP 与 Grok 通信
- 启动前检测 CLI 可用性与版本兼容性
- 权限请求默认安全（拒绝 / 取消不得静默放行）
- 变更附带可运行验证（至少 typecheck / 相关测试）
- 日志不落盘 secrets（token、API key）

### Ask first

- 新增重量级依赖（编辑器内核、完整 IDE 框架等）
- 改变 Tech Stack 或进程架构（例如改 Electron）
- 引入遥测 / 崩溃上报
- 扩大范围到 PRD 非目标（LSP、多 Agent 编排等）
- 发布到公网分发渠道

### Never

- 解析 Grok TUI 输出作为功能实现
- 在 Desktop 内重新实现模型调用 / tool loop
- 提交 `auth.json`、API keys、用户项目代码到本仓库
- 在无用户批准时执行 Always Approve 等价行为（除非用户显式选择该模式）

## Feature ↔ ACP Mapping（摘要）

| Desktop 功能 | 协议 / 来源 |
|--------------|-------------|
| 连接 Agent | spawn `grok agent stdio` |
| 初始化 | `initialize` |
| 新建会话 | `session/new` + `cwd` |
| 发送消息 | `session/prompt` |
| 流式文本 / 思考 | `session/update` → `agent_message_chunk` / `agent_thought_chunk` |
| 工具可视化 | `tool_call` / `tool_call_update` |
| Plan 展示 | `plan` |
| 权限审批 | ACP permission request / response |
| 会话 Fork | `x.ai/session/fork` |
| Git 状态 / Diff / Commit | `x.ai/git/*` |
| Worktree | `x.ai/git/worktree/*` |
| 终端 | `x.ai/terminal/*` 或本地 PTY |
| 认证辅助 | `x.ai/auth/*` + `grok login` |
| 文件系统索引 | `x.ai/fs/*` / `x.ai/fs_notify` |

完整表见后续 `docs/acp-mapping.md`。

## Permission Mode Mapping

| UI 模式 | CLI / 行为意图 |
|---------|----------------|
| Plan | `permission-mode=plan` / plan-first 行为 |
| Ask | default / 逐项询问 |
| Accept Edits | `acceptEdits` |
| Always Approve | `--always-approve` 或等价 session 模式 |

UI 必须对 Always Approve 使用醒目警告样式，并支持一键退回 Ask。

## M0 可交付范围（第一个可运行切片）

1. Tauri + React 应用壳可启动
2. 检测 `grok` 是否在 PATH / 常见安装路径
3. 启动 `grok agent stdio`，完成 `initialize` + `session/new`
4. 发送一条 prompt，流式渲染 `agent_message_chunk`
5. 展示连接状态（connected / disconnected / restarting）
6. 基础错误面板（CLI 未安装、认证失败、进程退出）

**M0 验收：**

- [x] 工程骨架与前端构建：`pnpm typecheck` / `pnpm build` 通过
- [x] CLI 检测 + ACP 客户端 + 流式领域事件代码已落地（Rust）
- [ ] `pnpm tauri dev` 可启动窗口（依赖本机 `rustc` 可用）
- [ ] 本机已安装并已登录 Grok 时，可完成一轮流式对话
- [x] 未安装 CLI 时 UI 展示安装引导（非白屏崩溃）
- [x] 无 TUI 解析代码路径

## Success Criteria（工程）

- [ ] 规格文档经人工确认
- [ ] 仓库可复现构建（Windows 优先，macOS 并行）
- [ ] ACP 适配层有 fixture 测试
- [ ] PRD P0 能力按 M0–M5 落地且每里程碑可演示
- [ ] Desktop 与 CLI 版本兼容策略文档化（semver / min CLI version）

## 已确认决策

| 项 | 决定 |
|----|------|
| 应用壳 | **Tauri 2** |
| UI 语言 | **中文** |
| CLI 安装 | **内置下载安装**（对接官方 install / `grok update`） |
| ACP 实现 | **Rust 主路径**（stdio JSON-RPC） |
| 包管理 | **pnpm** |
| 平台 | Windows + macOS（v1） |
| 包名 | `ai.x.grok.desktop`（可后续调整） |

## Assumptions（其余默认）

1. 产品名：**Grok Build Desktop**
2. 认证与模型调用全部走本机 Grok CLI
3. 配置与会话复用 `~/.grok`；Desktop 只存 UI 偏好
4. 首版代码查看器只读；完整编辑器不在 v1
5. 自动更新渠道后续再定；M0 先做 CLI 内置安装
