use serde::{Deserialize, Serialize};

/// Connection lifecycle for the status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Restarting,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatusPayload {
    pub status: ConnectionStatus,
    pub message: Option<String>,
    pub session_id: Option<String>,
    pub cli_path: Option<String>,
}

/// Normalized domain events for the React UI (see docs/spec.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentStreamEvent {
    MessageDelta { text: String },
    ThoughtDelta { text: String },
    ToolCall {
        id: String,
        kind: String,
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        input: Option<serde_json::Value>,
    },
    ToolUpdate {
        id: String,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<serde_json::Value>,
    },
    Plan {
        entries: serde_json::Value,
    },
    PermissionRequest {
        request_id: String,
        tool_call_id: String,
        summary: String,
    },
    TurnComplete,
    Error {
        message: String,
    },
}

pub const EVENT_STATUS: &str = "acp://status";
pub const EVENT_STREAM: &str = "acp://stream";
