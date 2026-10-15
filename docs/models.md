# Models and effort

**Pick a model by writing it after the program, with a slash: `codex/gpt-6.1-sol`.** Leave it out and the program uses
its own default. Effort (how hard the agent thinks) is a setting of its own. A wrong name or level is refused when you
set it, with the ones that work.

```
herdr-projects models            every program, a few names each
herdr-projects models codex      every name codex takes, written as a setting writes them
```

Every agent the board starts has a **program**, and if you want, a **model** and an **effort**. You choose them per role:
the working agent (`worker.`), the reviewer (`reviewer.`), the Coordinator (`coordinator.`), the plan's writer (`plan.`),
the Watcher (`watcher.`), and the lists that name agents (`worker.fallback`, `reviewer.fallback`, `attempts.by`).

## Examples

```
herdr-projects set worker.agent codex/gpt-6.1-sol          you, for every task
herdr-projects set worker.effort high
herdr-projects set reviewer.agent claude/opus --repo my-app  one repo
herdr-projects delegate 12 --worker.agent pi/anthropic/claude-opus-5-5 --worker.effort medium   one task
herdr-projects set worker.fallback "codex, claude/sonnet"
herdr-projects coordinator new --agent claude/sonnet --effort low   the next Coordinator only
```

Or ask the Coordinator in your own words ("do this one with Opus on high effort", "let Codex review it"): it looks the
names up with `herdr-projects models` and sets them the same way. Where a value comes from when several places set it:
[settings](settings.md).

## A model

The model travels with its program, so a Claude model is never handed to Codex. The name is **the program's own**, and
each program names its models its own way:

| Program | Written as | Its names |
|---|---|---|
| `claude` | `claude/opus`, `claude/claude-opus-5-5`, `claude/claude-opus-5-5[1m]` | a short name (`default`, `fable`, `opus`, `sonnet`, `haiku`, `opusplan`: each the newest of its kind) or one of its full names (`herdr-projects models claude`) |
| `codex` | `codex/gpt-6.1-sol` | what `codex debug models` lists |
| `pi` | `pi/anthropic/claude-opus-5-5` (its provider first), `pi/anthropic/claude-opus-5-5:high` | what `pi --list-models` lists |
| `cursor` | `cursor/claude-opus-5-5-high`, `cursor/claude-opus-4-8[effort=high]` | what `cursor-agent models` lists; a listed model's name without its last part takes parameters in brackets |
| `opencode` | `opencode/anthropic/claude-opus-5-5` (its provider first) | what `opencode models` lists |
| `droid`, `mastracode` | no model | chosen in the program's own settings |
| the others | `agy/gemini-3-pro` | not checked: the names the program's own help gives |

### A wrong name is refused when you set it

Not found out later, when the agent fails to start. The answer says what the program takes, and the nearest name:

```
$ herdr-projects set worker.agent codex/gpt-5.5
worker.agent: codex has no model "gpt-5.5". It takes gpt-6.1-sol, gpt-6-astra, gpt-6-sol, … (from `codex debug models`).
Write it after the program: codex/gpt-6.1-sol.
```

- **The list is asked of the program**, with the account you are logged in with, and kept for an hour. A name that is not
  on it is asked again first, so a model the program has just added is taken at once.
- **When the list cannot be read** (the program is installed but not logged in, offline, or its list command fails), the
  name is refused too, with what the program said. Run its list command in a terminal to fix that, or leave the model
  out.
- **Claude Code has no command that lists its models**, so its names are written in the board's table of programs. A
  Claude model newer than that table is refused until the board is updated. Its short name (`claude/opus`) always means
  the newest of its kind.

**A name is not checked** (it goes to the program as you wrote it) when:

- the program has no list and no names written in the table;
- the program is not installed here;
- the program is pointed at a provider of its own: Claude Code with `ANTHROPIC_BASE_URL` or a cloud
  (`CLAUDE_CODE_USE_BEDROCK`, `…_VERTEX`, `…_FOUNDRY`), or Codex with a `model_provider` in its `config.toml`.

## An effort

`<role>.effort` is how hard the agent thinks: `default` (the program's own) or a level, such as `low`, `medium` or `high`.

- **Each program has its own levels** ([the table in agents](agents.md#the-programs)): Claude Code `low` to `max`, Codex
  `minimal` to `xhigh`, pi `off` to `xhigh`.
- A level the role's program does not have is refused, with the ones it has. A program that takes no effort at start
  takes only `default`.
- Cursor takes its effort in the model's name (`cursor/claude-opus-4-8[effort=high]`). pi also takes one after its model
  (`:high`).
- An effort set for one program, and kept when you choose another, is left out at start if the new one does not have it.
- `<role>.fast` is Codex's faster service tier. Elsewhere it does nothing.

## A new program

How a program's models are listed and read is one entry in its row of the table of programs
(`crates/core/src/agents/kinds/`, `models`): the command that lists them and its format, or its short and full names. A
program with neither is not checked.
