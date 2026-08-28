# Agent 协作约定

## 硬边界

- 只通过 ACP（`grok agent stdio`）与 Grok 通信；禁止解析 TUI。
- 不在 Desktop 内重新实现 Agent Loop / 模型调用。
- 配置单一事实源：`~/.grok` + CLI；Desktop 只做 UI 与本地窗口偏好。

## 代码组织

- 前端按 `src/features/*` 切片；领域事件类型在 `src/shared/types.ts`。
- Rust ACP 适配在 `src-tauri/src/acp/`；CLI 检测在 `src-tauri/src/cli/`。
- UI 只消费规范化事件（`AgentStreamEvent`），不直接绑定原始 JSON-RPC。

## 变更要求

- 改 ACP 映射时同步更新 `docs/acp-mapping.md`。
- 扩大 PRD 非目标范围前先询问用户。
- 不提交 secrets、`auth.json`、用户项目代码。
