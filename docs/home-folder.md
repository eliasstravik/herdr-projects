# The home folder

**Everything the board keeps for you is in one folder, `~/.herdr-projects/`, the same on every machine.**

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

## Putting it somewhere else

| To move | Set |
|---|---|
| the whole folder | `HERDR_PROJECTS_HOME` |
| only the data | `HERDR_PROJECTS_DATA_DIR` |
| only the settings | `HERDR_PROJECTS_CONFIG_DIR` |
| only the reports | `HERDR_PROJECTS_REPORTS_DIR` |
| only the folder the Coordinator starts in | `HERDR_PROJECTS_COORDINATOR_DIR` |

The `XDG_*` variables (`XDG_STATE_HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`) no longer choose a place. They are read for
one thing only: to find where an older version kept its files, for the move below.

## Nothing of the board's goes into the folder an agent works in

What an agent writes for the board goes in its task's own folder, `data/tasks/<repo>/<n>/`:

- its closing note (`closing-note.md`);
- while a review runs, the reviewer's verdict and evidence (`review/`, moved to `artifacts/` when the review ends).

The agent is told the full path. So no repo or worktree gets a folder of the board's, and a task that works in your
home folder writes nothing into `~/.herdr-projects/` itself.

The closing note is removed when its task is cleaned up (after a merge, a task that ended with nothing to merge, or
`clear`). The whole task folder goes when the task is cleared away.

`.herdr-projects.json` (a repo's own settings) is a different file.

### Leftovers from older versions

Older versions had agents write into `.herdr-superwork/` inside the folder they worked in (ignored by git):
`closing-<n>.md` and `review-<n>/`. For a task in the home folder that was `~/.herdr-superwork/` itself, which is where
a `closing-1.md` beside `data` and `settings` comes from. Now:

- An agent started with the old rule keeps working. The board still reads a note from the old place for its task (the
  newer one, when both places have one) until that task is done. A task started by this version is never looked for
  there.
- When such a task is cleaned up, its own note in the old place is removed with it, and the old folder too when that
  leaves it empty.
- Anything else in an old `.herdr-superwork/` folder is left alone (a task started by this version removes nothing from
  it, whatever the files are called), and so is a folder that is not empty. Delete it yourself when no agent from
  before is still running. Its line in the repo's `.git/info/exclude` stays, so leftovers never show up as changes.

## What stays outside it

- **The how-to file** in each agent program's own skills folder (`~/.claude/skills/herdr-projects` and the others), and
  **the marked blocks** in Herdr's `config.toml`. They have to be where those programs read them.
  `herdr-projects remove-extras` takes them out ([installing](install.md)).
- **The web UI's shared link.** With the Herdr web UI plugin installed, and only after a yes: where that web UI's shared
  link (`tailscale serve`) points, so the Board shows in it. `herdr-projects web-ui unlink` and `remove-extras` point it
  back ([the board in the Herdr web UI](web-ui.md)).
- **The trust mark for a repo**, in the agent program's own settings (`~/.claude.json`, `~/.codex/config.toml`).
- **This repo's own test runner** keeps what already passed in `~/.cache/herdr-projects-check` ([testing](testing.md)).
  That is a cache of a tool for working on this code, not something the board keeps for you: delete it any time.

## Coming from herdr-superwork or herdr-projects 0.2

`~/.herdr-superwork` becomes this folder. The old herdr-projects 0.2's projects, which it kept under this very name, are
set aside to `~/.herdr-projects-legacy` first: [older installs](old-installs.md).

## Coming from an older version: the one-time move

Older versions kept the data in `~/.local/state/herdr-superwork`, the settings in `~/.config/herdr-superwork`, and
agents' reports in `~/.local/share/herdr-superwork-reports`. **The first run of a new version moves all three into
`~/.herdr-projects/` by itself**: any run, whether the board, a command, or the hook Herdr calls. It then says so once,
in one line, the first time you would see it (on the board when it opens, or after a command you typed):

    Moved the board's files into ~/.herdr-projects: data (from ~/.local/state/herdr-superwork), settings (from
    ~/.config/herdr-superwork), reports (from ~/.local/share/herdr-superwork-reports). The old places are links to
    the new ones, so whatever is running carries on.

### Why it is safe

- **Nothing is copied or deleted, and no old path is ever missing.** The old path is the folder itself, and then, in one
  step, a link to where the folder now is. Each folder goes in three steps, each whole or not done at all:
  1. two spare links are made (`<old>.moving` and the new place);
  2. the old folder and the spare link change places;
  3. the new place and the spare name change places.

  "Change places" is one system call (Linux `renameat2`, a Mac's `renamex_np`), so a command started at the old path at
  any moment finds its file. A run that was cut off is carried on by the next from the shape it left, and a second run
  finds nothing to do. `moved.json` is written before the first step.
- **The old places become links to the new ones.** Everything that still holds an old path keeps working on the same
  files: a board or keeper still running old code, the command path agents were handed
  (`~/.local/state/herdr-superwork/bin/herdr-superwork`), the hook settings of running agents, a task's saved artifacts,
  the page server. A lock is one file whichever path takes it, so old and new code still take turns. The links can be
  removed once nothing started before the move still runs; the board leaves them.
- **Data already in the new place is never written over.** What the new place holds is first put into the old folder
  (the one in use), then the folder moves as above. Whatever only one side has is kept. Of a file both have, the newer
  one stays in use and the other goes to `set-aside/`, and the line says how many.
- **Nobody's lock is taken from under it.** When both places have the same lock file, the one in the old place (the
  place in use) is the lock and is never replaced or removed: a process that holds it, or has opened it and is about to
  take it, still has the one real lock afterwards. The new place's copy goes, and only while nobody holds it; while one
  does, that folder is not moved now, and a later run moves it. And after the board takes a lock, it checks that what
  it holds is still the file at that path, and takes the one there if not.
- **A run never starts on an empty new place.** Whatever has not moved when a run begins (another run is still moving it
  after three seconds of waiting, a step failed, a lock in the new place was held) is used where it is, in the old
  place, for that run.
- **If two paths cannot change places in one step** (a system without that call), **or the old place is on another
  disk**, the folder stays where it is and the new place becomes a link to it. Still one folder, still no moment without
  the old path, and the line says so.
- **A folder you named yourself stays.** With `HERDR_PROJECTS_DATA_DIR` (or the settings or reports one) set to a place
  of your own, that part is not moved.
- **Only the copy the board runs from moves anything.** A second checkout of this code (a git worktree an agent works
  in, a copy under test) leaves the files alone and works on the board where it is.

### The Coordinator's conversation

Its folder moves with the data (`data/coordinator/<session>/chat`), and a running Coordinator stays in it. After its
next restart it is in the same conversation: the board reopens it by its id, and all three programs find a conversation
by id from the moved folder. (Tried for real with Claude Code 2.1.296, which keys saved conversations by folder:
`--resume` found it and went on in it. Codex and pi keep theirs by date and by id.)

A program started before the move still knows its folder by the old path, and saves a new chat (`/clear`, `/new`) under
that one. The board looks there too, so it is followed as always.

### Undoing it

Stop the board, remove the three links, move `data`, `settings` and `reports` back to the old places, and run the old
version. Nothing else was changed.
