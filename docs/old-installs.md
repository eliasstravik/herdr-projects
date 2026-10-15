# Older installs: herdr-superwork and herdr-projects 0.2

**You do nothing.** The first run of this version takes over both older programs by itself, and says what it did in
one line (on the terminal, or on the board when it next opens). Nothing is deleted.

- `herdr-projects doctor` looks again at any time, takes over whatever is left, and says where things stand.
- `herdr-projects install` ends with the same lines.
- Running either again changes nothing.

"The first run" is any run: the board, a command, or the hook Herdr calls.

| Older program | What happens to it |
|---|---|
| **herdr-superwork** (this version's working title) | moved into `~/.herdr-projects` and carried on: tasks, keys and history stay |
| **herdr-projects 0.2** (the old Rust program, now on the `legacy` branch) | set aside in `-legacy` folders, never mixed in |

## What they had

Both use names this version needs:

- **herdr-superwork** kept everything in `~/.herdr-superwork/` (tasks, settings, reports, the Coordinator's chat). It
  wrote `# >>> herdr-superwork …` blocks in Herdr's config, a `herdr-superwork` how-to in each agent program's skills
  folder, and ran its own keeper and page server (with the web UI's front door).
- **herdr-projects 0.2** kept its projects in `~/.herdr-projects/` (this version's home folder), its settings in
  `~/.config/herdr-projects/`, and a file in Herdr's state folder for the plugin. It changed Herdr's config, the agent
  programs' settings (hooks) and their skills folders.

## herdr-projects 0.2: set aside, never mixed, never deleted

- **Its projects.** A `~/.herdr-projects` that holds the old version's projects (folders with a `PROJECT.md`) or its own
  files (`.ticker.log`, `.machines.json`, …) is renamed to `~/.herdr-projects-legacy`, whole and as it is. If this
  version's files are there too, only the old version's move into that folder.
  - Its ticker is asked to stop first, its own way (a `.ticker.stop` file), then with a signal. While it still runs,
    nothing is moved, and the next run tries again.
  - The old version finds its projects there with `HERDR_PROJECTS_ROOT=~/.herdr-projects-legacy`.
- **Its settings** (`~/.config/herdr-projects`) move to `~/.config/herdr-projects-legacy`. Herdr's state folder for it
  (`~/.local/state/herdr/plugins/herdr-projects`, which this version shares by its id) moves to
  `…/herdr-projects-legacy`.
- **Only what would clash is taken out**, and every other line stays as it was:
  - in Herdr's config: its key (the `herdr-projects.open-popup` action), its line in the tab bar
    (`… needs-you --line`) and its sidebar rows (the `$hp_…` tokens; where it had put one row in place of Herdr's two,
    Herdr's two come back);
  - its hooks in Claude's, Codex's, Droid's, Gemini's and Copilot's settings (`… herdr-projects --root … hook --agent …`),
    each file backed up first (`<file>.bak-herdr-projects`);
  - its `autoproject` skill links;
  - the `herdr-projects` link it put on your PATH, when that points into Herdr's plugins and not at this program.
- `HERDR_PROJECTS_ROOT` is read by the old version only. `doctor` says so when it is set.

## herdr-superwork: moved into ~/.herdr-projects, carried on

- **The folder.** `~/.herdr-superwork` becomes `~/.herdr-projects` in the same three whole steps as the one-time move of
  the home folder ([the home folder](home-folder.md)): there is never a moment without either path, and a run that is
  cut off is finished by the next.
  - `~/.herdr-superwork` is left as a link to the new folder, so an agent, hook or page that still holds an old path
    carries on. Remove it once nothing started before the move still runs.
  - A `~/.herdr-projects` that already has this version's files is merged as that move merges: the newer of a file both
    have stays in use, the other goes to `set-aside/`.
  - The old 0.2 projects are set aside first: the two are never mixed.
- **The paths inside it.** In what the board reads back (its `.json`, `.jsonl` and `.md` files in `data/` and
  `settings/`), every path into `~/.herdr-superwork` becomes the same path into `~/.herdr-projects`, and the command
  agents were handed (`…/data/bin/herdr-superwork`) becomes the new one (`…/data/bin/herdr-projects`).
  - A repo's name in a task's id (`herdr-superwork/218`) is a name, not a path, and stays. Tasks, keys (`HSW-218`) and
    history carry over as they are.
  - Kept as they were: a task's artifacts, reports, the Coordinator's chat folder, the copies an older version kept
    (`data-0.2.0`), and the record of earlier moves.
  - The old launcher, `data/bin/herdr-superwork`, now runs this program, for agents that still use it.
- **The Coordinator's conversation** carries on: after its next restart it is found by its id in the moved folder, as
  after the first move.
- **Its processes.** Its keeper and page server (with the web UI's front door) are stopped, but only when the program
  they run is herdr-superwork's, and this version's are started. The front door opens on the port the old one had, so
  the web UI's shared link points at the new one with no change. A link that moved is brought along
  ([the board in the Herdr web UI](web-ui.md)).
- **Its plugin**, if Herdr still has `herdr-superwork` installed, is switched off (`herdr plugin disable`, not
  removed), so its hooks no longer run the old program on the moved board.
- **Its extras** become this version's:
  - its marked blocks in Herdr's config (`# >>> herdr-superwork key`, `… sidebar`) are replaced by this version's, with
    the same key;
  - a binding of a `herdr-superwork.<action>` now names `herdr-projects.<action>` (the same action);
  - its how-to folders are removed and this version's written;
  - Herdr's own config and state folders for the plugin are taken over into `herdr-projects`'s.
  - Herdr's config is changed only after `herdr config check` accepts the result on a copy. It is backed up first, and
    put back if Herdr rejects it; `doctor` says so and tries again.
- **A repo's own settings file.** A repo that still has only `.herdr-superwork.json` keeps its settings: the board reads
  that file until the repo has a `.herdr-projects.json`.

## Only the copy the board runs from

As with the one-time move, none of this is done by a copy of the program in a git worktree (an agent working on this
code), by the tests, or by `help`, `version` and `docs`.

## Undoing it

1. Stop the board.
2. For herdr-projects 0.2: rename the `-legacy` folders back.
3. For herdr-superwork: remove the link `~/.herdr-superwork`, rename `~/.herdr-projects` back to it, and make
   `~/.herdr-projects` a link to it (the paths written inside now name `~/.herdr-projects`).
4. The backups of Herdr's config and of the agents' settings are beside them (`.bak-herdr-projects`).
5. `herdr plugin enable herdr-superwork` switches its plugin back on.
