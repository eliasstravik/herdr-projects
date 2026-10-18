# Models and effort

This page tells how to write a model and an effort for each program, and how the board checks them. For the steps,
read [choose a model and an effort](../how-to/choose-a-model-and-effort.md).

Every agent that the board starts has a **program**. It can also have a **model** and an **effort**. You choose them for
each role:

| Role | Settings |
|---|---|
| The working agent | `worker.` |
| The reviewer | `reviewer.` |
| The Coordinator | `coordinator.` |
| The writer of the plan | `plan.` |
| The Watcher | `watcher.` |
| The lists that name agents | `worker.fallback`, `reviewer.fallback`, `attempts.by` |

## The models command

```
herdr-projects models            every program, a few names each, and its effort levels
herdr-projects models codex      every name codex takes, written as a setting writes them, and its effort levels
```

## A model

Write the model after the program, with a slash: `codex/gpt-6.1-sol`. If you do not give a model, the program uses its
own default.

The model goes with its program. Thus the board never gives a Claude model to Codex. The name is **the name that the
program uses**. Each program names its models in its own way:

| Program | Written as | Its names |
|---|---|---|
| `claude` | `claude/opus`, `claude/claude-opus-5-5`, `claude/claude-opus-5-5[1m]` | a short name (`default`, `fable`, `opus`, `sonnet`, `haiku`, `opusplan`: each the newest of its kind) or one of its full names (`herdr-projects models claude`) |
| `codex` | `codex/gpt-6.1-sol` | what Codex lists for the account it is signed in with (its app-server's `model/list`; `codex debug models` when that cannot be asked) |
| `pi` | `pi/anthropic/claude-opus-5-5` (its provider first), `pi/anthropic/claude-opus-5-5:high` | what `pi --list-models` lists |
| `cursor` | `cursor/claude-opus-5-5-high`, `cursor/claude-opus-4-8[effort=high]` | what `cursor-agent models` lists; a listed model's name without its last part takes parameters in brackets |
| `opencode` | `opencode/anthropic/claude-opus-5-5` (its provider first) | what `opencode models` lists |
| `droid`, `mastracode` | no model | chosen in the program's own settings |
| the others | `agy/gemini-3-pro` | not checked: the names the program's own help gives |

### The board refuses a wrong name when you set it

The board finds a wrong name when you set it, not later when the agent cannot start. The answer tells what the program
takes, and the nearest name:

```
$ herdr-projects set worker.agent codex/sol-6.1
worker.agent: codex has no model "sol-6.1" for your ChatGPT sign-in (pro plan). Did you mean gpt-6.1-sol? It takes
gpt-6.1-sol, gpt-6-astra, gpt-6-sol, … (codex's own list for that sign-in). Write it after the program: codex/gpt-6.1-sol.
```

- **The board asks the program for the list**, with the account that you are logged in with. It keeps the list for one
  hour. If a name is not on the list, the board asks the program again first. Thus it accepts a model that the program
  added a short time ago.
- **The names of Codex depend on how you signed in.** A ChatGPT sign-in takes different names from an API key. Codex
  refuses a name that is correct elsewhere at the first turn: "The 'sol-6.1' model is not supported when using Codex
  with a ChatGPT account". Thus the board asks Codex, through its app-server, which names this sign-in can use, and
  which sign-in it is. `herdr-projects models codex` tells you. If the board cannot ask the app-server (an older Codex),
  it reads `codex debug models`.
- pi, cursor and opencode list only the models that their own sign-ins and keys can use. Thus their lists are already
  your lists.
- **If the board cannot read the list, it refuses the name too.** It tells what the program said. This occurs when the
  program is installed but not logged in, when it is offline, or when its list command fails. To repair this, run its
  list command in a terminal, or do not give a model.
- **Claude Code has no command that lists its models.** Thus its names are in the table of programs of the board. The
  board refuses a Claude model that is newer than that table, until you update the board. The short name
  (`claude/opus`) always means the newest of its kind.

### A model that the program refuses while the agent works

The program can also refuse a name while the agent runs. This can occur in these conditions:

- The name is in a settings file that you edited by hand. The board checks only a value that you set with a command.
- The sign-in changed.
- The program is on another machine.
- The program refuses its own default model.

The board finds this at the end of the turn. It never leaves the task in the agent with only its error:

- **If the board gave the program that model**, the same program starts again immediately, on its own default model.
  It uses the same worktree, and the board tells it what was done. The row tells so:
  `gpt-6.1 can't be used with your ChatGPT sign-in: started again on codex's own default model`. The task panel lists
  it under "Took over". The same is true for the reviewer: the review starts again on the default model of its program.
- **If the program also refuses its default model, or the board gave no model**, the task waits for you. It shows the
  reason and a name to try: `Agent stopped: gpt-6.1 can't be used with your ChatGPT sign-in; try gpt-6.1-sol`. The panel
  shows the next step. The board stops the program. Thus the retry starts the program again on the model that is set
  then. The steps are in
  [choose a model and an effort](../how-to/choose-a-model-and-effort.md#if-the-program-refuses-the-model-while-the-agent-works).
- If there is a `worker.fallback` (or `reviewer.fallback`), the next agent on that list comes after the default model
  ([the fallback agent](../explanation/fallback.md)).

### When the board does not check a name

The board does not check a name in these conditions. It gives the name to the program as you wrote it.

- The program has no list, and the table has no names for it.
- The program is not installed on this machine.
- The program uses a provider of its own: Claude Code with `ANTHROPIC_BASE_URL` or a cloud (`CLAUDE_CODE_USE_BEDROCK`,
  `…_VERTEX`, `…_FOUNDRY`), or Codex with a `model_provider` in its `config.toml`.

## An effort

`<role>.effort` is how much the agent thinks. Its value is `default` (the default of the program) or a level, such as
`low`, `medium` or `high`. The board refuses a wrong level when you set it, and tells you the levels that work.

- **Each program has its own levels. The board asks the program for them**, as it does for the names of models:
  - Codex: what `codex debug models` lists for each model. Today this is `low` to `max`, and `ultra` on some models. The
    board checks the level against the model that you chose (`codex/gpt-6-luna` has no `ultra`). If you chose no model,
    it checks against all levels that any model takes.
  - The other programs: what their `--help` lists for their effort argument. `claude --help` gives `low` to `max`.
    `pi --help` gives `off` to `max`.
  - The board keeps the levels for one hour. If a level is not on the list, the board asks the program again first. Thus
    it accepts a level that the program added a short time ago.
  - **If the board cannot read the levels**, it uses the levels in its table of programs
    ([the table of programs](agent-programs.md#the-table-of-programs)). This occurs when the program is not installed on this machine, is offline, or
    its help names no levels. The board never refuses all levels.
- The board refuses a level that the program of the role (or its model) does not have. It tells you the levels that the
  program has:

  ```
  $ herdr-projects set worker.effort minimal
  worker.effort: codex has no effort "minimal": it takes low, medium, high, xhigh, max, ultra, as `codex debug models` lists them (or default, for its own).
  ```

- A program that takes no effort at start takes only `default`.
- Cursor takes its effort in the name of the model (`cursor/claude-opus-4-8[effort=high]`). pi also takes an effort
  after its model (`:high`).
- You can set an effort for one program and then choose a different program or model. If the new one does not have
  that level, the board does not give the effort at start.
- At start, the board uses the levels that the program gave last. It never waits to ask.
- `<role>.fast` is the faster service tier of Codex. It has no effect on other programs.

## Related pages

- [Choose a model and an effort](../how-to/choose-a-model-and-effort.md)
- [Settings](settings.md)
- [Agent programs](agent-programs.md)
- [The fallback agent](../explanation/fallback.md)
- [Add a model list for a new program](../contributing/models-new-program.md)
