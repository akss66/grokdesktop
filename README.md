# Grok Build Desktop

跨平台桌面端 **AI 编程工作台**：通过 Grok Build CLI 的 ACP（`grok agent stdio`）提供可视化、可控制、可审计的 AI 编程工作流。

> 状态：M0 实现中（应用壳 + CLI 检测 + ACP 流式对话）  
> 平台目标：Windows、macOS（v1）

## 产品能力

- **可视化 Agent 会话**：在桌面界面中创建会话、发送任务并查看流式响应。
- **CLI 环境诊断**：检测本机 Grok Build CLI、版本和登录状态，明确展示连接失败原因。
- **标准 ACP 通信**：通过 stdio JSON-RPC 完成初始化、会话创建与消息流转，不依赖终端界面文本。
- **本地进程边界**：模型、工具和 Agent Loop 仍由 Grok Build 负责，桌面端只管理交互、权限与可观测状态。
- **面向下一阶段的工作台**：产品结构为 Diff 审阅、Git 操作、工具审批和项目管理预留了清晰边界。

```text
React 工作台 → Tauri Command → Rust ACP Client → grok agent stdio
      ↑                                      ↓
      └──────── 流式事件 / 状态 / 错误 ────────┘
```

## 文档

| 文档 | 说明 |
|------|------|
| [docs/prd.md](docs/prd.md) | 产品需求 |
| [docs/spec.md](docs/spec.md) | 工程规格、技术栈、边界、M0 范围 |
| [docs/acp-mapping.md](docs/acp-mapping.md) | UI 功能与 ACP/CLI 映射 |

## 架构原则

1. Desktop **只**通过 ACP 与 Grok Build 通信  
2. **不**解析终端 TUI  
3. **不**重新实现 Agent Loop  
4. Grok Build 负责模型与工具；Desktop 负责项目、会话 UI、审批、Diff、Git 与诊断  

## 当前边界

M0 已聚焦应用壳、CLI 检测和 ACP 流式对话。Diff 审阅、完整工具审批、项目工作区和发布级安装体验属于后续里程碑；README 不将规划中的能力描述为已经交付。

## 开发

### 前置条件

- Node.js 20+
- pnpm
- Rust（stable）+ 系统 WebView 依赖（见 [Tauri 前置](https://tauri.app/start/prerequisites/)）
- 本机已安装 [Grok Build CLI](https://x.ai)，且 `grok` 在 PATH 或 `~/.grok/bin`
- 已登录：`grok login` 或配置 `XAI_API_KEY`

### 安装与运行

```bash
pnpm install
pnpm tauri dev
```

仅前端（无 ACP，无法真正对话）：

```bash
pnpm dev
```

类型检查 / 测试：

```bash
pnpm typecheck
pnpm test
cargo test --manifest-path src-tauri/Cargo.toml
```

构建安装包：

```bash
pnpm tauri build
```

### M0 使用步骤

1. 启动应用后点 **检测 CLI**
2. 点 **连接 Agent**（启动 `grok agent stdio` → `initialize` → `session/new`）
3. 输入消息并发送，查看流式 `agent_message_chunk`

## 本机依赖说明

| 组件 | 用途 |
|------|------|
| Grok Build CLI | ACP 服务端（模型、工具、权限） |
| Tauri 2 + Rust | 进程管理、stdio JSON-RPC |
| React + Vite | 对话与状态 UI |

## 故障排查

| 现象 | 处理 |
|------|------|
| 未检测到 CLI | 安装 Grok 并确保 `where grok` / `which grok` 有输出 |
| 连接超时 / 认证错误 | 运行 `grok login` 或检查 API Key |
| `rustc` 被策略拦截 | 在本机安全策略中放行 `~/.cargo/bin/rustc.exe` 与 toolchain |
| 仅浏览器预览 | 必须用 `pnpm tauri dev` 才能 invoke Rust 命令 |

## 里程碑

- **M0**（当前）：壳、CLI 检测、ACP 连接、流式对话、错误面板  
- **M1**：项目打开、权限审批、工具时间线、代码查看  
- **M2+**：Diff、Git、终端、扩展与发布（见 PRD）
