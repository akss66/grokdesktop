import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AgentStreamEvent,
  CliStatus,
  ConnectionStatusPayload,
} from "./types";

export async function detectCli(): Promise<CliStatus> {
  return invoke<CliStatus>("detect_cli");
}

export async function getConnectionStatus(): Promise<ConnectionStatusPayload> {
  return invoke<ConnectionStatusPayload>("get_connection_status");
}

export async function connectAgent(cwd?: string): Promise<{
  sessionId: string;
  cliPath: string;
}> {
  return invoke("connect_agent", { cwd: cwd ?? null });
}

export async function disconnectAgent(): Promise<void> {
  return invoke("disconnect_agent");
}

export async function sendPrompt(text: string): Promise<void> {
  return invoke("send_prompt", { text });
}

export function onConnectionStatus(
  handler: (payload: ConnectionStatusPayload) => void,
): Promise<UnlistenFn> {
  return listen<ConnectionStatusPayload>("acp://status", (event) => {
    handler(event.payload);
  });
}

export function onAgentStream(
  handler: (payload: AgentStreamEvent) => void,
): Promise<UnlistenFn> {
  return listen<AgentStreamEvent>("acp://stream", (event) => {
    handler(event.payload);
  });
}

/** True when running inside Tauri webview. */
export function isTauri(): boolean {
  return (
    typeof window !== "undefined" &&
    // Tauri 2 injects __TAURI_INTERNALS__
    ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)
  );
}
