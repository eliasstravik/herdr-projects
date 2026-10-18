# What the takeover of an old install does

This page lists what the first run of this version does to herdr-superwork and to herdr-projects 0.2. For the
steps, and for how to undo the takeover, see [Move from an old install](../how-to/move-from-an-old-install.md).

The first run takes over both older programs by itself. It deletes nothing. "The first run" is any run: the board,
a command, or the hook that Herdr calls.

## What the old programs had

Both older programs use names that this version needs.

- **herdr-superwork** kept everything in `~/.herdr-superwork/`: tasks, settings, reports and the chat of the
  Coordinator. It wrote `# >>> herdr-superwork …` blocks in the Herdr config. It wrote a `herdr-superwork` how-to
  in the skills folder of each agent program. It ran its own keeper and page server, with the front door of the web
  UI.
- **herdr-projects 0.2** kept its projects in `~/.herdr-projects/`, which is the home folder of this version. It kept
  its settings in `~/.config/herdr-projects/`, and a file for the plugin in the Herdr state folder. It changed the
  Herdr config, the settings (hooks) of the agent programs, and their skills folders.

## herdr-projects 0.2: set aside, never mixed, never deleted

### Its projects

- If `~/.herdr-projects` holds projects of the old version (folders with a `PROJECT.md`), the board renames it to
  `~/.herdr-projects-legacy`. It does this as it is, with all of its contents.
- The same applies if `~/.herdr-projects` holds files of the old version (`.ticker.log`, `.machines.json`, …).
- If files of this version are there too, only the files of the old version move into `~/.herdr-projects-legacy`.
- The board first asks the ticker of the old version to stop, in the way of the old version (a `.ticker.stop` file).
  Then it sends a signal. While the ticker still runs, the board moves nothing. The next run tries again.
- The old version finds its projects there with `HERDR_PROJECTS_ROOT=~/.herdr-projects-legacy`.

### Its settings and state

- Its settings move from `~/.config/herdr-projects` to `~/.config/herdr-projects-legacy`.
- Its Herdr state folder, `~/.local/state/herdr/plugins/herdr-projects`, moves to `…/herdr-projects-legacy`. This
  version uses the same folder, because it has the same plugin id.

### Only what clashes is removed

The board removes only the items in this list. Every other line stays as it was.

- In the Herdr config:
  - its key (the `herdr-projects.open-popup` action);
  - its line in the tab bar (`… needs-you --line`);
  - its sidebar rows (the `$hp_…` tokens). Where it had put one row in place of the two rows of Herdr, the two rows
    of Herdr come back.
- Its hooks in the settings of Claude, Codex, Droid, Gemini and Copilot (`… herdr-projects --root … hook --agent …`).
  The board makes a backup of each file first (`<file>.bak-herdr-projects`).
- Its `autoproject` skill links.
- The `herdr-projects` link that it put on your PATH, if that link points into the Herdr plugins and not at this
  program.

Only the old version reads `HERDR_PROJECTS_ROOT`. If it is set, `doctor` tells you this.

## herdr-superwork: moved into `~/.herdr-projects`, carried on

### The folder

- `~/.herdr-superwork` becomes `~/.herdr-projects`. The board uses the same three whole steps as the one-time move
  of the home folder. There is never a moment without one of the two paths. If a run stops before the end, the next
  run finishes the move. See [The home folder](../explanation/home-folder.md#how-the-one-time-move-keeps-your-data-safe).
- The board leaves `~/.herdr-superwork` as a link to the new folder. Thus an agent, a hook or a page that still has an
  old path continues to work. Remove the link when nothing that started before the move still runs.
- If `~/.herdr-projects` already has files of this version, the board merges the two in the same way as the one-time
  move. Of a file that both have, the newer one stays in use and the other goes to `set-aside/`.
- The board sets aside the old 0.2 projects first. It never mixes the two.

### The paths inside it

- In the files that the board reads back (its `.json`, `.jsonl` and `.md` files in `data/` and `settings/`), each
  path into `~/.herdr-superwork` becomes the same path into `~/.herdr-projects`.
- The command that agents got (`…/data/bin/herdr-superwork`) becomes the new one (`…/data/bin/herdr-projects`).
- A repo name in the id of a task (`herdr-superwork/218`) is a name, not a path, so it stays. Tasks, keys (`HSW-218`)
  and history carry over as they are.
- These items stay as they were: the artifacts of a task, reports, the chat folder of the Coordinator, the copies
  that an older version kept (`data-0.2.0`), and the record of earlier moves.
- The old launcher, `data/bin/herdr-superwork`, now runs this program, for agents that still use it.

### The conversation of the Coordinator

The conversation of the Coordinator continues. After its next restart, the Coordinator finds it by its id in the
moved folder, as after the first move.

### Its processes

- The board stops the keeper and the page server of herdr-superwork, with the front door of the web UI. It does this
  only if the program that they run is the herdr-superwork program. Then it starts the keeper and page server of this
  version.
- The new front door opens on the port that the old one had. Thus the shared link of the web UI points at the new
  front door with no change.
- If a link moved, the board brings it along. See [The web UI](web-ui.md).

### Its plugin

If Herdr still has `herdr-superwork` installed, the board switches that plugin off (`herdr plugin disable`). It does
not remove it. Thus its hooks no longer run the old program on the moved board.

### Its extras

- The board replaces its marked blocks in the Herdr config (`# >>> herdr-superwork key`, `… sidebar`) with the
  blocks of this version, with the same key.
- A binding of a `herdr-superwork.<action>` now names `herdr-projects.<action>`. It is the same action.
- The board removes its how-to folders and writes the how-to folders of this version.
- The board takes over the Herdr config and state folders of the plugin into the folders of `herdr-projects`.
- The board changes the Herdr config only after `herdr config check` accepts the result on a copy. It makes a backup
  first. If Herdr rejects the change, the board puts the backup back. `doctor` then says so and tries again.

### The settings file of a repo

If a repo still has only `.herdr-superwork.json`, it keeps its settings. The board reads that file until the repo has
a `.herdr-projects.json`.

## Which copy of the program does the takeover

Only the copy that the board runs from does the takeover. As with the one-time move, these never do it:

- a copy of the program in a git worktree (an agent that works on this code);
- the tests;
- the commands `help`, `version` and `docs`.

## Related pages

- [Move from an old install](../how-to/move-from-an-old-install.md)
- [The home folder (reference)](home-folder.md)
- [The home folder (explanation)](../explanation/home-folder.md)
- [The web UI](web-ui.md)
