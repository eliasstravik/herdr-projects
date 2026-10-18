# The home folder: how it works and why

This page tells why the board keeps everything in one folder, why it writes nothing into the folder of an agent, and
how the one-time move from older places keeps your data safe. For the list of files, see
[The home folder](../reference/home-folder.md).

## One folder for everything

Everything that the board keeps for you is in one folder, `~/.herdr-projects/`. The folder is the same on every
machine. Thus you can find the tasks, settings and reports of the board in one place, and you can move them with
one variable (`HERDR_PROJECTS_HOME`).

Older versions used the `XDG_*` folders. Now these variables do not choose a place. The board reads them only to
find the files of an older version.

## Nothing of the board goes into the folder where an agent works

An agent writes its files for the board in the folder of its task, `data/tasks/<repo>/<n>/`. The board tells the
agent the full path. Thus no repo and no worktree gets a folder of the board. A task that works in your home folder
also writes nothing into `~/.herdr-projects/` itself.

Older versions did the opposite. Agents wrote into `.herdr-superwork/` inside the folder where they worked. The
board still reads those old notes for agents that started with the old rule. It removes only the old note of such a
task, when that task is cleaned up. It leaves everything else there. See [Leftovers from older versions](../reference/home-folder.md#leftovers-from-older-versions).

## Why some files stay outside the folder

The how-to file in the skills folder of each agent program and the marked blocks in the Herdr `config.toml` must be
where those programs read them. The same applies to the trust mark for a repo in the settings of an agent program.
The shared link of the web UI must point at the board, so that the board shows in the web UI. The cache of the test runner is for work on this code,
not for the board.

## How the one-time move keeps your data safe

Older versions kept the data in `~/.local/state/herdr-superwork`, the settings in `~/.config/herdr-superwork`, and
the reports of agents in `~/.local/share/herdr-superwork-reports`. The first run of a new version moves all three
into `~/.herdr-projects/` by itself. The same steps also move `~/.herdr-superwork` to `~/.herdr-projects`.

### Nothing is copied or deleted, and no old path is ever missing

The old path is first the folder itself. Then, in one step, it becomes a link to the new place of the folder. Each
folder moves in three steps. Each step is done completely or not at all:

1. The board makes two spare links: `<old>.moving` and the new place.
2. The old folder and the spare link change places.
3. The new place and the spare name change places.

"Change places" is one system call: `renameat2` on Linux, `renamex_np` on a Mac. Thus a command that starts at the
old path at any moment finds its file. If a run stops before the end, the next run continues from the state that it
found. A second run finds nothing to do. The board writes `moved.json` before the first step.

### The old places become links to the new ones

Everything that still has an old path continues to work on the same files:

- a board or a keeper that still runs old code;
- the command path that agents got (`~/.local/state/herdr-superwork/bin/herdr-superwork`);
- the hook settings of agents that run;
- the saved artifacts of a task;
- the page server.

A lock is one file, whichever path takes it. Thus old code and new code still take turns. You can remove the links
when nothing that started before the move still runs. The board does not remove them.

### Data that is already in the new place is never written over

The board first puts the contents of the new place into the old folder, which is the folder in use. Then the folder
moves as above. The board keeps whatever only one side has. Of a file that both sides have, the newer one stays in
use and the other goes to `set-aside/`. The line that reports the move says how many files went there.

### No lock is taken from a process that holds it

If both places have the same lock file, the lock in the old place is the real lock, because the old place is in use.
The board never replaces or removes it. A process that holds that lock, or has opened it to take it, still has the
one real lock after the move.

The board removes the copy in the new place, but only while no process holds it. While a process holds it, the board
does not move that folder now. A later run moves it. Also, after the board takes a lock, it checks that it still
holds the file at that path. If not, it takes the lock that is there.

### A run never starts on an empty new place

Sometimes a part has not moved when a run starts. For example, a different run still moves it after three seconds of
waiting, a step failed, or a process held a lock in the new place. For that run, the board uses the part where it is,
in the old place.

### When the folder cannot move

The system can have no call that swaps two paths in one step. Or the old place can be on a different disk. In these
cases, the folder stays where it is, and the new place becomes a link to it. There is still one folder, and there is
still no moment without the old path. The line that reports the move says so.

### A folder that you named yourself stays

If you set `HERDR_PROJECTS_DATA_DIR` to a place of your own, the board does not move the data. The same applies to
the settings variable (`HERDR_PROJECTS_CONFIG_DIR`) and to the reports variable (`HERDR_PROJECTS_REPORTS_DIR`).

### Only the copy that the board runs from moves anything

A second checkout of this code does not move the files. This can be a git worktree where an agent works, or a copy
under test. It leaves the files alone and works on the board where the board is. The tests and the commands
`help`, `version` and `docs` also move nothing.

## The conversation of the Coordinator after a move

The folder of the Coordinator moves with the data (`data/coordinator/<session>/chat`). A Coordinator that runs stays
in it. After its next restart, the Coordinator is in the same conversation. The board opens the conversation again
by its id. All three agent programs find a conversation by its id from the moved folder.

This was tried with Claude Code 2.1.296. Claude Code keys its saved conversations by folder. `--resume` found the
conversation and continued in it. Codex and pi keep their conversations by date and by id.

A program that started before the move still knows its folder by the old path. It saves a new chat (`/clear`,
`/new`) under that path. The board also looks there, so it follows the chat as always.

## Related pages

- [The home folder (reference)](../reference/home-folder.md)
- [Move from an old install](../how-to/move-from-an-old-install.md)
- [What the takeover of an old install does](../reference/old-installs-takeover.md)
- [Move the home folder](../how-to/move-the-home-folder.md)
- [The Coordinator](coordinator.md)
