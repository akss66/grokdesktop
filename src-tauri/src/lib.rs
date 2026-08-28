mod acp;
mod cli;

use acp::{AcpClient, ConnectionStatus, ConnectionStatusPayload};
use cli::CliStatus;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, State};

struct AppState {
    client: Mutex<Option<AcpClient>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectResult {
    session_id: String,
    cli_path: String,
}

#[tauri::command]
fn detect_cli() -> CliStatus {
    cli::detect_cli()
}

#[tauri::command]
fn get_connection_status(state: State<'_, AppState>) -> ConnectionStatusPayload {
    let guard = state.client.lock().unwrap();
    match guard.as_ref() {
        Some(c) => ConnectionStatusPayload {
            status: ConnectionStatus::Connected,
            message: Some("已连接".into()),
            session_id: Some(c.session_id().to_string()),
            cli_path: Some(c.cli_path().to_string_lossy().into_owned()),
        },
        None => ConnectionStatusPayload {
            status: ConnectionStatus::Disconnected,
            message: Some("未连接".into()),
            session_id: None,
            cli_path: None,
        },
    }
}

#[tauri::command]
fn connect_agent(
    app: AppHandle,
    state: State<'_, AppState>,
    cwd: Option<String>,
) -> Result<ConnectResult, String> {
    let cwd = match cwd {
        Some(c) if !c.trim().is_empty() => PathBuf::from(c),
        _ => std::env::current_dir().map_err(|e| e.to_string())?,
    };

    // Tear down any previous session first.
    {
        let mut guard = state.client.lock().unwrap();
        if let Some(mut old) = guard.take() {
            old.shutdown();
        }
    }

    let client = AcpClient::connect(app, cwd).map_err(|e| e.to_string())?;
    let result = ConnectResult {
        session_id: client.session_id().to_string(),
        cli_path: client.cli_path().to_string_lossy().into_owned(),
    };

    *state.client.lock().unwrap() = Some(client);
    Ok(result)
}

#[tauri::command]
fn disconnect_agent(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.client.lock().unwrap();
    if let Some(mut client) = guard.take() {
        client.shutdown();
    }
    Ok(())
}

#[tauri::command]
fn send_prompt(state: State<'_, AppState>, text: String) -> Result<(), String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("消息不能为空".into());
    }
    let guard = state.client.lock().unwrap();
    let client = guard
        .as_ref()
        .ok_or_else(|| "尚未连接 Agent，请先点击连接".to_string())?;
    client.send_prompt(&text).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            client: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            detect_cli,
            get_connection_status,
            connect_agent,
            disconnect_agent,
            send_prompt,
        ])
        .setup(|app| {
            // Best-effort initial status for UI listeners.
            let _ = app;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
