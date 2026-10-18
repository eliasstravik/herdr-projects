# The home folder

This page lists the contents of the home folder, `~/.herdr-projects/`, and the variables that move it. Everything
that the board keeps for you is in this one folder. The folder is the same on every machine.

## Contents

```
~/.herdr-projects/
  settings/     config.json: your own preferences (docs/settings.md)
  data/         everything the board writes while it runs
    tasks/<repo>/<n>/        one folder per task: task.json, inbox.jsonl, closing-note.md, artifacts/
    coordinator/<session>/   the Coordinator's saved state, and chat/, the folder it starts in
    automations/             the automations, their firings, the board events counted already (docs/automations.md)
    bin/herdr-projects      the command agents are handed, by its full path
    conflicts.jsonl          one line per conflict with main, once repaired (docs/how-work-gets-done.md)
    locks/ stamps/ *.json *.log   the board's own bookkeeping
  reports/      pages and reports an agent made for you that belong in no repo, one folder per task
    old-suggestions.md       the suggested tasks that still waited for an answer when they were removed from the board
  moved.json    written by the one-time move below: what was moved, from where
  set-aside/    only after a move that found a file in both places: the copy that was not kept in use
```

- For `settings/config.json`, see [Settings](settings.md).
- For `data/automations/`, see [Automations](automations.md).
- For `data/conflicts.jsonl`, see [How work gets done](../explanation/how-work-gets-done.md).
- `moved.json` and `set-aside/` come from the one-time move. See
  [Move from an old install](../how-to/move-from-an-old-install.md).

Other files that can be in the folder:

- `data/keeper.log`: the log of the keeper. It records a failed self-update. See
  [Install details](install-details.md#how-the-keeper-updates-itself).
- `data/known_hosts`: the list of host keys that the board keeps for other machines. See
  [Set up another machine](../how-to/set-up-another-machine.md).
- `data-0.2.0`: a copy of the data folder that the first run made when you came from the older Node version.

## Variables that move the folder

| To move | Set |
|---|---|
| the whole folder | `HERDR_PROJECTS_HOME` |
| only the data | `HERDR_PROJECTS_DATA_DIR` |
| only the settings | `HERDR_PROJECTS_CONFIG_DIR` |
| only the reports | `HERDR_PROJECTS_REPORTS_DIR` |
| only the folder the Coordinator starts in | `HERDR_PROJECTS_COORDINATOR_DIR` |

For the steps, see [Move the home folder](../how-to/move-the-home-folder.md).

The `XDG_*` variables (`XDG_STATE_HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`) no longer choose a place. The board
reads them for one thing only: to find where an older version kept its files, for the one-time move.

## Files that an agent writes for the board

An agent writes its files for the board in the folder of its task, `data/tasks/<repo>/<n>/`:

| File | What it is | When it goes |
|---|---|---|
| `closing-note.md` | The closing note of the agent. | When its task is cleaned up: after a merge, after a task that ended with nothing to merge, or after `clear`. |
| `review/` | While a review runs: the verdict and the evidence of the reviewer. | It moves to `artifacts/` when the review ends. |

The board gives the agent the full path. The whole folder of the task goes when the task is cleared away.

`.herdr-projects.json` is a different file. It holds the settings of one repo, in that repo.

## Leftovers from older versions

Older versions had agents write into `.herdr-superwork/` inside the folder where they worked. Git ignored this
folder. The files were `closing-<n>.md` and `review-<n>/`. For a task in the home folder, that folder was
`~/.herdr-superwork/` itself. This is why a `closing-1.md` can be beside `data` and `settings`.

- An agent that started with the old rule continues to work.
- The board still reads a note from the old place for the task of that agent, until that task is done. If both
  places have a note, it reads the newer one.
- The board never looks in the old place for a task that this version started.
- When such a task is cleaned up, the board removes its own note in the old place. It also removes the old folder if
  the folder is then empty.
- The board leaves everything else in an old `.herdr-superwork/` folder. A task that this version started removes
  nothing from it, whatever the names of the files are.
- The board also leaves a folder that is not empty. Delete it yourself when no agent from before still runs.
- The line of the folder in the `.git/info/exclude` file of the repo stays. Thus leftovers never show as changes.

## What stays outside the home folder

| Item | Where it is | How to remove it |
|---|---|---|
| The how-to file of each agent program | The skills folder of that program: `~/.claude/skills/herdr-projects` and the others. | `herdr-projects remove-extras` |
| The marked blocks | The Herdr `config.toml`. | `herdr-projects remove-extras` |
| The shared link of the web UI | Where the shared link (`tailscale serve`) of the Herdr web UI points. Only if the Herdr web UI plugin is installed, and only after you said yes. | `herdr-projects web-ui unlink` or `herdr-projects remove-extras` |
| The trust mark for a repo | The settings of the agent program: `~/.claude.json`, `~/.codex/config.toml`. | |
| The cache of the test runner of this repo | `~/.cache/herdr-projects-check`. | Delete it at any time. |

The how-to files and the marked blocks must be where those programs read them. The shared link lets the board show
in the web UI. For the web UI, see [The web UI](web-ui.md). For the test runner, see
[Testing](../contributing/testing.md). The cache is for a tool that you use to work on this code. It is not
something that the board keeps for you.

## Old paths after a move

| Old path | What it is now |
|---|---|
| `~/.herdr-superwork` | A link to `~/.herdr-projects`. |
| `~/.local/state/herdr-superwork` | A link to `~/.herdr-projects/data`. |
| `~/.config/herdr-superwork` | A link to `~/.herdr-projects/settings`. |
| `~/.local/share/herdr-superwork-reports` | A link to `~/.herdr-projects/reports`. |
| `~/.herdr-projects-legacy` | The projects of herdr-projects 0.2, set aside. |
| `~/.config/herdr-projects-legacy` | The settings of herdr-projects 0.2, set aside. |

If the system cannot swap two paths in one step, or the old place is on a different disk, the old folder stays
where it is. The new place is then a link to the old folder.

The board does not remove the old links. For more, see
[What the takeover of an old install does](old-installs-takeover.md).

## Related pages

- [The home folder (explanation)](../explanation/home-folder.md)
- [Move the home folder](../how-to/move-the-home-folder.md)
- [Move from an old install](../how-to/move-from-an-old-install.md)
- [What the takeover of an old install does](old-installs-takeover.md)
- [Install details](install-details.md)
- [Settings](settings.md)
