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
  pi.on("session_start", (_event: any, ctx: any) => {
    void run({ hook_event_name: "SessionStart", session_id: ctx.sessionManager?.getSessionId?.() ?? "", cwd: ctx.cwd });
  });
}
