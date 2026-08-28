import { create } from "zustand";
import type {
  AgentStreamEvent,
  ChatMessage,
  CliStatus,
  ConnectionStatus,
  ConnectionStatusPayload,
} from "../../shared/types";

function uid(): string {
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 9)}`;
}

type ChatState = {
  connection: ConnectionStatus;
  connectionMessage: string | null;
  sessionId: string | null;
  cliPath: string | null;
  cli: CliStatus | null;
  messages: ChatMessage[];
  busy: boolean;
  panelError: string | null;
  draft: string;

  setDraft: (v: string) => void;
  setCli: (cli: CliStatus) => void;
  setPanelError: (msg: string | null) => void;
  applyStatus: (p: ConnectionStatusPayload) => void;
  addUserMessage: (text: string) => void;
  beginAssistant: () => void;
  applyStream: (ev: AgentStreamEvent) => void;
  clearMessages: () => void;
};

export const useChatStore = create<ChatState>((set, get) => ({
  connection: "disconnected",
  connectionMessage: null,
  sessionId: null,
  cliPath: null,
  cli: null,
  messages: [],
  busy: false,
  panelError: null,
  draft: "",

  setDraft: (v) => set({ draft: v }),
  setCli: (cli) => set({ cli }),
  setPanelError: (msg) => set({ panelError: msg }),

  applyStatus: (p) =>
    set({
      connection: p.status,
      connectionMessage: p.message ?? null,
      sessionId: p.sessionId ?? null,
      cliPath: p.cliPath ?? null,
      panelError:
        p.status === "error" ? (p.message ?? "连接出错") : get().panelError,
    }),

  addUserMessage: (text) =>
    set((s) => ({
      messages: [
        ...s.messages,
        { id: uid(), role: "user", content: text },
      ],
      busy: true,
    })),

  beginAssistant: () =>
    set((s) => ({
      messages: [
        ...s.messages,
        {
          id: uid(),
          role: "assistant",
          content: "",
          thought: "",
          streaming: true,
        },
      ],
    })),

  applyStream: (ev) => {
    switch (ev.type) {
      case "message_delta":
        set((s) => {
          const messages = [...s.messages];
          const last = messages[messages.length - 1];
          if (last?.role === "assistant" && last.streaming) {
            messages[messages.length - 1] = {
              ...last,
              content: last.content + ev.text,
            };
          }
          return { messages };
        });
        break;
      case "thought_delta":
        set((s) => {
          const messages = [...s.messages];
          const last = messages[messages.length - 1];
          if (last?.role === "assistant" && last.streaming) {
            messages[messages.length - 1] = {
              ...last,
              thought: (last.thought ?? "") + ev.text,
            };
          }
          return { messages };
        });
        break;
      case "tool_call":
        set((s) => {
          const messages = [...s.messages];
          const last = messages[messages.length - 1];
          if (last?.role === "assistant") {
            messages[messages.length - 1] = {
              ...last,
              content:
                last.content +
                `\n\n🔧 **${ev.title}** (\`${ev.kind}\`)\n`,
            };
          }
          return { messages };
        });
        break;
      case "tool_update":
        set((s) => {
          const messages = [...s.messages];
          const last = messages[messages.length - 1];
          if (last?.role === "assistant") {
            messages[messages.length - 1] = {
              ...last,
              content: last.content + `\n↪ 工具状态：${ev.status}\n`,
            };
          }
          return { messages };
        });
        break;
      case "error":
        set((s) => ({
          panelError: ev.message,
          messages: [
            ...s.messages,
            {
              id: uid(),
              role: "system",
              content: `错误：${ev.message}`,
            },
          ],
        }));
        break;
      case "permission_request":
        set((s) => ({
          messages: [
            ...s.messages,
            {
              id: uid(),
              role: "system",
              content: `权限请求（M0 未实现审批）：${ev.summary}`,
            },
          ],
        }));
        break;
      case "turn_complete":
        set((s) => {
          const messages = s.messages.map((m) =>
            m.streaming ? { ...m, streaming: false } : m,
          );
          return { messages, busy: false };
        });
        break;
      default:
        break;
    }
  },

  clearMessages: () => set({ messages: [], panelError: null }),
}));
