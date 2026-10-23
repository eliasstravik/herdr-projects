# Disk space

**A task cleans up after itself.** Everything temporary its agent and its reviewers make goes into one folder of the
task's own, and that folder goes when the task ends. What tasks that ended before left behind is swept away by itself,
and a task that would start on a nearly full disk waits instead of failing half-way through its build.

## A task's own temporary folder

Each task gets a short folder of its own, such as `/tmp/hp-1000/3f9a2c/HP-96` (your user id, then one folder per board,
then the task's name). Its agent and every reviewer start with it as `TMPDIR` (and `TMP`, `TEMP`,
`CLAUDE_CODE_TMPDIR`), so the test projects, previews, venvs and smoke installs they make land there, and so does what
their own tools keep: Claude Code's scratchpad is inside it. They are told so in their rules, and told to stop the
servers they started before they finish (unless you asked them to keep one running).

## When a task ends

Done, archived or removed, once its agent is gone with its space:

1. whatever still runs from its temporary folder (a dev server the agent or a reviewer started there) is stopped;
2. its temporary folder is removed, and so are the folders an agent's program kept for its worktree outside it (Claude
   Code's `/tmp/claude-<uid>/<worktree path with dashes>` of an agent started before this);
3. its line says what that freed: `Freed 1.4 GB of temporary files.`

A worktree that is kept (it holds unsaved or unmerged work, or `workspace.cleanup` is off) stays as it is: only the
temporary files go. A done task whose agent stays open (`workspace.cleanup` off) loses them too, all but the folders
its program keeps for itself (Claude Code's scratchpad folder, emptied), so the agent goes on if you talk to it again;
what it makes after that goes in the sweep once nobody talked to it for an hour. While its agent works, nothing is
touched.

### What it left running

Right before the worktree of a done or removed task goes, what still runs from that worktree or from its temporary
folder is ended, by the rules of `stop` ([roles and commands](roles-and-commands.md)): the dev server or mock gateway
its agent started for a visual check, also one that left the agent's terminal (handed to init, so nothing showed its
space in use), and what its agent launched in its terminal. Never what runs in another of its panes (yours, its
reviewer's), the board's own runs, or anything in your own checkout; and where herdr does not say what runs in its
panes, nothing from its folders. Its line says what went, and names what still runs for you to end: what is not the
board's to end, what would not end, what was started for the task and runs elsewhere now, and, where herdr does not
say, what runs from its folders:

```
Merged into main. Ended what it started: node …/vite.js (pid 81234). Still running, end it yourself: node …/mock.js
(pid 81240), in /home/me/wt/proj/work-12-x, from another of its panes.
```

`herdr-projects details <id>` lists what still runs under `stillRunning`. Removing a task (`clear`) says the same in
its answer. A worktree that is kept ends nothing. Archive keeps everything, so it ends nothing either: its line only
names what still runs from the worktree it keeps (`Still running, end it yourself: node …/vite.js (pid 81234), in …`).

## The sweep

The sweep removes what tasks that ended before left: it runs when the board opens (when the last sweep is over an hour
old), once a day by itself, before a start on a nearly full disk, and when you run `herdr-projects sweep`, which says
what it removed (`herdr-projects sweep --dry` only says what would go). What still runs from a folder it takes (a dev server left behind) is stopped first. It takes:

- the temporary folders of done and archived tasks whose agent is gone (or open and quiet for an hour, as above);
- the folders an agent's program kept for a worktree of the board's that is gone;
- the board's task folders whose task was removed;
- the build output (`target/`, `node_modules/`, `.next/`, and only what git ignores) of a worktree a done or archived
  task kept, once nothing changed on the task for `board.oldBuilds` days (7; `0`: never);
- old warm builds in `~/.cache/herdr-projects` that nothing uses.

**What it never touches:** a task at work, its worktree and its folders; your repos and your home folder; a loose folder
in `/tmp` the board did not make; a folder that changed in the last hour; a folder your environment points into (a `PATH` entry in an old scratchpad keeps that one); and anything
reached through a link: a folder goes only where it and each folder above it, up to the temporary folder, the build
caches or the worktree folder it is in, is a real folder. A link in a temporary folder's place is taken away, and
what it points to, and what runs there, stays. An agent's
folder for a worktree counts as the board's only when its name is a known worktree folder followed by a name as the
task branches start (`work-`), no folder of that name is there, and no other folder beside it starts so. What the
last sweep removed is kept in `~/.herdr-projects/data/sweep.json`.

## A nearly full disk

Before a task starts (its worktree, its setup, its agent, and the build once its plan has its yes), the board looks at the disks it writes to: the repo's, the
worktrees', the board's data and the temporary files. Under `board.diskFree` GB free (5; `0`: never looked at), it
sweeps first; still under, the task goes back to the queue with a plain line instead of failing with "No space left on
device":

```
Disk nearly full on vm-str: 778 MB free on /; this task starts once 5.0 GB is free (board.diskFree). Free some space:
herdr-projects sweep, and herdr-projects doctor lists more that can go.
```

It starts by itself once there is room. `herdr-projects machines` says how much each machine has free, and
`herdr-projects doctor` lists each disk and the leftovers it could clean.

### A small disk

A disk never has to keep more than half of itself free. A `/tmp` that is a small memory disk (3.4 GB in all; many Linux
machines give `/tmp` half their memory) could never have 5 GB free, and every task would wait forever: on it, a task
needs 1.7 GB free, and its line says why it is less than `board.diskFree`:

```
Disk nearly full on vm-str: 1.2 GB free on /tmp; this task starts once 1.7 GB is free (/tmp is only 3.4 GB in all, so
it needs 1.7 GB free, not the 5.0 GB of board.diskFree). Free some space: herdr-projects sweep, …
```

`herdr-projects doctor` measures such a disk the same way, so the two never disagree.

### What a waiting task says

A task that waits for room keeps its place in the queue, and says that it waits for disk space, not for a free slot:

- its row on the board and on the web board: the whole `Disk nearly full on …` line under its name, its place (`Q#1`)
  on the right;
- the task panel's State line, `list`, `details` and the line when you start it: `waiting for disk space, Q#1`;
- herdr's sidebar: `waiting for disk space`, with its place;
- `herdr-projects doctor`, beside the disk: `Only 1.2 GB free on /tmp, which holds temporary files: HP-12 waits for
  room on /tmp, and new tasks wait too`.

A task that waits for a slot of `board.agents` still says `waiting for a free slot, Q#2`.
