# UI Feature ↔ ACP / CLI Mapping

本表是 Desktop 功能实现的协议索引。实现时以本机 `~/.grok/docs/user-guide/15-agent-mode.md` 与 ACP 规范为准；`x.ai/*` 扩展以 `initialize` 返回的能力为准。

## Core session

| UI | Direction | Method / Event | Notes |
|----|-----------|----------------|-------|
| Connect | Client→Agent | spawn `grok agent [opts] stdio` | opts: model, always-approve, etc. |
| Handshake | C→A | `initialize` | protocolVersion, clientCapabilities |
| New chat | C→A | `session/new` | `cwd`, optional `mcpServers`, `_meta` |
| Load/resume | C→A | session load / resume (ACP + CLI sessions) | Align with `grok sessions` |
| Send message | C→A | `session/prompt` | text + file refs as content blocks |
| Stream text | A→C | `session/update` / `agent_message_chunk` | append to assistant bubble |
| Stream thought | A→C | `agent_thought_chunk` | collapsible reasoning panel |
| Tool start | A→C | `tool_call` | timeline card |
| Tool progress | A→C | `tool_call_update` | status/output |
| Plan | A→C | `plan` | plan panel |
| Permission | A→C / C→A | permission request/response | Ask / Accept Edits gates |
| Cancel turn | C→A | cancel (if supported) | stop generation |

## Grok extensions (`x.ai/*`)

| UI | Method family | Examples |
|----|---------------|----------|
| File tree / open file | `x.ai/fs/*` | list, read_file, exists |
| FS live updates | notifications | `x.ai/fs_notify`, `x.ai/fs/index`, delta |
| Git status / stage / commit | `x.ai/git/*` | status, stage, commit, diffs, discard |
| Worktree tasks | `x.ai/git/worktree/*` | create, remove, apply, list, gc |
| Fuzzy file/change search | `x.ai/search/*` | fuzzy/open, fuzzy/change, content |
| Integrated terminal | `x.ai/terminal/*` | create, output, kill, wait_for_exit |
| Session fork | `x.ai/session/*` | fork, worktree resume helpers |
| History / rewind / compact | `x.ai/*` | prompt_history, rewind/*, compact_conversation |
| OAuth helpers | `x.ai/auth/*` | get_url, submit_code |
| Diff review push | notifications | `x.ai/session_notification` |

## CLI-managed surfaces (Desktop UI wrappers)

这些能力可能部分不在 ACP 内，Desktop 通过调用 `grok` 子命令或读写约定配置实现：

| UI | CLI / path |
|----|------------|
| Login / logout | `grok login`, `grok logout` |
| Models list | `grok models` |
| Version | `grok version` |
| Project inspect | `grok inspect` |
| MCP manage | `grok mcp ...` |
| Plugins / marketplace | `grok plugin ...` |
| Update CLI | `grok update` |
| Config | `~/.grok/config.toml` |
| Auth store | `~/.grok/auth.json`（只读状态，不提交） |
| Sessions store | `~/.grok/sessions` |

## Permission modes

| Desktop label | CLI flag / mode |
|---------------|-----------------|
| Plan | `--permission-mode plan` / plan mode |
| Ask | default |
| Accept Edits | `--permission-mode acceptEdits` |
| Always Approve | `--always-approve` |

## Out of band (Desktop-only)

| Concern | Owner |
|---------|-------|
| Window layout, zoom, theme | Desktop local prefs |
| Recent projects | Desktop local prefs |
| App auto-update | Tauri updater |
| App logs | Desktop log dir |
| Min compatible CLI version | Desktop constant + diagnostics |
