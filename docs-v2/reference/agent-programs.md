# Agent programs

This page lists the agent programs that the board can use, the settings that choose them, and the limits of each
program. For the reasons behind these facts, read [how the board uses agent programs](../explanation/agent-programs.md).

## Which programs a role can use

Every role can use each program that Herdr can start and that has its own Herdr integration. Claude Code, Codex and pi
do all of the work that the board does. The other programs also work, but with some limits (see
[What the other programs do not get](#what-the-other-programs-do-not-get)).

```
herdr-projects set reviewer.agent agy/gemini-3-pro
herdr-projects set worker.fallback "codex, copilot/gpt-5.5"
herdr-projects delegate 12 --worker.agent qwen --reviewer.agent cursor
```

Write a model after a slash ([models and effort](models.md)).

These settings choose a program:

- `worker.agent`
- `reviewer.agent`
- `coordinator.agent`
- `watcher.agent`
- `plan.agent`
- `worker.fallback`
- `reviewer.fallback`
- `attempts.by`

## The program names

| Program | Name to write |
|---|---|
| Claude Code | `claude` |
| Codex | `codex` |
| pi | `pi` |
| Antigravity CLI | `agy` |
| GitHub Copilot CLI | `copilot` |
| Cursor Agent | `cursor` |
| Devin | `devin` |
| Factory's Droid | `droid` |
| Grok | `grok` |
| Hermes | `hermes` |
| Kilo Code | `kilo` |
| Kimi Code | `kimi` |
| Letta Code | `letta` |
| MastraCode | `mastracode` |
| oh-my-pi | `omp` |
| OpenCode | `opencode` |
| Qoder | `qodercli` |
| Qwen Code | `qwen` |

The name of the Herdr integration and the name of the program also work. For example, `antigravity-cli` is `agy`, and
`cursor-agent` is `cursor`.

## How the board starts a program

The board starts a program in the same way as Herdr does (`herdr agent start --kind <name>`). The board adds its own
arguments after that command. Herdr then finds the program in its pane and tells whether the program works or waits.

## What the other programs do not get

The board reads the saved conversations of Claude Code, Codex and pi only. For all other programs, these items come
from a different source:

| Item | Where it comes from |
|---|---|
| The end of a turn | Herdr: the program was at work and is at rest again (its `completion_seq`). |
| What the agent said | Its **closing note**: the file that the board tells every agent to write when it stops. An agent that stops without it waits as `Stopped without a summary`, with the last words on its screen. |
| The end of a review | The verdict file of the reviewer, as for every program. A reviewer that stops without one ends its round when Herdr sees it stop (15 seconds later), not after the full time of the review. |
| The chat with the agent | The words on its screen, not its conversation ([the chat of the board](board-chat.md)). |
| A new start (after you stopped it, or after a yes to a plan) | It opens its conversation again where it can: by the id that the Herdr integration reports (install it: `herdr integration install <name>`), or by an id that the board gave it at start (`copilot`, `grok`, `qodercli` and `qwen` take one). Without an id, it starts a new conversation with the task and what it said last. |
| The standing rules | Its first prompt. The exception is a program that takes a rules file at start (`droid`, `omp`). |

These programs do not get these functions:

- The board does not count tokens. `limit.tokens` says `does nothing` for them.
- The board cannot tell a passing provider error from a lasting one. Thus the board does not wait and try again, and the
  fallback agent does not take over for that error. A logged-out screen still causes a hand-over.
- The board does not follow the helpers that the agent starts.

## What the board refuses when you set it

If a program cannot do a role well, the board refuses it when you set it. The refusal gives the reason and the
programs that can do the role.

| The board refuses | Details |
|---|---|
| MastraCode for any task | This applies to `worker`, `reviewer`, `coordinator`, `plan`, the fallbacks and attempts. MastraCode can be the Watcher. |
| A Watcher program without a headless one-shot | A headless one-shot is a command that answers one time and ends. Every program on this page has one. |
| A model for Droid | Droid takes no model when the board starts it for work. Choose its model in the settings of Droid. Its one-shot takes a model, so `watcher.agent droid/<model>` works. |
| A model name that the program does not take | The refusal gives the names that it takes. The board reads them from the list of the program where it has one (`codex debug models`, `pi --list-models`, `cursor-agent models`, `opencode models`). For Claude Code, the board reads them from the names in the table. `herdr-projects models` lists them ([models and effort](models.md)). |
| An effort that the program does not have | This is a level that is not on the list of the program (the Codex catalog, for the model; `--help` for the others; the table where the board cannot read a list). It is also any level for a program that takes no effort at start (most of the other Herdr programs). Cursor takes its effort in the model name: `cursor/claude-opus-4-8[effort=high]`. |
| `access ask` for a program that never asks before it acts | This applies to `pi`. |

## What the board says when the program starts

The board tells you about two more problems when the program starts, not when you set it:

- **`omp` and MastraCode need the Herdr integration.** Without it, Herdr cannot tell whether they work. The board does
  not install the integration into the settings of your agent. It tells you to install it:
  `omp needs herdr's own integration, or herdr cannot see when it works: run herdr integration install omp, then try again.`
- **Herdr cannot see the program.** If you install Kilo Code or Qoder with npm, they run behind a small node launcher.
  The launcher starts the real program as its child, and Herdr 0.9.3 does not find the program in the pane. The start
  fails with `herdr did not see kilo come up in its pane …`. Install the standalone build of the program. The project
  tried this: Herdr found the own binary of Kilo at once.

## Programs that the board does not support

Herdr also starts Gemini CLI, Cline, Kiro, Amp, Maki and Muse. Herdr has no integration for them, and calls them "less
tested". The board does not take them.

## The table of programs

For each program, this table tells how the board starts it and what it can do. The table comes from the code
(`crates/core/src/agents/kinds/`), so it always agrees with what the board does.

**Note:** This table is a snapshot. `herdr-projects docs` writes the original table between the `made:agents` lines in
`docs/agents.md`. If the two are different, `docs/agents.md` is correct.

| Program | Started for work as | Model | Effort | Asking first | Standing rules | Reopened by | Watcher's one-shot | Turn's end read from | herdr's integration | Can do | Checked |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `claude` | `claude --dangerously-skip-permissions` | `--model <model>`; `default`, `fable`, `opus`, `sonnet`, `haiku`, `opusplan` or one of its full names in the table (`claude-fable-5-1`, `claude-fable-5`, `claude-opus-5-5`) | `--effort <level>`; the levels `claude --help` lists, else low, medium, high, xhigh, max | its own default | `--append-system-prompt-file <file>` | `--resume <id>` | `claude -p --no-session-persistence --tools ""`, the prompt on its input | its conversation file | `claude` | every role | everything the board does, on 2.1.296 to 2.1.300 |
| `codex` | `codex --dangerously-bypass-approvals-and-sandbox --no-daemon` | `-m <model>`; the names `codex debug models` lists | `-c model_reasoning_effort="<level>"`; the levels `codex debug models` lists for the model, else low, medium, high, xhigh, max, ultra | `--no-daemon` | `-c developer_instructions=…` | `resume <id>` | `codex exec --skip-git-repo-check --ephemeral --color never <prompt>` | its conversation file | `codex` | every role | everything the board does, on 0.153.4 to 0.162.1 |
| `pi` | `pi` | `--model <model>`; the names `pi --list-models` lists | `--thinking <level>`; the levels `pi --help` lists, else off, minimal, low, medium, high, xhigh, max | cannot ask | `--append-system-prompt <file>` | `--session-id <id>` | `pi -p --no-session --no-tools --no-skills --no-context-files -- <prompt>` | its conversation file | `pi` | every role | everything the board does, on 1.1.0 |
| `agy` (also `antigravity-cli`, `antigravity`) | `agy --dangerously-skip-permissions` | `--model <model>`; not checked | `--effort <level>`; the levels `agy --help` lists, else low, medium, high, xhigh, max | its own default | in its first prompt | `--conversation <id>` | `agy --disable-slash-commands -p <prompt>` | herdr's state and its closing note | `antigravity-cli` | every role | flags from agy --help 1.3.6; started by herdr 0.9.3 on the board's flags (it came up at its login) |
| `copilot` | `copilot --yolo` | `--model <model>`; not checked | `--reasoning-effort <level>`; the levels `copilot --help` lists, else none, minimal, low, medium, high, xhigh, max | its own default | in its first prompt | `--resume=<id>` | `copilot -s --no-custom-instructions --no-ask-user --allow-all-tools -p <prompt>` | herdr's state and its closing note | `copilot` | every role | flags from copilot --help 1.0.98; started by herdr 0.9.3 on the board's flags (its folder-trust dialog) |
| `cursor` (also `cursor-agent`) | `cursor-agent --yolo` | `--model <model>`; the names `cursor-agent models` lists | cursor takes its effort in the model's name: cursor/claude-opus-4-8[effort=high] | its own default | in its first prompt | `--resume <id>` | `cursor-agent -p --output-format text --mode ask --trust <prompt>` | herdr's state and its closing note | `cursor` | every role | flags from cursor-agent --help 2026.10.01; started by herdr 0.9.3 on the board's flags, given a task |
| `devin` | `devin --permission-mode dangerous` | `--model <model>`; not checked | none | its own default | in its first prompt | `--resume <id>` | `devin --respect-workspace-trust false -p <prompt>` | herdr's state and its closing note | `devin` | every role | flags from devin --help 3000.11.92; started by herdr 0.9.3 on the board's flags (it came up at its login) |
| `droid` | `droid --auto high` | none at start | none | its own default | `--append-system-prompt-file <file>` | `--resume <id>` | `droid exec <prompt>` | herdr's state and its closing note | `droid` | every role | flags from droid --help 0.235.0; a task on 0.242.0 in herdr 0.9.3: started, prompted and its turn's end seen |
| `grok` | `grok --always-approve` | `-m <model>`; not checked | `--reasoning-effort <level>`; the levels `grok --help` lists, else any level | its own default | in its first prompt | `--resume <id>` | `grok -p <prompt>` | herdr's state and its closing note | `grok` | every role | flags from grok --help 1.0.52; started by herdr 0.9.3 on the board's flags (it came up at its login) |
| `hermes` | `hermes chat --yolo` | `-m <model>`; not checked | none | its own default | in its first prompt | `--resume <id>` | `hermes --ignore-rules -z <prompt>` | herdr's state and its closing note | `hermes` | every role | flags from hermes --help 0.19.0; started by herdr 0.9.3 on the board's flags (its first setup) |
| `kilo` | `kilo --auto` | `-m <model>`; not checked | none | its own default | in its first prompt | `--session <id>` | `kilo run <prompt>` | herdr's state and its closing note | `kilo` | every role | flags from kilo --help 7.8.12; started by herdr 0.9.3 on the board's flags as its own program (npm's launcher hides it from herdr) |
| `kimi` | `kimi --auto` | `-m <model>`; not checked | none | its own default | in its first prompt | `--session <id>` | `kimi -p <prompt>` | herdr's state and its closing note | `kimi` | every role | flags from kimi --help 2.2.0; started by herdr 0.9.3 on the board's flags (its folder-trust dialog) |
| `letta` | `letta --new --yolo` | `-m <model>`; not checked | none | its own default | in its first prompt | `--conversation <id>` | `letta --ephemeral -p <prompt>` | herdr's state and its closing note | `letta` | every role | flags from letta --help 0.34.15; started by herdr 0.9.3 on the board's flags (its first setup held it) |
| `mastracode` | `mastracode` | none at start | none | its own default | in its first prompt | starts afresh | `mastracode --permission-mode deny --prompt <prompt>` | herdr's state and its closing note | `mastracode`, needed | Watcher only: mastracode can be told neither its model nor to work without asking when it starts (only through environment variables, which herdr's start cannot set), and it reopens its folder's last conversation, so it cannot work on a task: it can be the Watcher | flags from mastracode's own source 0.47.0 |
| `omp` | `omp --auto-approve` | `--model=<model>`; not checked | `--thinking=<level>`; the levels `omp --help` lists, else off, minimal, low, medium, high, xhigh, max, auto | `--approval-mode=write` | `--append-system-prompt=<file>` | `--resume=<id>` | `omp -p --no-session --no-tools --no-skills --no-rules <prompt>` | herdr's state and its closing note | `omp`, needed | every role | flags from omp --help 18.11.1; started by herdr 0.9.3 on the board's flags (its first setup) |
| `opencode` | `opencode --auto` | `-m <model>`; the names `opencode models` lists | none | its own default | in its first prompt | `--session <id>` | `opencode run <prompt>` | herdr's state and its closing note | `opencode` | every role | flags from opencode --help 1.18.34; started by herdr 0.9.3 on the board's flags |
| `qodercli` | `qodercli --dangerously-skip-permissions` | `-m <model>`; not checked | `--reasoning-effort <level>`; the levels `qodercli --help` lists, else any level | its own default | in its first prompt | `--resume <id>` | `qodercli -p --no-session-persistence --tools "" -- <prompt>` | herdr's state and its closing note | `qodercli` | every role | flags from qodercli --help 1.1.72; npm's launcher hid it from herdr 0.9.3 |
| `qwen` | `qwen --yolo` | `-m <model>`; not checked | none | its own default | in its first prompt | `--resume <id>` | `qwen -p <prompt>` | herdr's state and its closing note | `qwen` | every role | flags from qwen --help 0.25.2; started by herdr 0.9.3 on the board's flags |

## Related pages

- [How the board uses agent programs](../explanation/agent-programs.md)
- [Models](models.md)
- [Choose a model and an effort](../how-to/choose-a-model-and-effort.md)
- [Set a fallback agent](../how-to/set-a-fallback-agent.md)
- [Fix an agent that could not start](../how-to/could-not-start-fix.md)
- [Roles](roles.md)
- [The code of the agent programs](../contributing/agents-code.md)
