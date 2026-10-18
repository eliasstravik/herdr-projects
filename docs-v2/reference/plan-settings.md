# Plan settings

This page gives the facts about a plan before the build: the two settings, the plan file, what the board shows, the
messages and the read-only mount.

## The two settings

| Setting | Values | Means |
|---|---|---|
| `plan.approve` | `none` (built in), `you`, `reviewer` | `none`: no plan, the agent builds at once. `you`: the agent writes its plan and the task waits as **plan ready** for your yes. `reviewer`: the reviewer checks the plan against the task's description, and the build continues by itself when the plan passes |
| `plan.agent` | `none` (built in), or a program with an optional model after a slash (`claude/claude-opus-5-5`) | who writes the plan. `none`: the working agent. A program: a planner of its own |

Both are work settings, the same as all other work settings ([settings](settings.md)):

- The board looks them up in this order: the task, your entry for the repo, you, built in.
- You set them with `set`.
- `add --delegate` and `delegate` take them as flags, for example `--plan.approve you`.
- An [automation](automations.md) can hold them.
- `show` lists them.
- **A repo's own file cannot set them.** Thus code that an agent writes cannot turn a plan's approval on or off.

### When the board reads them

- The board reads both settings when the agent first starts on the task. If you set them on a task whose agent
  already builds, they change nothing about that run.
- The board reads `plan.approve` one more time when the plan is written, to know who says yes. If it was turned off
  in the meantime, nobody is asked and the build continues.
- A new round of a task that landed (delegated again after its clean-up) starts with a new plan.
- With [several attempts](attempts-settings.md), the board reads `attempts.by` when the build starts. Thus a count
  that you change while the plan waits counts.

## The plan ready state

With `plan.approve: you`, the task waits in a state of its own, **plan ready**.

| Where | What it shows |
|---|---|
| The board | The task is under **needs you**. The row says `Plan ready · <the plan's first words>` |
| The State line, Herdr's sidebar | `plan ready` |
| `board.notify` | It tells you, as for anything that needs you |
| The task panel, part **Plan** | Where the plan stands, who wrote it, and the plan's own words: its first lines beside the list, all of it full screen. Also **`Open plan`**, a private link that the board serves the same way as a report's link |
| The web board | The same part |

## What you can do on a plan ready task

| Action | Key or command | Result |
|---|---|---|
| Yes | The approve key (`Ctrl+L`), or `herdr-projects approve <id>` | The plan is approved as it stands, and the build starts |
| A reply | `Space`, then your words, or `herdr-projects tell <id> <text>` | The plan goes back to its writer with your words, and the writer writes it again. The task is plan ready again when the new plan is there |

## The reviewer's look at a plan

With `plan.approve: reviewer`, the reviewer (`reviewer.agent`) works in a tab named `Plan review`.

| Result | What happens |
|---|---|
| The plan passes | The build continues, and nobody is asked |
| The plan fails | The plan goes back to its writer with the reviewer's points, by itself, up to `review.rounds` times |
| The plan still fails after that | The task is plan ready. The plan's part shows what the reviewer found. The yes is yours |

- [The gate](../explanation/gate.md) decides by the same rule as for work (`review.mustFix`).
- A look at a plan is not one of the task's review rounds. The first review of the work is still `Reviewer #1`.

## After the yes

| Who | What it gets |
|---|---|
| The plan's own writer | The message `Your plan is approved`, in its conversation |
| Claude Code, when it wrote the plan | The board starts it again in the same conversation, with its usual access |
| The working agent, after a planner | The task and the plan's file, when it starts |
| The planner | Its tab closes. Its time and tokens stay on the task, in the plan's part of the task panel: `Planner used 51s · 18k in · 29 out` |
| The review of the finished work | It is told of the approved plan, and checks the work against the task and the plan. It says where the build left the plan: `Left the plan`, under the review in the task panel |

## Who writes the plan

| `plan.agent` | Writer | Where the plan goes |
|---|---|---|
| `none` | The working agent. Claude Code starts with `--permission-mode plan` in place of its access flags, and presents its plan with `ExitPlanMode` | Claude Code: the board takes the plan through a hook that it passes at launch. Every other program (Codex, pi, Herdr's others): a file in the task's own folder |
| `<program>` | A planner of its own, in the task's worktree, in a tab named **`Planner`** ([a task's tabs](tabs.md)), on its own program and model. The working agent's tab `Agent` waits beside it | [The plan file](#the-plan-file) |

- While the planner writes, it is the task's agent in every way: the row, the Watcher's line, the chat, a reply and
  the stop key are its.
- If the planner is not installed on this machine, the working agent writes the plan. The task's settings say so in
  a `Note`.

## The plan file

The plan is one file, `plan.md`, in the task's own folder on the board, never in the repo:

```
~/.herdr-projects/data/tasks/<repo>/<n>/artifacts/plan/
```

| Case | The plan is |
|---|---|
| The agent stopped and saved no file | Its last message |
| The agent saved no file and said nothing | No plan: `No plan was written`. This goes back to the agent like any failure |
| A plan went back, and its writer did not save it again | Not the file: the file holds the old plan. What its writer said last is the new plan. If it said nothing, there is no new plan |

An older file never gets the yes.

## Messages

| Message | Means |
|---|---|
| `Plan ready · <the plan's first words>` | The plan waits for your yes |
| `Your plan is approved` | What the plan's writer hears after the yes |
| `No plan was written` | The agent saved no plan and said nothing |
| `Read-only file system` (Linux), `Operation not permitted` (macOS) | A program in the plan step tried to write in the worktree or its git folder |
| `Permission denied` | Something outside the mount tried to write a read-only file of the worktree |
| `Put back  2 changes made in the repo before the yes` | The board undid changes in the task's own worktree. It keeps them in `changed-too-early.patch`, beside the plan |
| `The repo was changed before the plan was approved` | The board cannot undo a change. The plan is not shown, and the build does not start |
| `Left the plan` | Under the review in the task panel: where the build left the plan |

## The mount on Linux and on macOS

| | Linux | macOS |
|---|---|---|
| The tool | bubblewrap: `bwrap --dev-bind / / --ro-bind <worktree> <worktree> --ro-bind <git folder> <git folder>` | macOS's own sandbox: `sandbox-exec -p '<profile>'` |
| What it guards | the worktree and its git folder are mounted read-only | a Seatbelt profile allows everything except a write under the worktree and its git folder: `(version 1)(allow default)(deny file-write* (subpath "<worktree>") (subpath "<git folder>"))`, each by its real path (`/var/…` is `/private/var/…`) |
| A write there | `Read-only file system` | `Operation not permitted` |
| It needs | bubblewrap installed, and user namespaces allowed | `/usr/bin/sandbox-exec` (part of macOS), and Herdr not itself started inside a sandbox |

### The program's own sandbox on macOS

macOS allows no sandbox inside another. Thus, for the plan step, the board turns off the program's own sandbox:

| Program | What the board does |
|---|---|
| Codex | `-c sandbox_mode="danger-full-access"`. With full access, `--dangerously-bypass-approvals-and-sandbox` already turns it off. Its approvals stay as they were |
| Claude Code | `"sandbox": {"enabled": false}` in the settings that the board passes for that launch (`--settings`), in case your own settings turn its sandbox on. Its plan mode stays on |
| pi | Nothing: pi has no sandbox of its own |

After the yes, the program starts again the usual way, with its own sandbox as you set it.

### `HERDR_PROJECTS_SANDBOX`

| Value | The board uses |
|---|---|
| `bwrap` | bubblewrap |
| `sandbox-exec` | macOS's sandbox |
| `none` | No mount. No plan is written in a repo |
| Not set | The platform's own tool. The board remembers for ten minutes if it works |

## Related pages

- [Get a plan before the build](../how-to/get-a-plan-first.md)
- [A plan first](../explanation/plan.md)
- [Attempts settings](attempts-settings.md)
- [Settings](settings.md)
- [A task's tabs](tabs.md)
- [Agent programs](agent-programs.md)
