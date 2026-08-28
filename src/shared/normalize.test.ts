import { describe, expect, it } from "vitest";
import type { AgentStreamEvent } from "./types";

/** Lightweight fixture: ensure event discriminant stays stable for UI. */
function isStreamEvent(v: unknown): v is AgentStreamEvent {
  if (!v || typeof v !== "object" || !("type" in v)) return false;
  const t = (v as { type: string }).type;
  return [
    "message_delta",
    "thought_delta",
    "tool_call",
    "tool_update",
    "plan",
    "permission_request",
    "turn_complete",
    "error",
  ].includes(t);
}

describe("AgentStreamEvent shape", () => {
  it("accepts message_delta", () => {
    const ev = { type: "message_delta", text: "hi" };
    expect(isStreamEvent(ev)).toBe(true);
  });

  it("rejects unknown type", () => {
    expect(isStreamEvent({ type: "nope" })).toBe(false);
  });
});
