# Auto mode: `access auto`

**`worker.access auto` starts the agent in its program's own auto mode**: the program checks each action itself, does
the safe ones without asking and stops the risky ones. It sits between the two values the board always had:

| `<role>.access` | The agent | Claude Code | Codex |
|---|---|---|---|
| `full` (built in) | does anything, without asking | `--dangerously-skip-permissions` | `--dangerously-bypass-approvals-and-sandbox` |
| `auto` | does what its own check finds safe, stops the rest | `--permission-mode auto` | `--approve-for-me` |
| `ask` | asks you before it acts | its own default | its own default |

The same value is there for every role that has an access: `worker.access`, `reviewer.access` and `coordinator.access`.
A research task is a worker's, and takes `worker.access`. Set it as any setting: `herdr-projects set worker.access auto`,
for one task `herdr-projects delegate <id> --worker.access auto`, or on the task panel's settings page, where the access
row goes `full`, `ask`, `auto` in turn. The panel's agent line says `auto mode: acts on what it checks is safe`.

## Which programs have one

Only Claude Code and Codex: the table of programs says it in its `Auto mode` column ([agents](agents.md)).

- **Claude Code**: `claude --permission-mode auto`. A classifier model reads each action before it runs: what stays in
  the repo goes ahead, what could do harm outside it (a download run as a script, a force push, a production change) is
  stopped, and Claude is told why and tries another way. From Claude Code CLAUDE_FIRST.
  CLAUDE_CAVEATS
- **Codex**: `codex --approve-for-me`. Its own reviewer answers each request for approval, in its workspace-write
  sandbox: the agent writes in the repo freely, and what needs more is put to that reviewer instead of to you. From
  Codex CODEX_FIRST.

Every other program has no such mode, and `auto` is refused for it, never made `full` or `ask` behind your back:

- **When you set it**: `worker.agent: pi cannot do "auto": it has no auto mode (only claude, codex have one)`, and
  the same for `worker.access auto` while the role's program is one without it.
- **Written by hand in a settings file**, where another place names such a program: the value stays `auto`, unlike an
  effort or an `ask` that does not fit (which fall to the next place down), because the next place down could be
  `full`. The agent is then not started, and the task says why: `pi cannot be started with access "auto": it has no
  auto mode (only claude, codex have one). Set the role's access to full or ask, or choose another program.`
  `herdr-projects doctor` names every such setting.
- **A fallback agent** without one is passed over while the access is `auto`, as one that cannot ask is while it is
  `ask` ([the fallback agent](fallback.md)).

## An older version

The board reads the program's version (`claude --version`, `codex --version`, as for everything else it checks) and
starts nothing in auto mode on a version older than the first that has it. The task says so instead:

```
claude cannot be started with access "auto": claude 2.1.50 has no auto mode: it came in claude CLAUDE_FIRST. Update claude, or set the role's access to full or ask.
```

`herdr-projects doctor` says the same before any task starts. A version that cannot be read is taken to be new: the
program then says itself what it does not know.

## When the agent stops for you

Auto mode is not `full`: where its own check will not decide, the program asks, and that is a permission prompt on its
screen like any other. Nothing about it is new to the board:

PROMPT_FACTS
