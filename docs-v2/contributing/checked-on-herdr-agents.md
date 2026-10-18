# Herdr 0.9.3 facts: agents and their start

This page holds the facts about Herdr 0.9.3 that the board uses to start agents, send them prompts and read their
status. Each entry is one fact, with what a real test showed. For the list of all facts, see
[what was checked on Herdr 0.9.3](herdr-0.9.3-facts.md).

## Events and status

### Plugin event hooks on `pane.agent_status_changed`

- Yes, they fire. The event carries the pane id, the workspace id, the status and the agent.

### `agent start`, and `state_change_seq` and `completion_seq` in `agent get`

- Yes, they work. `completion_seq` appears after the first finished turn.

### `agent_session` in `agent get`

- It is there only when the own integration of Herdr for that agent is installed. Without the integration, a
  restarted agent starts fresh.

### Workspace and pane ids across a server restart

- The workspace ids and the pane ids survive. `terminal_id` does not survive. Without an integration, the agents are
  gone.

### Which event fires when an agent exits to its shell

- `pane.agent_detected` (released), and a status change to `unknown`.

### Codex after a reply

- Codex reported `done` here. The closing note also covers `unknown`.

## Hooks and processes

### A process spawned detached from a hook

- The process survives the exit of the hook. Herdr did not stop a hook that ran for 150 seconds.

### `PATH` in processes that a hook spawns

- It is the environment of the Herdr server, which can lack `~/.local/bin`. Thus the plugin asks a login shell.

### `worktree create --cwd <repo>` with no workspace open for the repo

- It works. Herdr also opens the repo itself as a workspace. That workspace stays when the worktree is removed, so the
  plugin closes it.

### Claude and Codex in a brand-new worktree with full access

- Both show a trust dialog, one time for each repository. Trust given before the start removes the dialog.

### `herdr machine list --json` with a saved machine

Seen on 2026-10-13.

- Each entry has `id`, `label`, `target`, `session` (`default`), `enabled` and `selected`. `target` is
  `ssh://user@host:port`, as given to `herdr machine add`.
- Herdr keeps the list in `$XDG_STATE_HOME/herdr/client/endpoints.json`, not in the config folder.
- `herdr machine add` refuses a remote server that was started as `herdr server` in the foreground
  ("not ready for saved machines"). It accepts a server that was started the normal way.

## Prompts

### When `agent start` says "ready"

Seen on 2026-10-12, with claude 2.1.296, codex 0.160.0 and pi 1.1.0.

- `agent start` says "ready" when the program is the foreground process of the pane. This is about 3.5 s after the
  command is typed, whatever is on the screen.
- A program held to 8% of a processor had drawn nothing at that time. Claude drew its screen 13 to 17 s after the
  start, and pi after 19 s. Codex had drawn its screen.

### What `agent prompt` types

Seen by a program that logs its input.

- `agent prompt` types the text as one bracketed paste, then a lone enter 300 ms later.
- `pane run` sends the text and the enter in one write.
- `pane send-text` sends the text bare.

### A prompt typed into Claude before it has drawn its screen

- Claude keeps the text and shows it in its box, but the enter is lost.
- The status stays `idle`, and `state_change_seq` does not move, for as long as nobody presses enter (watched for
  60 s).
- `agent send-keys <pane> enter` then starts the prompt.
- On the same starved program, the same prompt typed after the screen was drawn went through on its own enter. Herdr
  showed `working` 12 s later.

### A prompt typed into a running Claude at 3% of a processor, or with paste and enter in one write

- The prompt goes through. Only a program that is still starting drops the enter.

### A message typed into Claude in the middle of a long turn

Tried on 2026-10-13, in a scratch session inside tmux, with a real claude 2.1.296 on haiku. The turn was a 30 s wait,
then 16 commands of 29 KB of output each.

- Claude queues the message and reads it between two steps of the turn. The status and the counters of Herdr do not
  move for it.
- The saved conversation of Claude gets `queue-operation` lines when the message is queued and when it is taken out.
  The words themselves are an `attachment` of type `queued_command`, never a `user` entry.
- The attachment stood 1.04 MB before the end of the turn (3.5 MB on task 175).
- The board as it was then read only the last 768 KB. When the agent rested, the board pressed enter three times. It
  said "Not started: its instructions sit unsent in its box", but the work was done.
- The board as it is now removed its mark 15 s after the message was typed, while the agent still worked. The task
  ended as done (see [handing over](../explanation/handing-over.md)).

### A long start command or message

Tried on 2026-10-15, with Herdr 0.9.3, a scratch Herdr, Codex 0.162.1 and Claude Code 2.1.301.

- `agent start` types the command into the shell of the pane.
- A 7 KB `-c developer_instructions=…` went through when the shell was at its prompt.
- It was cut off at 4096 bytes when the shell still ran its start-up file. The shell then waits at `>`, and
  `agent start` answers "timed out waiting for agent startup". A short command came up in the same slow shell.
- The interrupt character (`pane send-text <pane> $'\x03'`) clears the half-typed command.
- If Codex gets `developer_instructions` that only name a rules file, it reads the file before its first answer and
  obeys it.
- `agent prompt` typed messages of 4 KB, 22 KB and 111 KB into idle Claude Code and Codex. Each program took the whole
  message on its own enter.
- A message of 186 KB cannot go to the `herdr` command at all ("Argument list too long"). Linux takes at most 128 KiB
  in one argument.

## Agent programs and versions

### Codex before 0.156.0 under `herdr agent start`

Seen on 2026-10-15, with Herdr 0.9.3 and codex 0.153.4 to 0.156.0 from npm.

- 0.155.1 and older have no `--no-daemon`. 0.156.0 is the first version that has it.
- An older codex prints `error: unexpected argument '--no-daemon' found` and quits. The pane is back at its shell.
  `agent start` waits until its `--timeout` ends, and answers `timeout`, "timed out waiting for agent startup".
- Started without the flag, 0.153.4 comes up in the pane and shows its update dialog (`agent_not_ready`, blocked).

### The other programs of Herdr under `herdr agent start --kind <name> -- <the board's flags>`

Seen on 2026-10-15, with Herdr 0.9.3, in a private session. Each program had its own version: agy 1.3.6, copilot
1.0.98, cursor-agent 2026.10.01, devin 3000.11.92, droid 0.242.0, grok 1.0.52, hermes 0.19.0, kimi 2.2.0, omp 18.11.1,
opencode 1.18.34, qwen 0.25.2.

- Each program came up in about 3.5 s, and none quit because of an argument (see
  [agent programs](../reference/agent-programs.md)).
- None was logged in or set up. Herdr read a login page or a first-setup page as `idle` (agy, cursor, devin, grok,
  hermes, omp). It also read the folder-trust dialogs of droid and kimi as `idle`. It read the dialog of copilot as
  `blocked`.

### kilo 7.8.12 and qodercli 1.1.72 installed with npm

Seen in the same session.

- Their command is a node script that starts the real program as its child. Herdr does not find the real program.
  `agent start` waited until its timeout ended, with the program running in the pane.
- The own binary of kilo, on the PATH as `kilo`, came up in 3.4 s.
- Herdr found Letta Code 0.34.15 (`unknown`). Its first-run choice of backend held it past the timeout.

### omp 18.11.1 without the integration of Herdr for it

- Herdr finds it and says `idle` whatever it does. The docs of Herdr say that its state needs the integration. Thus the
  board does not start omp without the integration.

### A task on droid 0.242.0 through the board

Seen with droid as worker and reviewer, in the same session.

- The board started droid with `--auto high --append-system-prompt-file <rules>`. Its screen said
  `Auto (High) · allow all commands`.
- The first prompt went in. The `done` of Herdr, with a new `completion_seq`, ended the turn. The board took the turn
  as stopped without a summary.
- The reason: the account had no Droid subscription, so no model answered.

## Related pages

- [What was checked on Herdr 0.9.3](herdr-0.9.3-facts.md)
- [Herdr 0.9.3 facts: the turn end and the conversation files](checked-on-herdr-turn-end.md)
- [Agent programs](../reference/agent-programs.md)
- [Agent programs, explained](../explanation/agent-programs.md)
- [Handing over](../explanation/handing-over.md)
