// herdr-projects: progress extension for Pi and OMP.
// Installed by `herdr-projects configure`, rewritten by `doctor --fix` and
// removed by `unconfigure`. It forwards the session's events to the
// `hook` command, which stays silent outside a Herdr pane, and puts what the
// hook answers in the model's context: the instructions and prompt reminder
// before the next turn, a check-in reminder after a tool result.
import { spawn } from "node:child_process";

const HOOK = "__HOOK_COMMAND__";
const TIMEOUT_MS = 10_000;

/** Runs the hook with one event on stdin; its `additionalContext`, or "". */
function run(event: Record<string, unknown>): Promise<string> {
  return new Promise((resolve) => {
    const child = spawn("/bin/sh", ["-c", HOOK], { stdio: ["pipe", "pipe", "ignore"] });
    let out = "";
    const timer = setTimeout(() => child.kill(), TIMEOUT_MS);
    child.stdout.on("data", (chunk) => (out += chunk));
    child.on("error", () => resolve(""));
    child.on("close", () => {
      clearTimeout(timer);
      try {
        resolve(JSON.parse(out).additionalContext ?? "");
      } catch {
        resolve("");
      }
    });
    child.stdin.on("error", () => {});
    child.stdin.end(JSON.stringify(event));
  });
}

export default function (pi: any) {
  // Text for the next turn: session start does not wait for the hook.
  let pending: Promise<string>[] = [];

  // Only the pane's own session reports: never a subagent (OMP) or a headless run.
  const root = (ctx: any) => ctx?.mode === "tui" && ctx?.agent?.kind !== "sub";
  const send = (ctx: any, name: string, extra: Record<string, unknown> = {}) =>
    root(ctx) ? run({ hook_event_name: name, session_id: ctx.sessionManager?.getSessionId?.() ?? "", cwd: ctx.cwd, ...extra }) : Promise.resolve("");

  pi.on("session_start", (_event: any, ctx: any) => {
    pending = [send(ctx, "SessionStart")];
  });

  pi.on("input", async (_event: any, ctx: any) => {
    pending.push(send(ctx, "UserPromptSubmit"));
  });

  pi.on("before_agent_start", async () => {
    const content = (await Promise.all(pending)).filter(Boolean).join("\n\n");
    pending = [];
    if (content) return { message: { customType: "herdr-projects", content, display: false } };
  });

  pi.on("tool_result", async (event: any, ctx: any) => {
    if (event.parentToolCallId) return;
    const text = await send(ctx, "PostToolUse", { tool_name: event.toolName, tool_input: event.input });
    if (text) return { content: [...event.content, { type: "text", text }] };
  });
}
