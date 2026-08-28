/** Domain events from Rust ACP layer (mirrors docs/spec.md). */

export type ConnectionStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "restarting"
  | "error";

export type ConnectionStatusPayload = {
  status: ConnectionStatus;
  message?: string | null;
  sessionId?: string | null;
  cliPath?: string | null;
};

export type CliStatus = {
  installed: boolean;
  path?: string | null;
  version?: string | null;
  error?: string | null;
  installHint: string;
};

export type AgentStreamEvent =
  | { type: "message_delta"; text: string }
  | { type: "thought_delta"; text: string }
  | {
      type: "tool_call";
      id: string;
      kind: string;
      title: string;
      input?: unknown;
    }
  | {
      type: "tool_update";
      id: string;
      status: string;
      output?: unknown;
    }
  | { type: "plan"; entries: unknown }
  | {
      type: "permission_request";
      request_id: string;
      tool_call_id: string;
      summary: string;
    }
  | { type: "turn_complete" }
  | { type: "error"; message: string };

export type ChatRole = "user" | "assistant" | "system";

export type ChatMessage = {
  id: string;
  role: ChatRole;
  content: string;
  thought?: string;
  streaming?: boolean;
};
