//! Line-delimited JSON-RPC 2.0 over child process stdio.

use super::events::{
    AgentStreamEvent, ConnectionStatus, ConnectionStatusPayload, EVENT_STATUS, EVENT_STREAM,
};
use crate::cli;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    #[error("{0}")]
    Message(String),
    #[error("CLI 未安装或无法启动：{0}")]
    Cli(String),
    #[error("ACP 请求超时：{0}")]
    Timeout(String),
    #[error("JSON-RPC 错误：{0}")]
    Rpc(String),
    #[error("IO：{0}")]
    Io(#[from] std::io::Error),
}

impl From<AcpError> for String {
    fn from(e: AcpError) -> Self {
        e.to_string()
    }
}

type PendingMap = Arc<Mutex<HashMap<u64, Sender<Result<Value, AcpError>>>>>;

pub struct AcpClient {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    pending: PendingMap,
    next_id: AtomicU64,
    session_id: String,
    cli_path: PathBuf,
    app: AppHandle,
}

impl AcpClient {
    /// Spawn `grok agent stdio`, initialize, and create a session at `cwd`.
    pub fn connect(app: AppHandle, cwd: PathBuf) -> Result<Self, AcpError> {
        emit_status(
            &app,
            ConnectionStatus::Connecting,
            Some("正在启动 Grok Agent…".into()),
            None,
            None,
        );

        let cli_path = cli::resolve_grok_path().ok_or_else(|| {
            AcpError::Cli("未找到 grok 可执行文件".into())
        })?;

        let mut child = Command::new(&cli_path)
            .args(["agent", "stdio"])
            .current_dir(&cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AcpError::Cli(format!("无法启动 `{} agent stdio`：{e}", cli_path.display())))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AcpError::Message("无法获取 agent stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AcpError::Message("无法获取 agent stdout".into()))?;

        // Drain stderr so the pipe never fills; surface useful lines as stream errors.
        if let Some(stderr) = child.stderr.take() {
            let app_err = app.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().flatten() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    // Avoid spamming UI with routine logs; only emit likely failures.
                    let lower = trimmed.to_ascii_lowercase();
                    if lower.contains("error")
                        || lower.contains("auth")
                        || lower.contains("unauthor")
                        || lower.contains("login")
                        || lower.contains("failed")
                    {
                        let _ = app_err.emit(
                            EVENT_STREAM,
                            AgentStreamEvent::Error {
                                message: format!("CLI: {trimmed}"),
                            },
                        );
                    }
                }
            });
        }

        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let pending_reader = Arc::clone(&pending);
        let app_reader = app.clone();
        let stdin = Arc::new(Mutex::new(stdin));
        let stdin_reader = Arc::clone(&stdin);

        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(e) => {
                        let _ = app_reader.emit(
                            EVENT_STREAM,
                            AgentStreamEvent::Error {
                                message: format!("读取 ACP stdout 失败：{e}"),
                            },
                        );
                        break;
                    }
                };
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                handle_stdout_line(&app_reader, &pending_reader, &stdin_reader, line);
            }

            emit_status(
                &app_reader,
                ConnectionStatus::Disconnected,
                Some("Agent 进程已退出".into()),
                None,
                None,
            );
        });

        let client = AcpClient {
            child,
            stdin,
            pending,
            next_id: AtomicU64::new(1),
            session_id: String::new(),
            cli_path: cli_path.clone(),
            app: app.clone(),
        };

        // initialize
        let _init = client.request(
            "initialize",
            json!({
                "protocolVersion": 1,
                "clientInfo": {
                    "name": "grok-build-desktop",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "clientCapabilities": {
                    "fs": { "readTextFile": true, "writeTextFile": false },
                    "terminal": false
                }
            }),
            Duration::from_secs(30),
        )?;

        // session/new
        let session = client.request(
            "session/new",
            json!({
                "cwd": cwd.to_string_lossy(),
                "mcpServers": []
            }),
            Duration::from_secs(30),
        )?;

        let session_id = session
            .get("sessionId")
            .or_else(|| session.get("session_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AcpError::Rpc(format!("session/new 未返回 sessionId：{session}"))
            })?
            .to_string();

        let mut client = client;
        client.session_id = session_id.clone();

        emit_status(
            &app,
            ConnectionStatus::Connected,
            Some("已连接".into()),
            Some(session_id),
            Some(cli_path.to_string_lossy().into_owned()),
        );

        Ok(client)
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn cli_path(&self) -> &PathBuf {
        &self.cli_path
    }

    /// Send `session/prompt`. Streaming chunks arrive via `acp://stream` events.
    pub fn send_prompt(&self, text: &str) -> Result<(), AcpError> {
        if self.session_id.is_empty() {
            return Err(AcpError::Message("尚未创建会话".into()));
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        self.pending.lock().unwrap().insert(id, tx);

        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "session/prompt",
            "params": {
                "sessionId": self.session_id,
                "prompt": [{ "type": "text", "text": text }]
            }
        });

        self.write_line(&msg)?;

        // Wait for the final result of this turn in a background thread so the
        // Tauri command can return immediately after the request is accepted.
        let app = self.app.clone();
        thread::spawn(move || {
            match rx.recv_timeout(Duration::from_secs(600)) {
                Ok(Ok(_)) => {
                    let _ = app.emit(EVENT_STREAM, AgentStreamEvent::TurnComplete);
                }
                Ok(Err(e)) => {
                    let _ = app.emit(
                        EVENT_STREAM,
                        AgentStreamEvent::Error {
                            message: e.to_string(),
                        },
                    );
                    let _ = app.emit(EVENT_STREAM, AgentStreamEvent::TurnComplete);
                }
                Err(_) => {
                    let _ = app.emit(
                        EVENT_STREAM,
                        AgentStreamEvent::Error {
                            message: "等待模型回复超时".into(),
                        },
                    );
                    let _ = app.emit(EVENT_STREAM, AgentStreamEvent::TurnComplete);
                }
            }
        });

        Ok(())
    }

    /// Respond to a permission request. M0: used by UI or auto-reject.
    pub fn respond_permission(
        &self,
        request_id: Value,
        allow: bool,
    ) -> Result<(), AcpError> {
        // ACP permission responses vary by version; send a common shape.
        let result = if allow {
            json!({ "outcome": { "outcome": "selected", "optionId": "allow-once" } })
        } else {
            json!({ "outcome": { "outcome": "cancelled" } })
        };

        let msg = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": result
        });
        self.write_line(&msg)
    }

    fn request(&self, method: &str, params: Value, timeout: Duration) -> Result<Value, AcpError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx): (Sender<Result<Value, AcpError>>, Receiver<_>) = mpsc::channel();
        self.pending.lock().unwrap().insert(id, tx);

        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        self.write_line(&msg)?;

        match rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                Err(AcpError::Timeout(method.into()))
            }
        }
    }

    fn write_line(&self, msg: &Value) -> Result<(), AcpError> {
        let mut stdin = self
            .stdin
            .lock()
            .map_err(|_| AcpError::Message("stdin 锁损坏".into()))?;
        let line = serde_json::to_string(msg)
            .map_err(|e| AcpError::Message(format!("序列化失败：{e}")))?;
        stdin.write_all(line.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        Ok(())
    }

    pub fn shutdown(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        emit_status(
            &self.app,
            ConnectionStatus::Disconnected,
            Some("已断开".into()),
            None,
            None,
        );
    }
}

impl Drop for AcpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn handle_stdout_line(
    app: &AppHandle,
    pending: &PendingMap,
    stdin: &Arc<Mutex<ChildStdin>>,
    line: &str,
) {
    let value: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            let _ = app.emit(
                EVENT_STREAM,
                AgentStreamEvent::Error {
                    message: format!("无法解析 ACP 消息：{e}"),
                },
            );
            return;
        }
    };

    // Response to a request
    if let Some(id_val) = value.get("id").cloned() {
        if value.get("method").is_none() {
            if let Some(id) = json_id_as_u64(&id_val) {
                let result = if let Some(err) = value.get("error") {
                    Err(AcpError::Rpc(err.to_string()))
                } else {
                    Ok(value.get("result").cloned().unwrap_or(Value::Null))
                };
                if let Some(tx) = pending.lock().unwrap().remove(&id) {
                    let _ = tx.send(result);
                }
            }
            return;
        }

        // Server request (e.g. permission) — id present + method
        if let Some(method) = value.get("method").and_then(|m| m.as_str()) {
            handle_server_request(app, stdin, &value, method, id_val);
            return;
        }
    }

    // Notification
    if let Some(method) = value.get("method").and_then(|m| m.as_str()) {
        if method == "session/update" || method.ends_with("session/update") {
            if let Some(params) = value.get("params") {
                for ev in map_session_update(params) {
                    let _ = app.emit(EVENT_STREAM, ev);
                }
            }
            return;
        }

        // Other notifications currently ignored in M0
        return;
    }
}

fn write_json_line(stdin: &Arc<Mutex<ChildStdin>>, msg: &Value) -> Result<(), AcpError> {
    let mut guard = stdin
        .lock()
        .map_err(|_| AcpError::Message("stdin 锁损坏".into()))?;
    let line = serde_json::to_string(msg)
        .map_err(|e| AcpError::Message(format!("序列化失败：{e}")))?;
    guard.write_all(line.as_bytes())?;
    guard.write_all(b"\n")?;
    guard.flush()?;
    Ok(())
}

fn handle_server_request(
    app: &AppHandle,
    stdin: &Arc<Mutex<ChildStdin>>,
    value: &Value,
    method: &str,
    id: Value,
) {
    if method.contains("permission") || method.ends_with("request_permission") {
        let params = value.get("params").cloned().unwrap_or(Value::Null);
        let tool_call_id = params
            .pointer("/toolCall/toolCallId")
            .or_else(|| params.pointer("/toolCallId"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let summary = params
            .pointer("/toolCall/title")
            .or_else(|| params.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("请求执行工具")
            .to_string();

        let _ = app.emit(
            EVENT_STREAM,
            AgentStreamEvent::PermissionRequest {
                request_id: id.to_string(),
                tool_call_id,
                summary: summary.clone(),
            },
        );

        // M0: auto-cancel so the turn does not hang without a permission UI.
        let cancel = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "outcome": { "outcome": "cancelled" }
            }
        });
        if let Err(e) = write_json_line(stdin, &cancel) {
            let _ = app.emit(
                EVENT_STREAM,
                AgentStreamEvent::Error {
                    message: format!("自动拒绝权限失败：{e}"),
                },
            );
        } else {
            let _ = app.emit(
                EVENT_STREAM,
                AgentStreamEvent::Error {
                    message: format!(
                        "已自动拒绝权限请求（M0 无审批 UI）：{summary}。完整审批将在 M1 提供。"
                    ),
                },
            );
        }
        return;
    }

    let _ = app.emit(
        EVENT_STREAM,
        AgentStreamEvent::Error {
            message: format!("未处理的服务端请求：{method}"),
        },
    );
}

fn map_session_update(params: &Value) -> Vec<AgentStreamEvent> {
    let update = params
        .get("update")
        .or_else(|| params.get("sessionUpdate").map(|_| params))
        .cloned()
        .unwrap_or_else(|| params.clone());

    // Shape A: { update: { sessionUpdate: "...", ... } }
    // Shape B: { sessionUpdate: "...", ... }
    let body = update
        .get("update")
        .cloned()
        .unwrap_or_else(|| update.clone());

    let kind = body
        .get("sessionUpdate")
        .or_else(|| body.get("session_update"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match kind {
        "agent_message_chunk" => {
            let text = extract_text_content(&body).unwrap_or_default();
            if text.is_empty() {
                vec![]
            } else {
                vec![AgentStreamEvent::MessageDelta { text }]
            }
        }
        "agent_thought_chunk" => {
            let text = extract_text_content(&body).unwrap_or_default();
            if text.is_empty() {
                vec![]
            } else {
                vec![AgentStreamEvent::ThoughtDelta { text }]
            }
        }
        "tool_call" => {
            let id = body
                .get("toolCallId")
                .or_else(|| body.get("tool_call_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let title = body
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("工具调用")
                .to_string();
            let kind = body
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("other")
                .to_string();
            let input = body.get("rawInput").or_else(|| body.get("input")).cloned();
            vec![AgentStreamEvent::ToolCall {
                id,
                kind,
                title,
                input,
            }]
        }
        "tool_call_update" => {
            let id = body
                .get("toolCallId")
                .or_else(|| body.get("tool_call_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let status = body
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let output = body
                .get("rawOutput")
                .or_else(|| body.get("content"))
                .cloned();
            vec![AgentStreamEvent::ToolUpdate {
                id,
                status,
                output,
            }]
        }
        "plan" => {
            let entries = body
                .get("entries")
                .cloned()
                .unwrap_or(Value::Array(vec![]));
            vec![AgentStreamEvent::Plan { entries }]
        }
        _ => vec![],
    }
}

fn extract_text_content(body: &Value) -> Option<String> {
    if let Some(c) = body.get("content") {
        if let Some(t) = c.get("text").and_then(|v| v.as_str()) {
            return Some(t.to_string());
        }
        if let Some(t) = c.as_str() {
            return Some(t.to_string());
        }
    }
    body.get("text")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn json_id_as_u64(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_u64().or_else(|| n.as_i64().map(|i| i as u64)),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn emit_status(
    app: &AppHandle,
    status: ConnectionStatus,
    message: Option<String>,
    session_id: Option<String>,
    cli_path: Option<String>,
) {
    let _ = app.emit(
        EVENT_STATUS,
        ConnectionStatusPayload {
            status,
            message,
            session_id,
            cli_path,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_message_chunk() {
        let params = json!({
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": "你好" }
            }
        });
        let evs = map_session_update(&params);
        assert_eq!(evs.len(), 1);
        match &evs[0] {
            AgentStreamEvent::MessageDelta { text } => assert_eq!(text, "你好"),
            _ => panic!("expected MessageDelta"),
        }
    }
}
