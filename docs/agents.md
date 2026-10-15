# Agent programs

Every setting that picks a program for a role takes every program herdr can start and has its own integration for:
`worker.agent`, `reviewer.agent`, `coordinator.agent`, `watcher.agent`, `plan.agent`, `worker.fallback`,
`reviewer.fallback` and `attempts.by`. That is Claude Code, Codex and pi, and Antigravity CLI (`agy`), GitHub
Copilot CLI (`copilot`), Cursor Agent (`cursor`), Devin (`devin`), Factory's Droid (`droid`), Grok (`grok`), Hermes
(`hermes`), Kilo Code (`kilo`), Kimi Code (`kimi`), Letta Code (`letta`), MastraCode (`mastracode`), oh-my-pi (`omp`),
OpenCode (`opencode`), Qoder (`qodercli`) and Qwen Code (`qwen`). A model goes after a slash, as for the others:

```
herdr-projects set reviewer.agent agy/gemini-3-pro
herdr-projects set worker.fallback "codex, copilot/gpt-5.5"
herdr-projects delegate 12 --worker.agent qwen --reviewer.agent cursor
```

herdr's integration name and the program's own name are taken too (`antigravity-cli` is `agy`, `cursor-agent` is
`cursor`). A program is started as herdr starts it (`herdr agent start --kind <name>`), with the board's own
arguments after it; herdr finds it in its pane and says whether it works or waits. The table at the end says, for
each program, how it is started and what it can do. It is made from the code (`crates/core/src/agents/kinds/`), so it
is never other than what the board does: adding a program is one more entry there.

## What the board reads, and what it does not

The board reads the conversation Claude Code, Codex and pi save, and so knows what they said, what went wrong with
the provider and how many tokens they used ([how work gets done](how-work-gets-done.md)). It reads no other
program's: there each of these comes from elsewhere.

- **A turn's end** is herdr's: the program was at work and is at rest again (its `completion_seq`). What the agent
  said is its **closing note**, the file the board tells every agent to write when it stops; one that stops without
  one waits as `Stopped without a summary`, with the last words on its screen.
- **A review** ends with the reviewer's verdict file, as for every program. A reviewer that stops without one ends
  its round as soon as herdr sees it stop (15 seconds later), not after the review's whole time.
- **The chat** with the agent shows the words on its screen, not its conversation ([the board](board.md)).
- **Not there**: tokens are not counted (`limit.tokens` says `does nothing` for it), a provider's passing error is not
  told from a lasting one (so the board does not wait and try again, and the fallback does not take over for it; a
  logged-out screen still hands over), and helpers the agent starts are not followed.
- **Started again** (after you stopped it, or after a plan's yes), it reopens its conversation where it can: by the id
  herdr's integration for it reports (install it: `herdr integration install <name>`), or by the id the board gave it
  at start (copilot, grok, qodercli and qwen take one). Without either it starts a new conversation with the task and
  what it said last.
- **The standing rules** go in its first prompt, except for a program that takes a rules file at start (droid, omp).

## What is refused, and why

A program that cannot do a role well is refused when the setting is set, with the reason and the programs that
can:

- **MastraCode works on no task** (`worker`, `reviewer`, `coordinator`, `plan`, the fallbacks and attempts): it can
  be told neither its model nor to work without asking when it starts (only through environment variables, which
  herdr's start cannot set), and it reopens its folder's last conversation, so a reviewer would join the worker's.
  It can be the Watcher.
- **The Watcher** needs a headless one-shot (a command that answers once and ends). Every program here has one; a
  program without one would be refused.
- **A model** where the program takes none when started for work: Droid (choose its model in Droid's own settings;
  its one-shot takes one, so `watcher.agent droid/<model>` is taken).
- **A model name** the program does not take, with the names it does: read from the program's own list where it has
  one (`codex debug models`, `pi --list-models`, `cursor-agent models`, `opencode models`), else from its own rule
  (Claude Code). `herdr-projects models` lists them ([models and effort](models.md)).
- **An effort** the program does not have: a level not on its list, or any level where it takes none at start (most
  of herdr's others). Cursor takes its effort in the model's name: `cursor/claude-opus-4-8[effort=high]`.
- **`access ask`** for a program that never asks before it acts (pi).

Two more are said when the program is started, not when it is set:

- **omp and MastraCode need herdr's own integration**: without it herdr cannot tell whether they work. The board does
  not install it into your agent's settings; it says to: `omp needs herdr's own integration, or herdr cannot see when
  it works: run herdr integration install omp, then try again.`
- **A program herdr cannot see**: Kilo Code and Qoder installed with npm run behind a small node launcher that starts
  the real program as its child, and herdr 0.9.3 does not find them in the pane. The start then fails with `herdr did
  not see kilo come up in its pane …`. Install the program's standalone build instead (tried: Kilo's own binary is
  found at once).

## Before a program's first task

Start it once yourself, in a terminal, in a folder of a repo: log in, finish its first setup, and trust the folder
if it asks. herdr reads a login page as a program at rest, so a program that is not logged in is typed into like one
that is (Cursor, Devin, Grok, agy and Letta all came up at a login or a setup page when tried here). A folder-trust
dialog of a new worktree is a dialog like any other: the task waits for you (`needs you`), and Claude Code and Codex
are told beforehand to trust it ([how work gets done](how-work-gets-done.md)).

## Not here

herdr also starts Gemini CLI, Cline, Kiro, Amp, Maki and Muse, for which it has no integration (it calls them less
tested): they are not taken. Each is one more entry in the table.

## The programs

<!-- made:agents Made by `herdr-projects docs` from crates/core/src/agents/kinds/. Do not edit between these two lines. -->
| Program | Started for work as | Model | Effort | Asking first | Standing rules | Reopened by | Watcher's one-shot | Turn's end read from | herdr's integration | Can do | Checked |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `claude` | `claude --dangerously-skip-permissions` | `--model <model>`; `default`, `fable`, `opus`, `sonnet`, `haiku`, `opusplan` or a name with `claude-` in it | `--effort <level>`: low, medium, high, xhigh, max | its own default | `--append-system-prompt-file <file>` | `--resume <id>` | `claude -p --no-session-persistence --tools ""`, the prompt on its input | its conversation file | `claude` | every role | everything the board does, on 2.1.296 to 2.1.300 |
| `codex` | `codex --dangerously-bypass-approvals-and-sandbox --no-daemon` | `-m <model>`; the names `codex debug models` lists | `-c model_reasoning_effort="<level>"`: minimal, low, medium, high, xhigh | `--no-daemon` | `-c developer_instructions=…` | `resume <id>` | `codex exec --skip-git-repo-check --ephemeral --color never <prompt>` | its conversation file | `codex` | every role | everything the board does, on 0.153.4 to 0.162.1 |
| `pi` | `pi` | `--model <model>`; the names `pi --list-models` lists | `--thinking <level>`: off, minimal, low, medium, high, xhigh | cannot ask | `--append-system-prompt <file>` | `--session-id <id>` | `pi -p --no-session --no-tools --no-skills --no-context-files -- <prompt>` | its conversation file | `pi` | every role | everything the board does, on 1.1.0 |
| `agy` (also `antigravity-cli`, `antigravity`) | `agy --dangerously-skip-permissions` | `--model <model>`; not checked | `--effort <level>`: low, medium, high, xhigh, max | its own default | in its first prompt | `--conversation <id>` | `agy --disable-slash-commands -p <prompt>` | herdr's state and its closing note | `antigravity-cli` | every role | flags from agy --help 1.3.6; started by herdr 0.9.3 on the board's flags (it came up at its login) |
| `copilot` | `copilot --yolo` | `--model <model>`; not checked | `--reasoning-effort <level>`: none, minimal, low, medium, high, xhigh, max | its own default | in its first prompt | `--resume=<id>` | `copilot -s --no-custom-instructions --no-ask-user --allow-all-tools -p <prompt>` | herdr's state and its closing note | `copilot` | every role | flags from copilot --help 1.0.98; started by herdr 0.9.3 on the board's flags (its folder-trust dialog) |
| `cursor` (also `cursor-agent`) | `cursor-agent --yolo` | `--model <model>`; the names `cursor-agent models` lists | cursor takes its effort in the model's name: cursor/claude-opus-4-8[effort=high] | its own default | in its first prompt | `--resume <id>` | `cursor-agent -p --output-format text --mode ask --trust <prompt>` | herdr's state and its closing note | `cursor` | every role | flags from cursor-agent --help 2026.10.01; started by herdr 0.9.3 on the board's flags, given a task |
| `devin` | `devin --permission-mode dangerous` | `--model <model>`; not checked | none | its own default | in its first prompt | `--resume <id>` | `devin --respect-workspace-trust false -p <prompt>` | herdr's state and its closing note | `devin` | every role | flags from devin --help 3000.11.92; started by herdr 0.9.3 on the board's flags (it came up at its login) |
| `droid` | `droid --auto high` | none at start | none | its own default | `--append-system-prompt-file <file>` | `--resume <id>` | `droid exec <prompt>` | herdr's state and its closing note | `droid` | every role | flags from droid --help 0.235.0; a task on 0.242.0 in herdr 0.9.3: started, prompted and its turn's end seen |
| `grok` | `grok --always-approve` | `-m <model>`; not checked | `--reasoning-effort <level>` (any level) | its own default | in its first prompt | `--resume <id>` | `grok -p <prompt>` | herdr's state and its closing note | `grok` | every role | flags from grok --help 1.0.52; started by herdr 0.9.3 on the board's flags (it came up at its login) |
| `hermes` | `hermes chat --yolo` | `-m <model>`; not checked | none | its own default | in its first prompt | `--resume <id>` | `hermes --ignore-rules -z <prompt>` | herdr's state and its closing note | `hermes` | every role | flags from hermes --help 0.19.0; started by herdr 0.9.3 on the board's flags (its first setup) |
| `kilo` | `kilo --auto` | `-m <model>`; not checked | none | its own default | in its first prompt | `--session <id>` | `kilo run <prompt>` | herdr's state and its closing note | `kilo` | every role | flags from kilo --help 7.8.12; started by herdr 0.9.3 on the board's flags as its own program (npm's launcher hides it from herdr) |
| `kimi` | `kimi --auto` | `-m <model>`; not checked | none | its own default | in its first prompt | `--session <id>` | `kimi -p <prompt>` | herdr's state and its closing note | `kimi` | every role | flags from kimi --help 2.2.0; started by herdr 0.9.3 on the board's flags (its folder-trust dialog) |
| `letta` | `letta --new --yolo` | `-m <model>`; not checked | none | its own default | in its first prompt | `--conversation <id>` | `letta --ephemeral -p <prompt>` | herdr's state and its closing note | `letta` | every role | flags from letta --help 0.34.15; started by herdr 0.9.3 on the board's flags (its first setup held it) |
| `mastracode` | `mastracode` | none at start | none | its own default | in its first prompt | starts afresh | `mastracode --permission-mode deny --prompt <prompt>` | herdr's state and its closing note | `mastracode`, needed | Watcher only: mastracode can be told neither its model nor to work without asking when it starts (only through environment variables, which herdr's start cannot set), and it reopens its folder's last conversation, so it cannot work on a task: it can be the Watcher | flags from mastracode's own source 0.47.0 |
| `omp` | `omp --auto-approve` | `--model=<model>`; not checked | `--thinking=<level>`: off, minimal, low, medium, high, xhigh, max, auto | `--approval-mode=write` | `--append-system-prompt=<file>` | `--resume=<id>` | `omp -p --no-session --no-tools --no-skills --no-rules <prompt>` | herdr's state and its closing note | `omp`, needed | every role | flags from omp --help 18.11.1; started by herdr 0.9.3 on the board's flags (its first setup) |
| `opencode` | `opencode --auto` | `-m <model>`; the names `opencode models` lists | none | its own default | in its first prompt | `--session <id>` | `opencode run <prompt>` | herdr's state and its closing note | `opencode` | every role | flags from opencode --help 1.18.34; started by herdr 0.9.3 on the board's flags |
| `qodercli` | `qodercli --dangerously-skip-permissions` | `-m <model>`; not checked | `--reasoning-effort <level>` (any level) | its own default | in its first prompt | `--resume <id>` | `qodercli -p --no-session-persistence --tools "" -- <prompt>` | herdr's state and its closing note | `qodercli` | every role | flags from qodercli --help 1.1.72; npm's launcher hid it from herdr 0.9.3 |
| `qwen` | `qwen --yolo` | `-m <model>`; not checked | none | its own default | in its first prompt | `--resume <id>` | `qwen -p <prompt>` | herdr's state and its closing note | `qwen` | every role | flags from qwen --help 0.25.2; started by herdr 0.9.3 on the board's flags |
<!-- /made:agents -->
