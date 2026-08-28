//! ACP JSON-RPC client over `grok agent stdio`.
//!
//! Domain events are emitted to the frontend via Tauri events; UI never sees raw RPC lines.

mod events;
mod rpc;

pub use events::{AgentStreamEvent, ConnectionStatus, ConnectionStatusPayload};
pub use rpc::{AcpClient, AcpError};
