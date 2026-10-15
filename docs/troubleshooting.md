# Troubleshooting

**Most problems say what they are on the task's row, with the next step.** Select the task and press `Alt+P` (or
`Alt+T` for full screen): the task panel has the whole story. This page lists the common ones and where to look next.

## Quick checks

| Command | Tells you |
|---|---|
| `herdr-projects doctor` | whether an older install (herdr-superwork, herdr-projects 0.2) is still around, and takes over what is left of it ([older installs](old-installs.md)) |
| `herdr-projects install` | writes the `herdr-projects` launcher again, and says if something else of that name comes first on your PATH ([installing](install.md)) |
| `herdr-projects models [program]` | the model names each agent program takes here, or why its list cannot be read ([models](models.md)) |
| `herdr-projects machines` | whether each other machine answers, and why not ([other machines](other-machines.md)) |
| `herdr-projects show <id>` | every setting a task runs on, and where each value comes from ([settings](settings.md)) |
| `herdr-projects details <id>` | everything about one task: reviews, hand-overs, limits, what it waits for |
| `herdr workspace list` | whether Herdr itself runs and answers |

## An agent could not start

The row says `Could not start: …` with the reason, for example `codex is not logged in` or `codex does not know the model it was given`. Fix what it says, then delegate again.

The full table of reasons and fixes: [when an agent could not start](could-not-start.md).

**First time with a program?** Start it once yourself, in a terminal, in a folder of a repo: log in, finish its first
setup, and trust the folder if it asks ([agent programs](agents.md#before-a-programs-first-task)).

## A model name is refused

**A wrong name is refused when you set it**, with the names the program does take and the nearest one:

```
$ herdr-projects set worker.agent codex/gpt-5.5
worker.agent: codex has no model "gpt-5.5". It takes gpt-6.1-sol, gpt-6-astra, gpt-6-sol, … (from `codex debug models`).
Write it after the program: codex/gpt-6.1-sol.
```

- **See what a program takes:** `herdr-projects models codex`.
- **"Its list cannot be read":** the program is installed but its list command failed (not logged in, offline). Run that
  list command in a terminal to see why, or leave the model out.
- **A new Claude model is refused:** the board's table of Claude names is older than the model. Use the short name
  (`claude/opus`): it always means the newest of its kind. Updating the board (`herdr-projects update`) brings the new
  names.
- **An effort is refused:** each program has its own levels. The refusal lists the ones it has.

More: [models and effort](models.md).

## A task seems stuck

Find what its row says:

| The row says | What is going on | What to do |
|---|---|---|
| `Waiting for a free slot` (`Q#2`) | every slot of `board.agents` is taken | wait, or raise `board.agents` |
| `queued after HSW-12` | it waits for other tasks to be done ([one task after another](after.md)) | wait, or `herdr-projects delegate <id> --now` |
| `Has not started yet: pressing enter again` | the task's text sits in the agent's box | wait a moment: the board presses enter again |
| `Not started: its instructions sit unsent in its box` | the text never went in | `Ctrl+L` (retry), or go into the agent and press enter ([handing over](handing-over.md)) |
| `Finishing` for a long time, with the agent at rest | the board missed the end of its turn | `Ctrl+L`, or `herdr-projects finish <id>` |
| `Stopped without a summary` | the agent stopped and the board cannot tell if it finished | `Ctrl+R` tells it to go on; `Ctrl+L` takes the work as it is |
| `Agent's connection dropped, trying again in 2m` | the provider cut the turn off | nothing: the board retries by itself ([a turn cut off by the provider](provider-errors.md)) |
| `Agent stopped` | an error waiting cannot fix: a usage limit, a refusal | read the reason in the panel; reply, or `Ctrl+R` |
| `Needs a login` | the agent's program is logged out | log in (go into the agent with `Enter`), then reply or `Ctrl+R` |
| `stopped at its 2h limit` | it reached `limit.time` or `limit.tokens` | `Ctrl+R` to continue, a higher limit, or `Ctrl+L` to take it as it is ([limits](limits.md)) |
| `resolving a conflict with main` | main changed under it; its agent is fixing that | wait |
| `behind main` | main could not be brought in | the board tries again every minute; check your network, or put your checkout back on its branch ([how work gets done](how-work-gets-done.md#staying-current-with-main)) |
| `Waiting for GitHub checks · 2 of 3 passed` | the pull request's checks still run | wait: it merges by itself once they pass |
| `Could not read GitHub's checks` | `gh` is not logged in, or its token may not read Actions and Commit statuses | log `gh` in, or give the token those two read permissions ([how work gets done](how-work-gets-done.md#a-fine-grained-github-token-is-enough)) |
| `The check stands still · nothing finished for 13m` | the repo's check hangs | look at what the check runs ([the check's progress](check-progress.md)) |

**An agent shows a dialog** (a permission prompt, a question with choices): the chat says so. Press `Enter` to go into the
agent and answer it there.

**An agent asked a question in plain words and stopped**: the board takes that as finished and reviews it. Reply to the
agent, and it goes on ([an agent that waits for you](waiting-for-you.md)).

## Where the logs are

All in the board's data folder, `~/.herdr-projects/data/` ([the home folder](home-folder.md)):

| File | Holds |
|---|---|
| `tasks/<repo>/<n>/runner.log` | one task's background work: delegating, the finish line, merges. A failed start adds its account at the end: the reason, what Herdr said, the agent terminal's last lines |
| `coordinator/<session>/start.log` | why the Coordinator could not start, with its terminal's last lines |
| `keeper.log` | the keeper (the board's background process): starts, updates, errors |
| `hook.log` | what Herdr's hooks and the agents' turn-end reports told the board |
| `jobs.log` | background jobs that belong to no single task |
| `tasks/<repo>/<n>/closing-note.md` | what the agent said when it stopped |

The task panel shows most of this already. `herdr-projects details <id>` prints the rest.

## The board itself

- **`herdr-projects` is not found**: open the board once from Herdr (the plugin's action "Open the work board"), then add
  `~/.herdr-projects/data/bin` to your PATH ([installing](install.md#the-command-on-your-path)).
- **The board key does nothing**: a popup of another plugin may be open (close it first), or the key is not bound. The
  board offers the key on its first screen ([installing](install.md#the-first-run)).
- **The board looks cut off or half empty**: another Herdr window on the same session (a second device, a browser tab)
  has the size. Type a key in this window ([the board's size](board-size.md)).
- **Herdr refuses the plugin**: your running Herdr server must be 0.9.3 or newer. Restart it after updating Herdr.
- **Another machine's tasks do not show**: run `herdr-projects machines`, and see
  [when a machine does not answer](other-machines.md#when-a-machine-does-not-answer).
