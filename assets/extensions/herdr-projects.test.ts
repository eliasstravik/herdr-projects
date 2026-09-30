// Tests for the Pi and OMP progress extension: `bun test assets/extensions`.
// The extension is loaded as `configure` writes it, with the hook command
// replaced by a fake hook that records each event and answers by event name.
import { expect, test } from "bun:test";
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const TEMPLATE = readFileSync(join(import.meta.dir, "herdr-projects.ts"), "utf8");

/** The extension as `configure` writes it, running `command` as its hook. */
async function extension(command: string) {
  const file = join(mkdtempSync(join(tmpdir(), "hp-ext-")), "extension.ts");
  writeFileSync(file, TEMPLATE.replace('"__HOOK_COMMAND__"', JSON.stringify(command)));
  const handlers: Record<string, (event: any, ctx: any) => any> = {};
  (await import(file)).default({ on: (name: string, handler: any) => (handlers[name] = handler) });
  return handlers;
}

/** A fake hook that appends each event to `events.jsonl` and answers `answers[name]`. */
async function load(answers: Record<string, string>) {
  const dir = mkdtempSync(join(tmpdir(), "hp-ext-"));
  const hook = join(dir, "hook.js");
  writeFileSync(
    hook,
    `const fs = require("fs");
     const event = JSON.parse(fs.readFileSync(0, "utf8"));
     fs.appendFileSync(${JSON.stringify(join(dir, "events.jsonl"))}, JSON.stringify(event) + "\\n");
     const text = ${JSON.stringify(answers)}[event.hook_event_name];
     if (text) process.stdout.write(JSON.stringify({ additionalContext: text }));`,
  );
  const handlers = await extension(`${process.execPath} ${hook}`);
  const log = join(dir, "events.jsonl");
  const events = () => (existsSync(log) ? readFileSync(log, "utf8").trim().split("\n").map((line) => JSON.parse(line)) : []);
  return { handlers, events };
}

const ctx = { mode: "tui", cwd: "/work", sessionManager: { getSessionId: () => "s1" } };

test("the session start instructions reach the first turn as a hidden message", async () => {
  const { handlers } = await load({ SessionStart: "report progress" });
  handlers.session_start({ reason: "startup" }, ctx);
  const result = await handlers.before_agent_start({ prompt: "hi" }, ctx);
  expect(result).toEqual({ message: { customType: "herdr-projects", content: "report progress", display: false } });
});

test("a prompt's reminder joins the next turn, and the hook sees Claude Code's event", async () => {
  const { handlers, events } = await load({ SessionStart: "report progress", UserPromptSubmit: "check in" });
  handlers.session_start({ reason: "startup" }, ctx);
  await handlers.input({ text: "hi", source: "interactive" }, ctx);
  const result = await handlers.before_agent_start({ prompt: "hi" }, ctx);
  expect(result.message.content).toBe("report progress\n\ncheck in");
  expect(events()).toContainEqual({ hook_event_name: "UserPromptSubmit", session_id: "s1", cwd: "/work" });
  // Delivered once.
  expect(await handlers.before_agent_start({ prompt: "again" }, ctx)).toBeUndefined();
});

test("a tool result's check-in reminder is appended to the tool's output", async () => {
  const { handlers, events } = await load({ PostToolUse: "check in" });
  const content = [{ type: "text", text: "ok" }];
  const result = await handlers.tool_result({ toolName: "bash", input: { command: "ls" }, content }, ctx);
  expect(result).toEqual({ content: [...content, { type: "text", text: "check in" }] });
  expect(events()).toContainEqual({ hook_event_name: "PostToolUse", session_id: "s1", cwd: "/work", tool_name: "bash", tool_input: { command: "ls" } });
});

test("a subagent or a headless run never calls the hook", async () => {
  const { handlers, events } = await load({ SessionStart: "report progress", PostToolUse: "check in" });
  for (const other of [{ ...ctx, agent: { kind: "sub" } }, { ...ctx, mode: "print" }]) {
    handlers.session_start({ reason: "startup" }, other);
    expect(await handlers.before_agent_start({ prompt: "hi" }, other)).toBeUndefined();
    expect(await handlers.tool_result({ toolName: "bash", input: {}, content: [] }, other)).toBeUndefined();
  }
  expect(events()).toEqual([]);
});

test("a nested tool call gets no reminder", async () => {
  const { handlers, events } = await load({ PostToolUse: "check in" });
  const nested = { toolName: "read", parentToolCallId: "call-1", input: {}, content: [] };
  expect(await handlers.tool_result(nested, ctx)).toBeUndefined();
  expect(events()).toEqual([]);
});

test("a missing or failing hook adds nothing and never throws", async () => {
  for (const command of ["/no/such/herdr-projects", "echo not-json", "exit 2"]) {
    const handlers = await extension(command);
    handlers.session_start({ reason: "startup" }, ctx);
    expect(await handlers.before_agent_start({ prompt: "hi" }, ctx)).toBeUndefined();
    expect(await handlers.tool_result({ toolName: "bash", input: {}, content: [] }, ctx)).toBeUndefined();
  }
});
