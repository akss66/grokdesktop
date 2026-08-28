import { useEffect, useRef } from "react";
import {
  connectAgent,
  detectCli,
  disconnectAgent,
  isTauri,
  onAgentStream,
  onConnectionStatus,
  sendPrompt,
} from "../../shared/api";
import { useChatStore } from "./store";
import type { ConnectionStatus } from "../../shared/types";

function statusLabel(s: ConnectionStatus): string {
  switch (s) {
    case "connected":
      return "已连接";
    case "connecting":
      return "连接中";
    case "restarting":
      return "重启中";
    case "error":
      return "错误";
    default:
      return "未连接";
  }
}

function statusDot(s: ConnectionStatus): string {
  switch (s) {
    case "connected":
      return "bg-emerald-400";
    case "connecting":
    case "restarting":
      return "bg-amber-400 animate-pulse";
    case "error":
      return "bg-rose-500";
    default:
      return "bg-zinc-500";
  }
}

export function ChatView() {
  const {
    connection,
    connectionMessage,
    sessionId,
    cliPath,
    cli,
    messages,
    busy,
    panelError,
    draft,
    setDraft,
    setCli,
    setPanelError,
    applyStatus,
    addUserMessage,
    beginAssistant,
    applyStream,
  } = useChatStore();

  const bottomRef = useRef<HTMLDivElement>(null);
  const inTauri = isTauri();

  useEffect(() => {
    if (!inTauri) {
      setPanelError(
        "当前在浏览器中预览前端。请使用 `pnpm tauri dev` 启动完整桌面应用以连接 Grok CLI。",
      );
      return;
    }

    let unsubs: Array<() => void> = [];

    (async () => {
      try {
        const status = await detectCli();
        setCli(status);
        if (!status.installed) {
          setPanelError(status.error ?? status.installHint);
        }
      } catch (e) {
        setPanelError(String(e));
      }

      try {
        unsubs.push(
          await onConnectionStatus((p) => applyStatus(p)),
          await onAgentStream((ev) => applyStream(ev)),
        );
      } catch (e) {
        setPanelError(`订阅事件失败：${e}`);
      }
    })();

    return () => {
      unsubs.forEach((u) => u());
    };
  }, [inTauri, setCli, setPanelError, applyStatus, applyStream]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, busy]);

  async function handleDetect() {
    if (!inTauri) return;
    try {
      const status = await detectCli();
      setCli(status);
      setPanelError(status.installed ? null : (status.error ?? status.installHint));
    } catch (e) {
      setPanelError(String(e));
    }
  }

  async function handleConnect() {
    if (!inTauri) return;
    setPanelError(null);
    try {
      const res = await connectAgent();
      applyStatus({
        status: "connected",
        message: "已连接",
        sessionId: res.sessionId,
        cliPath: res.cliPath,
      });
    } catch (e) {
      applyStatus({
        status: "error",
        message: String(e),
        sessionId: null,
        cliPath: null,
      });
      setPanelError(String(e));
    }
  }

  async function handleDisconnect() {
    if (!inTauri) return;
    try {
      await disconnectAgent();
      applyStatus({
        status: "disconnected",
        message: "已断开",
        sessionId: null,
        cliPath: null,
      });
    } catch (e) {
      setPanelError(String(e));
    }
  }

  async function handleSend() {
    const text = draft.trim();
    if (!text || busy) return;
    if (connection !== "connected") {
      setPanelError("请先连接 Agent");
      return;
    }

    setDraft("");
    addUserMessage(text);
    beginAssistant();
    try {
      await sendPrompt(text);
    } catch (e) {
      setPanelError(String(e));
      applyStream({ type: "turn_complete" });
      applyStream({ type: "error", message: String(e) });
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col bg-zinc-950 text-zinc-100">
      {/* Header */}
      <header className="flex shrink-0 items-center justify-between gap-4 border-b border-zinc-800 px-4 py-3">
        <div className="min-w-0">
          <h1 className="text-base font-semibold tracking-tight">
            Grok Build Desktop
          </h1>
          <p className="truncate text-xs text-zinc-500">
            M0 · ACP 流式对话骨架
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <div className="flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900 px-3 py-1 text-xs">
            <span className={`h-2 w-2 rounded-full ${statusDot(connection)}`} />
            <span>{statusLabel(connection)}</span>
            {connectionMessage ? (
              <span className="max-w-[12rem] truncate text-zinc-500">
                · {connectionMessage}
              </span>
            ) : null}
          </div>
          <button
            type="button"
            onClick={handleDetect}
            className="rounded-md border border-zinc-700 bg-zinc-900 px-3 py-1.5 text-xs hover:bg-zinc-800"
          >
            检测 CLI
          </button>
          {connection === "connected" ? (
            <button
              type="button"
              onClick={handleDisconnect}
              className="rounded-md border border-zinc-700 bg-zinc-900 px-3 py-1.5 text-xs hover:bg-zinc-800"
            >
              断开
            </button>
          ) : (
            <button
              type="button"
              onClick={handleConnect}
              disabled={!inTauri || cli?.installed === false}
              className="rounded-md bg-sky-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-sky-500 disabled:cursor-not-allowed disabled:opacity-40"
            >
              连接 Agent
            </button>
          )}
        </div>
      </header>

      {/* Meta strip */}
      <div className="flex shrink-0 flex-wrap gap-x-4 gap-y-1 border-b border-zinc-900 bg-zinc-950/80 px-4 py-2 text-[11px] text-zinc-500">
        <span>
          CLI：{" "}
          {cli?.installed
            ? `${cli.version ?? "已安装"} · ${cli.path ?? ""}`
            : "未检测到"}
        </span>
        {sessionId ? <span>Session：{sessionId}</span> : null}
        {cliPath && connection === "connected" ? (
          <span className="truncate">进程：{cliPath}</span>
        ) : null}
      </div>

      {/* Error panel */}
      {panelError ? (
        <div className="shrink-0 border-b border-rose-900/50 bg-rose-950/40 px-4 py-3 text-sm text-rose-100">
          <div className="font-medium text-rose-200">需要注意</div>
          <p className="mt-1 whitespace-pre-wrap text-rose-100/90">{panelError}</p>
          {cli && !cli.installed ? (
            <p className="mt-2 text-xs text-rose-200/70">{cli.installHint}</p>
          ) : null}
        </div>
      ) : null}

      {/* Messages */}
      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4">
        {messages.length === 0 ? (
          <div className="mx-auto mt-16 max-w-md text-center text-sm text-zinc-500">
            <p className="text-zinc-300">开始一轮对话</p>
            <p className="mt-2">
              1. 确认本机已安装并登录 Grok CLI
              <br />
              2. 点击「连接 Agent」
              <br />
              3. 发送消息，查看流式回复
            </p>
          </div>
        ) : (
          <ul className="mx-auto flex max-w-3xl flex-col gap-3">
            {messages.map((m) => (
              <li
                key={m.id}
                className={`rounded-xl border px-3.5 py-2.5 text-sm leading-relaxed ${
                  m.role === "user"
                    ? "ml-8 border-sky-900/60 bg-sky-950/40"
                    : m.role === "system"
                      ? "border-amber-900/50 bg-amber-950/30 text-amber-100"
                      : "mr-8 border-zinc-800 bg-zinc-900/80"
                }`}
              >
                <div className="mb-1 text-[10px] uppercase tracking-wider text-zinc-500">
                  {m.role === "user"
                    ? "你"
                    : m.role === "system"
                      ? "系统"
                      : "Grok"}
                  {m.streaming ? " · 生成中…" : ""}
                </div>
                {m.thought ? (
                  <details className="mb-2 rounded-md border border-zinc-800 bg-zinc-950/60 p-2 text-xs text-zinc-400">
                    <summary className="cursor-pointer select-none text-zinc-500">
                      思考过程
                    </summary>
                    <pre className="mt-2 whitespace-pre-wrap font-sans">
                      {m.thought}
                    </pre>
                  </details>
                ) : null}
                <div className="whitespace-pre-wrap break-words">
                  {m.content || (m.streaming ? "…" : "")}
                </div>
              </li>
            ))}
            <div ref={bottomRef} />
          </ul>
        )}
      </div>

      {/* Composer */}
      <form
        className="shrink-0 border-t border-zinc-800 p-3"
        onSubmit={(e) => {
          e.preventDefault();
          void handleSend();
        }}
      >
        <div className="mx-auto flex max-w-3xl gap-2">
          <textarea
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                void handleSend();
              }
            }}
            rows={2}
            placeholder={
              connection === "connected"
                ? "输入消息…（Enter 发送，Shift+Enter 换行）"
                : "连接 Agent 后输入消息…"
            }
            disabled={connection !== "connected" || busy}
            className="min-h-[2.75rem] flex-1 resize-none rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm outline-none ring-sky-600/40 placeholder:text-zinc-600 focus:ring-2 disabled:opacity-50"
          />
          <button
            type="submit"
            disabled={connection !== "connected" || busy || !draft.trim()}
            className="self-end rounded-lg bg-sky-600 px-4 py-2 text-sm font-medium text-white hover:bg-sky-500 disabled:cursor-not-allowed disabled:opacity-40"
          >
            {busy ? "…" : "发送"}
          </button>
        </div>
      </form>
    </div>
  );
}
