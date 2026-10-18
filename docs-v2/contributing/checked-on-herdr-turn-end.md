# Herdr 0.9.3 facts: the turn end and the conversation files

This page holds the facts that the turn-end hook of the board uses: the hooks of each agent program, its conversation
files and its launch flags. Each entry is one fact, with what a real test showed. For the list of all facts, see
[what was checked on Herdr 0.9.3](herdr-0.9.3-facts.md).

The facts on this page, and on the pages about [the sidebar](checked-on-herdr-sidebar.md) and
[terminal sizes and tabs](checked-on-herdr-tabs.md), are from the second part of the notebook. That part was checked on
2026-10-09 for the turn-end hook and the keeper (Herdr 0.9.3, Claude Code 2.1.296, Codex 0.160.0, pi 1.1.0), in a
separate scratch Herdr. Where an entry gives its own date or versions, those apply to that entry.

## Claude Code

### Claude `--settings <file with Stop and StopFailure hooks> --session-id <uuid>` through `herdr agent start`

- The hooks fire at the end of each turn. The input names the session, the transcript file and the last message.

### `claude --settings` adds to the hooks of the user

- Yes. A project-level Stop hook and the hook of the flag both ran. (The own settings of the user hold no hooks to
  lose.)

### A hook that fails, is noisy or is slow

- It does not change the reply.
- A slow hook delays the end of the turn, unless it is marked `"async": true`. The plugin marks it so.

### Claude resumes a known session id (`--resume <id>`)

- Yes, with the same id and the same file.

### The conversation file of Claude

- It is `~/.claude/projects/<folder>/<id>.jsonl`.
- A finished message has `stop_reason` `end_turn`. A pending message has `tool_use`.
- An API error or a login error is an entry with `isApiErrorMessage` and `error` (`authentication_failed`,
  `model_not_found`).
- An interrupt (Esc) is a user entry "[Request interrupted by user]", and it fires no hook.
- Each message has the token use and the model.
- A usage limit was not reproduced.

## Codex

### Codex obeys `-c notify=[…]` at launch

- Yes, headless and interactive. The JSON argument carries the thread id and the last message.
- It also reports side conversations (its conversation that names the chat). The board tells these apart by their
  prompt.
- It also reports the turns of the sub-agents that a Codex agent starts (`spawn_agent`, `followup_task`; seen in
  0.162.1 on 2026-10-15). Their reports carry the first prompt of that agent. The board tells these apart by their
  conversation file, whose first line names a `parent_thread_id`.
- The own file of the agent marks the start and the end of a sub-agent as `SubAgentActivity` items.
- An interrupt fires nothing. It is `turn_aborted` in the file.
- The flag would replace a `notify` of the user. Thus a user who has one gets no hook.

### The conversation file of Codex

- It is `~/.codex/sessions/<date>/rollout-…-<id>.jsonl`.
- It has `task_complete` with the last message, `token_count` with the context window, and `turn_context` with the
  model.
- `codex resume <id>` continues the same file.

### The fast mode of Codex

- `-c service_tier="fast"` works on `codex`, `codex resume` and `codex exec` (0.160.0). Each request then carries
  `service_tier: "priority"` and the routing hint `tier=priority`. This was seen with Codex pointed at a local server,
  and in its trace log. Without the flag, the field is absent.
- `"priority"` does the same. Codex drops an unknown tier with a warning. `-c service_tier="default"` after a fast
  override sends no tier and prints no warning.
- `codex debug models` lists the tier for each model (`gpt-6.1-sol`: "Fast, 2x speed, increased usage").
- The reply of the server still names the tier `default`. Thus the speed was measured. The same essay of 1,800 tokens
  took 81 to 87 s in fast mode, and 102 to 107 s without it, three runs each.
- In a reviewer that the board started, the own log of Codex (`logs_2.sqlite`) names the tier of the session
  `priority` with `reviewerFast` on, and `default` with it off.

  **Note:** `reviewerFast` is now an old key. The board moves its value to the setting `reviewer.fast` when it reads
  a config file of the old shape (`crates/core/src/store/settings/migrate.rs`).
- A review mostly runs commands and reads code, so it gains little. The same commit, reviewed in the two modes, took
  212 s fast and 226 s normal. The review of a small task took 66 s and 69 s.

## pi

### pi `-e <file>`

- The extension receives `agent_settled`.
- The session file is `~/.pi/agent/sessions/<folder>/<time>_<id>.jsonl`, with `stopReason` and `errorMessage` for each
  message.
- `--session-id` makes an exact id, or opens it again.

## All three programs

### A turn that the provider cut off, in the three files

Seen on 2026-10-12, in real conversations on this machine.

- Codex: `task_complete` with `last_agent_message` null and `error: {message, codex_error_info}`. Seen with
  "Selected model is at capacity" (`server_overloaded`), a 401 and a 400 (both `other`). No separate `error` event
  came before it.
- pi: `stopReason: "error"` with `errorMessage`. Seen with a closed socket, "servers are currently overloaded",
  gateway 502/503/507, a stream that ended early, an unknown provider and a flagged request.
- Claude Code: an `isApiErrorMessage` entry with `error` and `apiErrorStatus`. Seen only with `model_not_found` (404).
- Not reproduced: the entry of Claude Code for an overloaded server or a rate limit; a usage limit in any of the
  three; the other `codex_error_info` values of Codex. Until the board sees those Codex values, it tells them by
  status and words.
- The trimmed endings are the input of the tests (`tests/fixtures/endings`).

### A launch flag for standing rules

- The flags are Claude `--append-system-prompt-file`, pi `--append-system-prompt <file>` and Codex
  `-c developer_instructions=…`. All three kept a rule over the session.
- Herdr refuses a launch argument that holds a line break. This is why the rules are in files.
- The board uses these flags for working agents and reviewers. The rules of the Coordinator are a file in its folder
  in place of a flag.

### A rules file in the start folder (the rules of the Coordinator)

- Tried for real, with tools off, in a folder that held only the files.
- Claude Code 2.1.296 reads `CLAUDE.md`, and an `AGENTS.md` beside it by itself. With a `CLAUDE.md` that holds only
  `@AGENTS.md`, it got `AGENTS.md` one time.
- Codex 0.160.0 (`codex exec`) and pi 1.1.0 (`pi -p --no-tools`) read `AGENTS.md`.

### A conversation after its folder was moved

- Claude Code 2.1.296 keys saved conversations by the real path of the folder.
- The folder was moved, and the old path was made a link to it. Then `claude --resume <id>` from there found the
  conversation and continued in it. From then on, it saved under the new path.

### User-level skills folders

- `~/.claude/skills`, `~/.codex/skills` and `~/.pi/agent/skills` exist.
- Claude Code listed the how-to of the plugin as a skill immediately after it was written.

## Related pages

- [What was checked on Herdr 0.9.3](herdr-0.9.3-facts.md)
- [Herdr 0.9.3 facts: agents and their start](checked-on-herdr-agents.md)
- [Provider errors](../explanation/provider-errors.md)
- [Models](../reference/models.md)
- [Agent programs](../reference/agent-programs.md)
