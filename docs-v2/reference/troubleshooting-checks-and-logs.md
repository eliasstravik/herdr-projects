# The health check and the logs

This page lists the commands that find problems, the items that the health check examines, and the log files of the
board. To fix a problem, read [troubleshoot the board](../how-to/troubleshoot.md).

## Quick checks

| Command | What it tells you |
|---|---|
| `herdr-projects doctor` | The health check: each problem on this page in one run, each with what to do ([the health check](#the-health-check)). |
| `herdr-projects install` | It writes the `herdr-projects` launcher again. It tells you if a different program with that name comes first on your PATH ([install and update](../how-to/install-and-update.md)). |
| `herdr-projects models [program]` | The model names that each agent program takes on this machine, or why the board cannot read its list ([models](models.md)). |
| `herdr-projects machines` | Whether each other machine answers, and why not ([other machines](../explanation/other-machines.md)). |
| `herdr-projects show <id>` | Each setting that a task uses, and where each value comes from ([settings](settings.md)). |
| `herdr-projects details <id>` | All about one task: the reviews, the hand-overs, the limits, and what it waits for. |
| `herdr workspace list` | Whether Herdr itself runs and answers. |

## The health check

`herdr-projects doctor` examines all the items below at the same time, in approximately one second.

- A line with `✓` is correct.
- A line with `✗` is a problem. The line under it tells what to do.
- The command exits with 1 when there is a problem. Thus a script can also run it.

| It examines | A problem that it reports | What to do |
|---|---|---|
| this program | its version and how you installed it (downloaded, built from source); older than the version of the plugin | `herdr-projects update`, or build it again |
| the launcher | `~/.herdr-projects/data/bin/herdr-projects` (what agents and hooks run) is missing, names a program that is gone, or is not the launcher of the plugin | `sh <plugin folder>/bin/run.sh install`, or open the board one time from Herdr |
| your PATH | `herdr-projects` is not on it, or a different program with that name comes first | put the folder of the launcher first ([put the command on your PATH](../how-to/install-and-update.md#put-the-command-on-your-path)) |
| Herdr and the plugin | Herdr is not found or is older than the plugin needs; the plugin is not installed, is turned off, or is linked to a folder that is gone | install or update Herdr; `herdr plugin enable herdr-projects`; link or install the plugin again |
| each agent program that your settings name | not installed, older than the board was checked on, not signed in (Claude Code, Codex and Cursor can tell this), the Herdr integration is missing | install it or choose another; `claude auth login`, `codex login`, `cursor-agent login`; `herdr integration install <name>` |
| the model and effort settings | a model that the program does not take, an effort that it does not have, a program that cannot do its role, a value written by hand that does not count | `herdr-projects set <setting> …` ([models](models.md)) |
| plan approval | the writer of a plan cannot run in its read-only mount on this machine (on Linux: bubblewrap is missing or user namespaces are not allowed; on macOS: `sandbox-exec` is not usable), while your settings ask for plans (`plan.approve`). If no setting asks for plans, it tells this as a fact | install bubblewrap; on macOS, start Herdr outside all sandboxes; or `herdr-projects set plan.approve none` ([get a plan first](../how-to/get-a-plan-first.md)) |
| other machines | one does not answer, runs an older version, or cannot ask this machine | what `herdr-projects machines` tells ([other machines](../explanation/other-machines.md)) |
| disk space | less than 10 GB free on a disk that holds the data of the board, your repos, worktrees, build caches or temporary files: the setup of a task can fail there | make space free; start with the leftovers below |
| leftovers | the worktree of a done or archived task (a clean-up keeps one with unsaved work), a worktree on a task branch that no task has, a worktree that git still lists but whose folder is gone, an old warm build in `~/.cache/herdr-projects` | the `git worktree remove`, `git worktree prune` or `rm -rf` that it names. It never removes anything itself |
| older installs | herdr-superwork or herdr-projects 0.2 is still in the way | it takes over what it can, and tells what is left ([move from an old install](../how-to/move-from-an-old-install.md)) |

The health check asks other machines only when there are some. It asks each machine for a maximum of 5 seconds.

The worktree of a done task is not a leftover while its clean-up is still to come, or when `workspace.cleanup` is
`false`.

## The log files

All the logs are in the data folder of the board, `~/.herdr-projects/data/` ([the home folder](home-folder.md)):

| File | What it holds |
|---|---|
| `tasks/<repo>/<n>/runner.log` | The background work of one task: the delegation, the finish line, the merges. A failed start adds its account at the end: the reason, what Herdr said, the last lines of the terminal of the agent. |
| `coordinator/<session>/start.log` | Why the Coordinator could not start, with the last lines of its terminal. |
| `keeper.log` | The keeper (the background process of the board): starts, updates, errors. |
| `hook.log` | What the Herdr hooks and the turn-end reports of the agents told the board. |
| `jobs.log` | The background jobs that belong to no single task. |
| `tasks/<repo>/<n>/closing-note.md` | What the agent said when it stopped. |

The task panel already shows most of this. `herdr-projects details <id>` prints the rest.

## Related pages

- [Troubleshoot the board](../how-to/troubleshoot.md)
- [Fix an agent that could not start](../how-to/could-not-start-fix.md)
- [The home folder](home-folder.md)
- [Commands](commands.md)
- [Install and update](../how-to/install-and-update.md)
