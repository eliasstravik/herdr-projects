# Troubleshooting

**Most problems say what they are on the task's row, with the next step.** Select the task and press `Alt+P` (or
`Alt+T` for full screen): the task panel has the whole story. This page lists the common ones and where to look next.

## Quick checks

| Command | Tells you |
|---|---|
| `herdr-projects doctor` | the health check: every problem below in one run, each with what to do ([the health check](#the-health-check)) |
| `herdr-projects install` | writes the `herdr-projects` launcher again, and says if something else of that name comes first on your PATH ([installing](install.md)) |
| `herdr-projects models [program]` | the model names each agent program takes here, or why its list cannot be read ([models](models.md)) |
| `herdr-projects machines` | whether each other machine answers, how much disk each has free, and why not ([other machines](other-machines.md)) |
| `herdr-projects sweep` | removes what ended tasks left behind now, and says what ([disk space](disk-space.md)) |
| `herdr-projects show <id>` | every setting a task runs on, and where each value comes from ([settings](settings.md)) |
| `herdr-projects details <id>` | everything about one task: reviews, hand-overs, limits, what it waits for |
| `herdr workspace list` | whether Herdr itself runs and answers |

## The health check

`herdr-projects doctor` looks at everything below at once and takes about a second. A line with `✓` is fine. A line
with `✗` is a problem, with what to do under it. It exits 1 when there is a problem, so a script can run it too.

| It checks | A problem it reports | What to do |
|---|---|---|
| this program | its version and how it was installed (downloaded, built from source); older than the plugin's own version | `herdr-projects update`, or build it again |
| the launcher | `~/.herdr-projects/data/bin/herdr-projects` (what agents and hooks run) is missing, names a program that is gone, or not the plugin's | `sh <plugin folder>/bin/run.sh install`, or open the board once from Herdr |
| your PATH | `herdr-projects` is not on it, or something else of that name comes first | put the launcher's folder first ([installing](install.md#the-command-on-your-path)) |
| Herdr and the plugin | Herdr not found, older than the plugin needs, the plugin not installed, turned off, or linked to a folder that is gone | install or update Herdr; `herdr plugin enable herdr-projects`; link or install the plugin again |
| each agent program your settings name | not installed, older than the board was checked on, not signed in (Claude Code, Codex and Cursor can say), missing Herdr's integration | install it or choose another; `claude auth login`, `codex login`, `cursor-agent login`; `herdr integration install <name>` |
| model and effort settings | a model the program does not take, an effort it does not have, a program that cannot do its role, a value written by hand that does not count | `herdr-projects set <setting> …` ([models](models.md)) |
| plan approval | a plan's writer cannot run in its read-only mount here (bubblewrap missing or user namespaces not allowed on Linux; `sandbox-exec` not usable on macOS), while your settings ask for plans (`plan.approve`). With no setting asking, it is said as a fact | install bubblewrap; on macOS start Herdr outside any sandbox; or `herdr-projects set plan.approve none` ([a plan first](plan.md)) |
| other machines | one does not answer, runs an older version, or cannot ask this one | what `herdr-projects machines` says ([other machines](other-machines.md)) |
| disk space | under 10 GB free on a disk that holds the board's data, your repos, worktrees, build caches or temporary files: a task's setup may fail there | free some space, starting with the leftovers below |
| leftovers | the worktree of a done or archived task (a clean-up keeps one with unsaved work), a worktree on a task branch no task has, one git still lists whose folder is gone, an old warm build in `~/.cache/herdr-projects` | the `git worktree remove`, `git worktree prune` or `rm -rf` it names. It never removes anything itself; `herdr-projects sweep` removes what is provably an ended task's ([disk space](disk-space.md)) |
| older installs | herdr-superwork or herdr-projects 0.2 still in the way | it takes over what it can, and says what is left ([older installs](old-installs.md)) |

Other machines are asked only when there are some, and each for at most 5 seconds. A done task's worktree is no
leftover while its clean-up is still to come, or when `workspace.cleanup` is `false`.

## An agent could not start

The row says `Could not start: …` with the reason, for example `codex is not logged in` or `sol-6.1 can't be used with your ChatGPT sign-in`. Fix what it says, then delegate again.

The full table of reasons and fixes: [when an agent could not start](could-not-start.md).

**First time with a program?** Start it once yourself, in a terminal, in a folder of a repo: log in, finish its first
setup, and trust the folder if it asks ([agent programs](agents.md#before-a-programs-first-task)).

## No space left on device

| You see | What it is | What to do |
|---|---|---|
| a task's setup or check failed with `No space left on device` | the disk filled up during its build | `herdr-projects sweep`, then `herdr-projects doctor` for what else can go; then finish or delegate it again |
| `Disk nearly full on vm-str: … free on …; this task starts once 5.0 GB is free` | the board kept the task from starting on a nearly full disk (`board.diskFree`) | the same; it starts by itself once there is room |

Ended tasks clean up after themselves, and the board sweeps what they left once a day: [disk space](disk-space.md).

## A model name is refused

**A wrong name is refused when you set it**, with the names the program does take and the nearest one:

```
$ herdr-projects set worker.agent codex/sol-6.1
worker.agent: codex has no model "sol-6.1" for your ChatGPT sign-in (pro plan). Did you mean gpt-6.1-sol? It takes
gpt-6.1-sol, gpt-6-astra, gpt-6-sol, … (codex's own list for that sign-in). Write it after the program: codex/gpt-6.1-sol.
```

- **See what a program takes:** `herdr-projects models codex`. For Codex that is what your own sign-in can use: a ChatGPT
  sign-in and an API key take different names.
- **"Its list cannot be read":** the program is installed but its list command failed (not logged in, offline). Run that
  list command in a terminal to see why, or leave the model out.
- **A new Claude model is refused:** the board's table of Claude names is older than the model. Use the short name
  (`claude/opus`): it always means the newest of its kind. Updating the board (`herdr-projects update`) brings the new
  names.
- **An effort is refused:** each program has its own levels. The refusal lists the ones it has.

### "can't be used with your ChatGPT sign-in" while the agent works

Codex refused the model at the agent's first turn (`The 'sol-6.1' model is not supported when using Codex with a
ChatGPT account`). The board catches it:

- **`…: started again on codex's own default model`**: the board had given Codex that model; Codex now works on its own
  default. Nothing to do. To choose another, `herdr-projects models codex` lists the ones your sign-in can use.
- **`Agent stopped: … can't be used with your ChatGPT sign-in; try gpt-6.1-sol`**: its default was refused too, or the
  board gave it no model (then the name comes from Codex's own `model` in `~/.codex/config.toml`). Choose one for the
  task, `herdr-projects set worker.agent codex/gpt-6.1-sol --task <id>`, or fix that line, then `Ctrl+L` (retry): the
  agent starts again on it.

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
| `Agent stopped: … can't be used with your ChatGPT sign-in` | its program refused its model | choose a model, then `Ctrl+L` ([above](#cant-be-used-with-your-chatgpt-sign-in-while-the-agent-works)) |
| `Plan ready` · `The plan's review could not run (…): yours to judge` | the reviewer of a plan (`plan.approve reviewer`) could not start or gave no verdict | read the plan; `Ctrl+L` builds it, a reply sends it back, `herdr-projects retry <id>` has the reviewer look again ([a plan first](plan.md#the-reviewers-look)) |
| `Needs a login` | the agent's program is logged out | log in (go into the agent with `Enter`), then reply or `Ctrl+R` |
| `stopped at its 2h limit` | it reached `limit.time` or `limit.tokens` | `Ctrl+R` to continue, a higher limit, or `Ctrl+L` to take it as it is ([limits](limits.md)) |
| `resolving a conflict with main` | main changed under it; its agent is fixing that | wait |
| `behind main` | main could not be brought in; the work itself is fine | the board tries again every minute; the reason is in the panel ([below](#behind-main-main-could-not-be-fetched)) |
| `Waiting for GitHub checks · 2 of 3 passed` | the pull request's checks still run | wait: it merges by itself once they pass |
| `Could not read GitHub's checks` | `gh` is not logged in, or its token may not read Actions and Commit statuses | log `gh` in, or give the token those two read permissions ([how work gets done](how-work-gets-done.md#a-fine-grained-github-token-is-enough)) |
| `The check stands still · nothing finished for 13m` | the repo's check hangs | look at what the check runs ([the check's progress](check-progress.md)) |

**An agent shows a dialog** (a permission prompt, a question with choices): the chat says so. Press `Enter` to go into the
agent and answer it there.

**An agent asked a question in plain words and stopped**: the board takes that as finished and reviews it. Reply to the
agent, and it goes on ([an agent that waits for you](waiting-for-you.md)).

## A done task stays in Herdr's sidebar

A merged or finished task's space closes by itself (`workspace.cleanup`, on by default). When one stays, its line in the
task panel ends in `Kept:` and says why:

| It says | Why | What to do |
|---|---|---|
| `(uncommitted changes)` | its checkout has files that were never committed | look at them; reply to the task, or remove the worktree yourself |
| `(unmerged work on <branch>)` | its branch holds commits that were not merged | merge them, or reply to the task to go on with them |
| `(its space is in use: a turn in its agent's terminal)` | you talked to its agent in its own terminal | nothing: it is tried again about every half minute, and closes once the agent has been quiet a while |
| `(its space is in use beside its agent: <process> (pid <n>) <why>)` | a process runs in the agent's pane that is not one of its helpers; `<why>` says what is wrong with it (below) | end it if it is yours, and the space closes on the next try |
| `(its space is in use in another of its panes: <process> (pid <n>) runs in it)` | a command runs in another pane of its space | end it, or close that pane |
| `(its space is in use: herdr cannot tell whether its agent works, …)` | Herdr calls the agent's status unknown, and the rest of the line says what is not settled yet (its screen still moves, its last turn has not ended) | nothing: it closes once the agent has been still a while |
| `(its space is in use: herdr does not answer for its agent)` (or `for its space`) | Herdr did not answer | nothing: it is tried again |
| `(its herdr session cannot be reached)` | the Herdr session it ran in is not running | start that session; it is tried again |

- **What `<why>` means**: a process in the agent's own process group is one of its helpers (an MCP server) only when
  its input and output are wired to the agent and its errors go to no terminal. Otherwise it `streams to the terminal`,
  `is not wired to its agent` (a file, `/dev/null`, a pipe nobody in the agent's group holds), `has its input or output
  closed`, or `cannot be looked at` (the system does not say). A process under the agent in a group of its own (a dev
  server it started) `runs in a group of its own`; one its pane's shell runs `runs in the background of its shell`.
- **`; and 2 more`**: the line names the first thing only. `herdr-projects details <id>` lists every one, under `inUse`.
- **It names one of the agent's own helpers** (an MCP server, a language server): that is a bug. Please open an issue
  with the `Kept:` line and the `inUse` list of `herdr-projects details <id>`.
- **Nothing says `Kept:`, and the space still stays**: `herdr-projects show <id>` tells whether `workspace.cleanup` was
  turned off, and where. `herdr-projects set workspace.cleanup true` turns it on again.
- **An agent with MCP servers used to stay `in use` for good**: its servers counted as a command still running. Since
  this was fixed, the next pass closes those spaces by itself.
- **To close one at once**: archive the task (`Ctrl+A`); what is kept on disk stays, and the task says where.

## Behind main: main could not be fetched

Before a finished task is reviewed, and again right before it merges, the board fetches main and merges it into the
task's branch. When that fetch fails, nothing is failed: the task is checked and reviewed as usual, then waits as
`behind main`. It cannot be approved until it is up to date, and the board tries again by itself every minute. When
the remote cannot be reached at all, so the branch cannot be pushed either, the task waits the same way before its
push and review, and those run by themselves once the remote answers. A merge you had approved waits the same way,
with nothing merged: approve it again once the row is back to ready.

The panel (`Alt+P`) says why in plain words under **Behind main**, and what git itself said under **Details**:

| The reason says | What to do |
|---|---|
| origin asked for a login | make `git fetch origin` work in your checkout without a prompt: an SSH key loaded in the agent, or a credential helper. The board never types a password |
| origin did not answer within 60 seconds | the same, if it waits for a password or a passphrase; otherwise a slow or busy remote, and the next try may pass |
| origin could not be reached | the network or the server: nothing to do, the board tries again |
| origin has no branch … | the remote's main branch has another name, or the branch the task started from (`workspace.from`) is gone: `git fetch origin`, then `git remote set-head origin --auto` in your checkout, or push that branch again |
| git's lock file … stayed in the way | another git command was writing the same ref the whole time, or one that stopped left its `.lock` file behind: if no git command runs in that repo, delete the file it names |
| origin is not a repository the board can read | `git remote -v` shows where origin points |

Many tasks of one repo finishing at once never get in each other's way: they share one copy of main (every worktree
of a repo writes the same `origin/main`), so the board fetches it for one task at a time, and a task that waited for
another's fetch takes that one's result. A fetch that git refused because another program fetched at that very moment
(an agent's own `git fetch`, yours, an editor's) is tried again after 2 and 4 seconds, and a copy that program just
fetched counts. A remote whose main is not called `main` and was never named in your clone (no `origin/HEAD`, as after
`git remote add`) is asked which branch it is, and that branch is fetched, once.

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
- **`herdr` is not found although it is installed**: a command run over plain SSH (`ssh host herdr-projects …`) gets
  no login shell, so its PATH often lacks `~/.local/bin`. When PATH has no `herdr`, the board looks in
  `~/.local/bin`, `~/.cargo/bin`, `~/bin`, `/usr/local/bin` and `/opt/homebrew/bin`, then in a login shell's PATH. Herdr
  somewhere else: set `HERDR_BIN_PATH` to its full path.
- **The board key does nothing**: a popup of another plugin may be open (close it first), or the key is not bound. The
  board offers the key on its first screen ([installing](install.md#the-first-run)).
- **The board looks cut off or half empty**: another Herdr window on the same session (a second device, a browser tab)
  has the size. Type a key in this window ([the board's size](board-size.md)).
- **Herdr refuses the plugin**: your running Herdr server must be 0.9.3 or newer. Restart it after updating Herdr.
- **Another machine's tasks do not show**: run `herdr-projects machines`, and see
  [when a machine does not answer](other-machines.md#when-a-machine-does-not-answer).
