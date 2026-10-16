# Models and effort

**Pick a model by writing it after the program, with a slash: `codex/gpt-6.1-sol`.** Leave it out and the program uses
its own default. Effort (how hard the agent thinks) is a setting of its own. A wrong name or level is refused when you
set it, with the ones that work.

```
herdr-projects models            every program, a few names each, and its effort levels
herdr-projects models codex      every name codex takes, written as a setting writes them, and its effort levels
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
names up with `herdr-projects models` and sets them the same way. It writes only a name from that list ("sol 6.1" is
`gpt-6.1-sol`, or it asks you), and when you name no model it gives none: the program then uses its own default. Where a
value comes from when several places set it: [settings](settings.md).

## A model

The model travels with its program, so a Claude model is never handed to Codex. The name is **the program's own**, and
each program names its models its own way:

| Program | Written as | Its names |
|---|---|---|
| `claude` | `claude/opus`, `claude/claude-opus-5-5`, `claude/claude-opus-5-5[1m]` | a short name (`default`, `fable`, `opus`, `sonnet`, `haiku`, `opusplan`: each the newest of its kind) or one of its full names (`herdr-projects models claude`) |
| `codex` | `codex/gpt-6.1-sol` | what Codex lists for the account it is signed in with (its app-server's `model/list`; `codex debug models` when that cannot be asked) |
| `pi` | `pi/anthropic/claude-opus-5-5` (its provider first), `pi/anthropic/claude-opus-5-5:high` | what `pi --list-models` lists |
| `cursor` | `cursor/claude-opus-5-5-high`, `cursor/claude-opus-4-8[effort=high]` | what `cursor-agent models` lists; a listed model's name without its last part takes parameters in brackets |
| `opencode` | `opencode/anthropic/claude-opus-5-5` (its provider first) | what `opencode models` lists |
| `droid`, `mastracode` | no model | chosen in the program's own settings |
| the others | `agy/gemini-3-pro` | not checked: the names the program's own help gives |

### A wrong name is refused when you set it

Not found out later, when the agent fails to start. The answer says what the program takes, and the nearest name:

```
$ herdr-projects set worker.agent codex/sol-6.1
worker.agent: codex has no model "sol-6.1" for your ChatGPT sign-in (pro plan). Did you mean gpt-6.1-sol? It takes
gpt-6.1-sol, gpt-6-astra, gpt-6-sol, … (codex's own list for that sign-in). Write it after the program: codex/gpt-6.1-sol.
```

- **The list is asked of the program**, with the account you are logged in with, and kept for an hour. A name that is not
  on it is asked again first, so a model the program has just added is taken at once.
- **Codex's names depend on how you signed in.** A ChatGPT sign-in takes other names than an API key, and a name that is
  fine elsewhere is refused by Codex at the first turn ("The 'sol-6.1' model is not supported when using Codex with a
  ChatGPT account"). So the board asks Codex itself, through its app-server, which names this sign-in can use and which
  sign-in it is (`herdr-projects models codex` says it). Where the app-server cannot be asked (an older Codex), it reads
  `codex debug models`.
- pi, cursor and opencode list only the models their own sign-ins and keys reach, so their lists are already yours.
- **When the list cannot be read** (the program is installed but not logged in, offline, or its list command fails), the
  name is refused too, with what the program said. Run its list command in a terminal to fix that, or leave the model
  out.
- **Claude Code has no command that lists its models**, so its names are written in the board's table of programs. A
  Claude model newer than that table is refused until the board is updated. Its short name (`claude/opus`) always means
  the newest of its kind.

### A model refused while the agent works

A name can still be refused when the agent runs: one written in a settings file by hand (only a value being set is
checked), a sign-in that changed, a program on another machine, or the program's own default model. The board catches
that at the end of the turn and never leaves the task in the agent with only its error:

- **It gave the program that model:** the same program starts again at once on its own default model, in the same
  worktree, told what was done so far. The row says so: `gpt-6.1 can't be used with your ChatGPT sign-in: started again on
  codex's own default model`, and the task panel lists it under "Took over". The reviewer the same: the review starts
  again on its program's default model.
- **The default model is refused too, or it gave none:** the task waits for you, with the reason and a name to try
  (`Agent stopped: gpt-6.1 can't be used with your ChatGPT sign-in; try gpt-6.1-sol`) and the next step in the panel:
  choose one for the task (`herdr-projects set worker.agent codex/gpt-6.1-sol --task <id>`), or fix the program's own
  default (Codex: `model` in `~/.codex/config.toml`), then `Ctrl+L` (retry). The program is ended, so the retry starts it
  again on the model as it is set then.
- With a `worker.fallback` (or `reviewer.fallback`), the next on that list comes after the default model
  ([fallback](fallback.md)).

**A name is not checked** (it goes to the program as you wrote it) when:

- the program has no list and no names written in the table;
- the program is not installed here;
- the program is pointed at a provider of its own: Claude Code with `ANTHROPIC_BASE_URL` or a cloud
  (`CLAUDE_CODE_USE_BEDROCK`, `…_VERTEX`, `…_FOUNDRY`), or Codex with a `model_provider` in its `config.toml`.

## An effort

`<role>.effort` is how hard the agent thinks: `default` (the program's own) or a level, such as `low`, `medium` or `high`.

- **Each program has its own levels, and the board asks the program for them**, as it does for model names:
  - Codex: what `codex debug models` lists for each model (today `low` to `max`, and `ultra` on some). A level is checked
    against the model you chose (`codex/gpt-6-luna` has no `ultra`); with no model, against every level any model takes.
  - The others: what their `--help` lists at their effort argument (`claude --help`: `low` to `max`; `pi --help`: `off`
    to `max`).
  - The levels are kept for an hour. A level that is not on the list is asked again first, so a level the program has
    just added is taken at once.
  - **When they cannot be read** (the program is not installed here, is offline, or its help names none), the levels
    written in the board's table of programs are used ([the table in agents](agents.md#the-programs)). Never a refusal
    of every level.
- A level the role's program (or its model) does not have is refused, with the ones it has:

  ```
  $ herdr-projects set worker.effort minimal
  worker.effort: codex has no effort "minimal": it takes low, medium, high, xhigh, max, ultra, as `codex debug models` lists them (or default, for its own).
  ```

- A program that takes no effort at start takes only `default`.
- Cursor takes its effort in the model's name (`cursor/claude-opus-4-8[effort=high]`). pi also takes one after its model
  (`:high`).
- An effort set for one program, and kept when you choose another (program or model), is left out at start if the new
  one does not have it. A start goes by the levels the program said last: it never waits to ask.
- `<role>.fast` is Codex's faster service tier. Elsewhere it does nothing.

## A new program

How a program's models are listed and read is one entry in its row of the table of programs
(`crates/core/src/agents/kinds/`, `models`): the command that lists them and its format (and Codex's app-server, `server`),
or its short and full names. A
program with neither is not checked. Its effort levels are read from its `--help` unless its row says otherwise
(`efforts_read`: Codex's catalog, or only the table); its `efforts` are the levels used when they cannot be read.
