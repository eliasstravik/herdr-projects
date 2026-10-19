# Agent programs

**Every role can use any program Herdr can start and whose state Herdr can read.** Claude Code, Codex and pi do
everything the board does. Freebuff and CodeBuddy use their installed user plugins to report state; the others work
through Herdr's native integrations or pane detection, with a few limits (below).

```
herdr-projects set reviewer.agent agy/gemini-3-pro
herdr-projects set worker.fallback "codex, copilot/gpt-5.5"
herdr-projects delegate 12 --worker.agent qwen --reviewer.agent cursor
```

A model goes after a slash ([models and effort](models.md)). The settings that pick a program are `worker.agent`,
`reviewer.agent`, `coordinator.agent`, `watcher.agent`, `plan.agent`, `worker.fallback`, `reviewer.fallback` and
`attempts.by`.

The programs: Claude Code (`claude`), Codex (`codex`), Freebuff (`freebuff`), CodeBuddy (`codebuddy`), Cline (`cline`),
Gemini CLI (`gemini`), Cursor Agent (`cursor`), OpenCode (`opencode`), Kiro CLI (`kiro`), pi (`pi`), Antigravity CLI
(`agy`), GitHub Copilot CLI (`copilot`), Devin (`devin`), Factory's Droid (`droid`), Grok (`grok`), Hermes (`hermes`),
Kilo Code (`kilo`), Kimi Code (`kimi`), Letta Code (`letta`), MastraCode (`mastracode`), oh-my-pi (`omp`), Qoder
(`qodercli`) and Qwen Code (`qwen`). Herdr's integration name and the program's own name work too (`antigravity-cli` is `agy`,
`cursor-agent` is `cursor`).

Freebuff and CodeBuddy start as interactive terminal processes and rely on their Herdr plugins to report state. Those
plugins must be installed and enabled for Herdr Projects to track completion and wait for prompts. Gemini CLI and Kiro
CLI use Herdr's pane support. `worker.agent` chooses the primary CLI and `worker.fallback` is the ordered global
failover list. Failover starts when Herdr Projects recognizes the provider or allowance failure in the CLI's report.

## Before a program's first task

**Start it once yourself**, in a terminal, in a folder of a repo: log in, finish its first setup, and trust the folder
if it asks.

Why: Herdr reads a login page as a program at rest, so the board types the task into a logged-out program as if it were
ready. Cursor, Devin, Grok, agy and Letta all came up at a login or setup page when tried here.

A folder-trust dialog in a new worktree is a dialog like any other: the task waits for you (`needs you`). Claude Code
and Codex are told beforehand to trust it ([how work gets done](how-work-gets-done.md)).

If a start still fails, the task says why: [when an agent could not start](could-not-start.md).

## How a program is started

As Herdr starts it (`herdr agent start --kind <name>`), with the board's own arguments after it. Herdr finds it in its
pane and says whether it works or waits.

The table at the end says, for each program, how it is started and what it can do. It is made from the code
(`crates/core/src/agents/kinds/`), so it always matches what the board does. Adding a program is one more entry there.

## Helpers (subagents)

An agent on a task uses its program's own helpers (Claude Code's subagents, Codex's spawned agents, opencode's tasks,
and skills that use them) as it would outside the board. The board's arguments limit no tool, and its rules say only
that the work is not handed on to another task or agent on the board: helpers are named as allowed, also when the task
or a skill asks for them. Told only "never hand it to anyone else", Claude and Codex took that as no helpers and
refused them even when the task asked (HP-53). A test holds both: `tests/agent_helpers.rs`.

## What the board knows about each program

The board reads the conversations Claude Code, Codex and pi save. So it knows what they said, what went wrong with the
provider, and how many tokens they used ([how work gets done](how-work-gets-done.md)).

It reads no other program's conversation. For those, each of these comes from somewhere else:

- **A turn's end** comes from Herdr: the program was at work and is at rest again (its `completion_seq`).
- **What the agent said** is its **closing note**, the file the board tells every agent to write when it stops. One
  that stops without it waits as `Stopped without a summary`, with the last words on its screen.
- **A review** ends with the reviewer's verdict file, as for every program. A reviewer that stops without one ends its
  round as soon as Herdr sees it stop (15 seconds later), not after the review's whole time.
- **The chat** with the agent shows the words on its screen, not its conversation ([the board](board.md)).
- **Started again** (after you stopped it, or after a plan's yes), it reopens its conversation where it can: by the id
  Herdr's integration reports (install it: `herdr integration install <name>`), or by an id the board gave it at start
  (copilot, grok, qodercli and qwen take one). Without either it starts a new conversation with the task and what it
  said last.
- **The standing rules** go in its first prompt, except for a program that takes a rules file at start (droid, omp).

What these programs do **not** get:

- Tokens are not counted (`limit.tokens` says `does nothing` for them).
- A provider's passing error is not told from a lasting one in an unread conversation. For interactive CLIs without a
  transcript reader, the board checks explicit error markers in the terminal only after Herdr reports idle or done;
  recognized errors enter the same retry and fallback flow. A logged-out screen still hands over.
- Helpers the agent starts are not followed.

## What is refused, and why

A program that cannot do a role well is refused when you set it, with the reason and the programs that can:

- **MastraCode works on no task** (`worker`, `reviewer`, `coordinator`, `plan`, the fallbacks and attempts). It can be
  told neither its model nor to work without asking when it starts (only through environment variables, which Herdr's
  start cannot set). And it reopens its folder's last conversation, so a reviewer would join the worker's. It can be
  the Watcher.
- **The Watcher** needs a headless one-shot: a command that answers once and ends. Programs in the interactive CLI
  pool deliberately have no Watcher one-shot; they always run in visible terminal panes.
- **A model** for a program that takes none when started for work: Droid. Choose its model in Droid's own settings. Its
  one-shot does take one, so `watcher.agent droid/<model>` works.
- **A model name** the program does not take, with the names it does. They are read from the program's own list where
  it has one (`codex debug models`, `pi --list-models`, `cursor-agent models`, `opencode models`), else from the names
  written in the table (Claude Code). `herdr-projects models` lists them ([models and effort](models.md)).
- **An effort** the program does not have: a level not on the list the program gives (Codex's catalog, for the model;
  the others' `--help`; the table's where it cannot be read), or any level where it takes none at start (most of
  Herdr's others). Cursor takes its effort in the model's name: `cursor/claude-opus-4-8[effort=high]`.
- **`access ask`** for a program that never asks before it acts (pi).

Two more are said when the program starts, not when it is set:

- **omp and MastraCode need Herdr's own integration**, or Herdr cannot tell whether they work. The board does not
  install it into your agent's settings. It tells you to: `omp needs herdr's own integration, or herdr cannot see when it works: run herdr integration install omp, then try again.`
- **A program Herdr cannot see.** Kilo Code and Qoder installed with npm run behind a small node launcher that starts
  the real program as its child, and Herdr 0.9.3 does not find them in the pane. The start fails with `herdr did not see kilo come up in its pane …`. Install the program's standalone build instead (tried: Kilo's own binary is found at
  once).

## Not supported

Herdr also starts Gemini CLI, Cline, Kiro, Amp, Maki and Muse, but has no integration for them (it calls them less
tested). The board does not take them. Each would be one more entry in the table.

## The programs

<!-- made:agents Made by `herdr-projects docs` from crates/core/src/agents/kinds/. Do not edit between these two lines. -->
| Program | Started for work as | Model | Effort | Asking first | Standing rules | Reopened by | Watcher's one-shot | Turn's end read from | herdr's integration | Can do | Checked |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `claude` | `claude --dangerously-skip-permissions` | `--model <model>`; `default`, `fable`, `opus`, `sonnet`, `haiku`, `opusplan` or one of its full names in the table (`claude-fable-5-1`, `claude-fable-5`, `claude-opus-5-5`) | `--effort <level>`; the levels `claude --help` lists, else low, medium, high, xhigh, max | its own default | `--append-system-prompt-file <file>` | `--resume <id>` | none | its conversation file | `claude` | work, review, Coordinator, plan only: claude has no headless one-shot mode, so it cannot be the Watcher. | everything the board does, on 2.1.296 to 2.1.300 |
| `codex` | `codex --dangerously-bypass-approvals-and-sandbox --no-daemon` | `-m <model>`; the names `codex debug models` lists | `-c model_reasoning_effort="<level>"`; the levels `codex debug models` lists for the model, else low, medium, high, xhigh, max, ultra | `--no-daemon` | `-c developer_instructions=…` | `resume <id>` | none | its conversation file | `codex` | work, review, Coordinator, plan only: codex has no headless one-shot mode, so it cannot be the Watcher. | everything the board does, on 0.153.4 to 0.162.1 |
| `pi` | `pi` | `--model <model>`; the names `pi --list-models` lists | `--thinking <level>`; the levels `pi --help` lists, else off, minimal, low, medium, high, xhigh, max | cannot ask | `--append-system-prompt <file>` | `--session-id <id>` | `pi -p --no-session --no-tools --no-skills --no-context-files -- <prompt>` | its conversation file | `pi` | every role | everything the board does, on 1.1.0 |
| `agy` (also `antigravity-cli`, `antigravity`) | `agy --dangerously-skip-permissions` | `--model <model>`; not checked | `--effort <level>`; the levels `agy --help` lists, else low, medium, high, xhigh, max | its own default | in its first prompt | `--conversation <id>` | none | herdr's state and its closing note | `antigravity-cli` | work, review, Coordinator, plan only: agy has no headless one-shot mode, so it cannot be the Watcher. | flags from agy --help 1.3.6; started by herdr 0.9.3 on the board's flags (it came up at its login) |
| `copilot` | `copilot --yolo` | `--model <model>`; not checked | `--reasoning-effort <level>`; the levels `copilot --help` lists, else none, minimal, low, medium, high, xhigh, max | its own default | in its first prompt | `--resume=<id>` | `copilot -s --no-custom-instructions --no-ask-user --allow-all-tools -p <prompt>` | herdr's state and its closing note | `copilot` | every role | flags from copilot --help 1.0.98; started by herdr 0.9.3 on the board's flags (its folder-trust dialog) |
| `cursor` (also `cursor-agent`) | `cursor-agent --yolo` | `--model <model>`; the names `cursor-agent models` lists | cursor takes its effort in the model's name: cursor/claude-opus-4-8[effort=high] | its own default | in its first prompt | `--resume <id>` | none | herdr's state and its closing note | `cursor` | work, review, Coordinator, plan only: cursor has no headless one-shot mode, so it cannot be the Watcher. | interactive flags from cursor-agent --help 2026.10.01; headless mode excluded |
| `freebuff` | `freebuff` | none at start | none | its own default | in its first prompt | `--continue <id>` | none | herdr's state and its closing note | none | work, review, Coordinator, plan only: freebuff has no headless one-shot mode, so it cannot be the Watcher. | interactive flags from freebuff --help; Herdr state supplied by the user's installed Freebuff plugin |
| `codebuddy` | `codebuddy --dangerously-skip-permissions` | `--model <model>`; not checked | none | its own default | in its first prompt | `--resume <id>` | none | herdr's state and its closing note | none | work, review, Coordinator, plan only: codebuddy has no headless one-shot mode, so it cannot be the Watcher. | interactive flags from codebuddy --help; Herdr state supplied by the user's installed CodeBuddy plugin |
| `cline` | `cline --tui --auto-approve true` | `--model <model>`; not checked | `--thinking <level>`; the levels `cline --help` lists, else none, low, medium, high, xhigh | `--auto-approve false` | in its first prompt | `--id <id>` | none | herdr's state and its closing note | none | work, review, Coordinator, plan only: cline has no headless one-shot mode, so it cannot be the Watcher. | interactive TUI, model, effort, permission, and session flags from cline --help; Herdr supports Cline pane detection |
| `gemini` | `gemini --yolo` | `--model <model>`; not checked | none | its own default | in its first prompt | `--resume <id>` | none | herdr's state and its closing note | none | work, review, Coordinator, plan only: gemini has no headless one-shot mode, so it cannot be the Watcher. | interactive start and resume from Gemini CLI reference; Herdr 0.9.3 supports Gemini CLI panes |
| `kiro` | `kiro-cli chat --trust-all-tools` | none at start | `--effort <level>`; the levels `kiro-cli --help` lists, else low, medium, high, xhigh, max | its own default | in its first prompt | `--resume-id <id>` | none | herdr's state and its closing note | none | work, review, Coordinator, plan only: kiro has no headless one-shot mode, so it cannot be the Watcher. | interactive start, effort, and resume from Kiro CLI reference; Herdr supports Kiro CLI panes |
| `devin` | `devin --permission-mode dangerous` | `--model <model>`; not checked | none | its own default | in its first prompt | `--resume <id>` | `devin --respect-workspace-trust false -p <prompt>` | herdr's state and its closing note | `devin` | every role | flags from devin --help 3000.11.92; started by herdr 0.9.3 on the board's flags (it came up at its login) |
| `droid` | `droid --auto high` | none at start | none | its own default | `--append-system-prompt-file <file>` | `--resume <id>` | `droid exec <prompt>` | herdr's state and its closing note | `droid` | every role | flags from droid --help 0.235.0; a task on 0.242.0 in herdr 0.9.3: started, prompted and its turn's end seen |
| `grok` | `grok --always-approve` | `-m <model>`; not checked | `--reasoning-effort <level>`; the levels `grok --help` lists, else any level | its own default | in its first prompt | `--resume <id>` | none | herdr's state and its closing note | `grok` | work, review, Coordinator, plan only: grok has no headless one-shot mode, so it cannot be the Watcher. | interactive flags from grok --help 1.0.52; headless mode excluded |
| `hermes` | `hermes chat --yolo` | `-m <model>`; not checked | none | its own default | in its first prompt | `--resume <id>` | `hermes --ignore-rules -z <prompt>` | herdr's state and its closing note | `hermes` | every role | flags from hermes --help 0.19.0; started by herdr 0.9.3 on the board's flags (its first setup) |
| `kilo` | `kilo --auto` | `-m <model>`; not checked | none | its own default | in its first prompt | `--session <id>` | `kilo run <prompt>` | herdr's state and its closing note | `kilo` | every role | flags from kilo --help 7.8.12; started by herdr 0.9.3 on the board's flags as its own program (npm's launcher hides it from herdr) |
| `kimi` | `kimi --auto` | `-m <model>`; not checked | none | its own default | in its first prompt | `--session <id>` | `kimi -p <prompt>` | herdr's state and its closing note | `kimi` | every role | flags from kimi --help 2.2.0; started by herdr 0.9.3 on the board's flags (its folder-trust dialog) |
| `letta` | `letta --new --yolo` | `-m <model>`; not checked | none | its own default | in its first prompt | `--conversation <id>` | `letta --ephemeral -p <prompt>` | herdr's state and its closing note | `letta` | every role | flags from letta --help 0.34.15; started by herdr 0.9.3 on the board's flags (its first setup held it) |
| `mastracode` | `mastracode` | none at start | none | its own default | in its first prompt | starts afresh | `mastracode --permission-mode deny --prompt <prompt>` | herdr's state and its closing note | `mastracode`, needed | Watcher only: mastracode can be told neither its model nor to work without asking when it starts (only through environment variables, which herdr's start cannot set), and it reopens its folder's last conversation, so it cannot work on a task: it can be the Watcher | flags from mastracode's own source 0.47.0 |
| `omp` | `omp --auto-approve` | `--model=<model>`; not checked | `--thinking=<level>`; the levels `omp --help` lists, else off, minimal, low, medium, high, xhigh, max, auto | `--approval-mode=write` | `--append-system-prompt=<file>` | `--resume=<id>` | `omp -p --no-session --no-tools --no-skills --no-rules <prompt>` | herdr's state and its closing note | `omp`, needed | every role | flags from omp --help 18.11.1; started by herdr 0.9.3 on the board's flags (its first setup) |
| `opencode` | `opencode --auto` | `-m <model>`; the names `opencode models` lists | none | its own default | in its first prompt | `--session <id>` | none | herdr's state and its closing note | `opencode` | work, review, Coordinator, plan only: opencode has no headless one-shot mode, so it cannot be the Watcher. | interactive flags from opencode --help 1.18.34; headless mode excluded |
| `qodercli` | `qodercli --dangerously-skip-permissions` | `-m <model>`; not checked | `--reasoning-effort <level>`; the levels `qodercli --help` lists, else any level | its own default | in its first prompt | `--resume <id>` | none | herdr's state and its closing note | `qodercli` | work, review, Coordinator, plan only: qodercli has no headless one-shot mode, so it cannot be the Watcher. | interactive flags from qodercli --help 1.1.72; headless mode excluded; Herdr pane detection needs live verification |
| `qwen` | `qwen --yolo` | `-m <model>`; not checked | none | its own default | in its first prompt | `--resume <id>` | `qwen -p <prompt>` | herdr's state and its closing note | `qwen` | every role | flags from qwen --help 0.25.2; started by herdr 0.9.3 on the board's flags |
<!-- /made:agents -->
