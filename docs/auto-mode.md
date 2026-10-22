# Auto mode: `access auto`

**`worker.access auto` starts the agent in its program's own auto mode**: the program checks each action itself, does
the safe ones without asking and stops the risky ones. It sits between the two values the board always had:

| `<role>.access` | The agent | Claude Code | Codex |
|---|---|---|---|
| `full` (built in) | does anything, without asking | `--dangerously-skip-permissions` | `--dangerously-bypass-approvals-and-sandbox` |
| `auto` | does what its own check finds safe, stops the rest | `--permission-mode auto` | `--approve-for-me` |
| `ask` | asks you before it acts | its own default | its own default |

`full` and `ask` are as they always were, and `full` stays the built-in value.

The same value is there for every role that has an access: `worker.access`, `reviewer.access` and `coordinator.access`.
A research task is a worker's, and takes `worker.access`. Set it as any setting: `herdr-projects set worker.access auto`,
for one task `herdr-projects delegate <id> --worker.access auto`, or on the task panel's settings page, where the access
row goes `full`, `ask`, `auto` in turn. The task panel says `auto mode` under the agent.

## Which programs have one

Five: Claude Code, Codex, Cursor, Qwen Code and Qoder. The table of programs says each one's flags and first version in
its `Auto mode` column ([agents](agents.md)).

- **Claude Code**: `claude --permission-mode auto`, from Claude Code 2.1.111 (2.1.71 to 2.1.110 took the value only
  with a hidden `--enable-auto-mode` as well, and 2.1.70 and older quit on it). A classifier model reads each action
  before it runs: what stays in the repo goes ahead, what could do harm outside it (a download run as a script, a force
  push, a production change) is stopped, and Claude is told why and tries another way. Where auto mode cannot be had
  (a model it does not support, `disableAutoMode` in Claude Code's settings, or turned off for the account), Claude
  Code starts in its default mode instead, which asks: never in `full`.
- **Codex**: `codex --approve-for-me`, from Codex 0.147.0. It is Codex's auto review: in its workspace-write sandbox
  the agent works in the repo freely, and a request to go beyond it is answered by Codex's own reviewer instead of by
  you. After 3 refusals in a row, or 10 of the last 50, it stops its turn with a warning.
- **Cursor** `cursor-agent --auto-review` (from 2026.06.22), **Qwen Code** `qwen --approval-mode auto` (from 0.16.0)
  and **Qoder** `qodercli --permission-mode auto` (from 0.2.14): each its own classifier, as its documentation says.
  These three were read from their documentation, not tried here.

Some programs have a flag called `auto` that is not this: `kilo --auto`, `opencode --auto`, `kimi --auto` and
MastraCode's `--permission-mode auto` approve everything, as `full` does. Hermes and Grok have a checked mode in their
own settings files but no flag to start in it. None of these is given `auto`.

Every other program has no such mode, and `auto` is refused for it, never made `full` or `ask` behind your back:

- **When you set it**: `worker.agent: pi cannot do "auto": it has no auto mode (only claude, codex, cursor, qodercli,
  qwen have one)`, and the same for `worker.access auto` while the role's program is one without it.
- **Written by hand in a settings file**, where another place names such a program: the value stays `auto`, unlike an
  effort or an `ask` that does not fit (which fall to the next place down), because the next place down could be
  `full`. The task panel says `no auto mode in pi: it will not start`, the agent is not started, and the task says why:
  `pi cannot be started with access "auto": it has no auto mode (…). Set the role's access to full or ask, or choose
  another program.` `herdr-projects doctor` names every such setting.
- **On the task panel's settings page** the access row passes over a value its program cannot do: for OpenCode it goes
  `full`, `ask`, `full`; for pi, which can do neither, it stays `full`.
- **A fallback agent** without one is passed over while the access is `auto`, as one that cannot ask is while it is
  `ask` ([the fallback agent](fallback.md)).
- **A task that plans first** is refused before the plan, not after you approved it.

## An older version

The board reads the program's version (`claude --version`, `codex --version`, as it does for everything else it checks)
and starts nothing in auto mode on a version older than the first that has it. The task says so instead:

```
claude cannot be started with access "auto": claude 2.1.70 has no auto mode: it came in claude 2.1.111. Update claude, or set the role's access to full or ask.
```

`herdr-projects doctor` says the same before any task starts. A version that cannot be read is taken to be new: the
program then says itself what it does not know.

## When the agent stops for you

Auto mode is not `full`: where its own check will not decide, the program asks, and that is a permission prompt on its
screen like any other. Claude Code shows its prompt again after 3 actions stopped in a row, or 20 in all, and for
anything your own `permissions.ask` rules name. Nothing about it is new to the board:

- **A step its check stops** does not stop the agent: it is told why and goes on another way. When that leaves it
  needing you (it wants a script from the internet run, say), it says so as every agent does, with the board's
  `report … --state blocked --kind permission`, and the task waits for you with its reason: `REPO-8 · Need OK to
  download and run the install script from example.com`. Answer it in the chat ([an agent that waits for
  you](waiting-for-you.md)).
- **A permission prompt on its screen** (after the stops above, or a rule of your own) is a dialog like any other: Herdr
  says the agent is blocked, the task shows under "needs you", and the chat says `The agent shows a dialog on its screen
  that waits for your answer: alt+o opens it.` Press `Alt+O` to go into the agent and answer it there.

Tried on 2026-10-19 with Claude Code 2.1.303 on Herdr, on a scratch board: a task with `worker.access auto` started
as `claude --permission-mode auto`, showed `auto mode on` at its foot, did its work and merged; one told by its check
to run `curl … | sh` had the download stopped (`denied by the Claude Code auto mode classifier … [Code from
External]`) and waited for an OK through the board, as above.
