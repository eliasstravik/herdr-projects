# `--after` options

This page gives the facts about a task that waits for other tasks: the flags and commands, the states, what the board
shows, and the names of tasks on other machines.

## Flags and commands

| Flag or command | What it does |
|---|---|
| `add … --after <id>[,<id>…]` | Adds a task with the condition only. It stays under **to do** |
| `add … --after <id>[,<id>…] --delegate` | Adds the task and queues it after those tasks |
| `delegate <id> --after <id>[,<id>…]` | Delegates the task, queued after those tasks |
| `delegate <id> --after none` | Removes the wait while it delegates |
| `delegate <id> --now` | Starts a queued task at once. Its agent is told that the tasks were not all done, and what there is of them |
| `herdr-projects after <id> <id>[,<id>…]` | Sets what a task waits for. On a waiting task, the new list counts from now: if all are done already, it starts. On a task not delegated yet, the board keeps it for when it is |
| `after <id>` | Says what the task waits for |
| `after <id> --clear` | Removes the wait. A task that was waiting starts at once |
| `stop <id>` | On a queued task: stops the wait (on the board: the stop key, as for any queued task) |

- `delegate` on a task that is queued after others says that it still is, and how to continue: `delegate <id> --now`.
- `Tab` on such a task, on the board and on the web board, asks first: `Start HSW-4 "…" now?`.
- `stop <id>` sends the task back to **to do**. The board keeps what it waits for as its condition for its next `Tab`.
  A task that waits for this one is told that it was stopped, as for any stopped task, until it is delegated again.
- A task that an agent already works on cannot be made to wait. The wait comes before the start.

## The two kinds of wait

| You did | The task |
|---|---|
| `--after` without `Tab` | stays under **to do**, and **never starts by itself** |
| `--after` and `Tab` (`delegate`) | is **queued**, and starts by itself when they are all done |

Tasks added with `--after … --delegate` before this feature existed count as tabbed, and show as queued.

## What counts as done

**Done** is what the board calls done:

- its work merged;
- a research task finished with its report;
- a task marked done (`report <id> --state done`, with nothing merged).

A task archived after it was done still counts as done.

## What the board shows

### A condition only (not tabbed)

| Where | What it shows |
|---|---|
| The row | The task is under **to do**, with its `○`. The row says nothing of the wait |
| The web board, the Herdr sidebar | Nothing of the wait |
| The task panel | A `To wait for` line (`To wait for  SKL-6`), where you see and change it |
| `list` | Beside the title: `after SKL-6`, or `after SKL-4, SKL-5`. It names the tasks not done yet (none when all are done) |

### Queued after other tasks (tabbed)

| Where | What it shows |
|---|---|
| The row | The pink `◌` and the queue's colour, under **working**, with the queued tasks (after those that wait for a slot) |
| The row's line | `queued after AGT-2`, or `queued after SKL-4, SKL-5`. It names the tasks not done yet by their key (`vm-str:ELA-3` for a task of another machine) |
| The place in the queue | None yet (no `Q#2`): it waits for tasks, not for a slot |
| The task panel | State says `queued after AGT-2`. A `Starts after` line lists each task, with `(done)` beside those that are |
| `details` | An `after` part |
| `list` | Beside the title |
| The web board, another machine's board, the Herdr sidebar | The same (`queued after AGT-2`) |

Another machine's board shows the line and the names as the task's own machine says them (its keys, which that board
does not always know). It never shows the bare id (`gtm-agent/2`).

### After the wait

| Case | What it shows |
|---|---|
| All done, no free slot | `waiting for a free slot, Q#2` |
| A task it waits for failed, was stopped, removed or archived without being done | Under **needs you**, with the reason on the row: `gtm-skills/12 was archived without being done`. `board.notify` tells you |
| A task on another machine was stopped | `vm-str:ELA-3 was stopped` |
| The other machine does not answer | `queued after vm-str:ELA-3 · waiting for vm-str: not reachable since 10:42` (`details`: `after.notReachable`) |
| What the agent was told | `details`: `after.handedOver` |

## Refused waits

The board refuses these, and changes nothing:

- An id that does not exist.
- A wait in a circle (`a` waits for `b`, which waits for `a`). The answer shows the circle:
  `proj/1 → proj/3 → proj/2 → proj/1`.
- On another machine: an unknown machine, a task that the machine does not have, and a machine that does not answer
  at that time.
- A circle across machines, and a wait that the board cannot follow to the end (a machine on the way does not answer,
  or names a machine that this board does not know).
An `--after` that the board refuses on `add` adds no task.

A machine with an older version of herdr-projects cannot answer. The board says so, with "update it there".

## Names of tasks

| What | Form |
|---|---|
| A task of any repo on this machine | its id or its key, for example `gtm-skills/12`, `HSW-12`, `SKL-6` |
| A task on another machine | the machine first: `vm-str:ELA-3`, `vm-str:ela/3` or `vm-str#3` |
| How the board keeps a task on another machine | `vm-str:ela/3` |
| How the board shows it | `vm-str:ELA-3`: on the row, the panel, `details`, `list`, another machine's board |

The board knows the machines in `board.machines`, and in Herdr's own list of machines
([other machines](../explanation/other-machines.md)).

## Timing

| What | When |
|---|---|
| The board asks another machine about a task it waits for | Every 15 seconds, in a job of its own |
| The board takes what came before, for the agent's first prompt | When the wait ends |

## Related pages

- [Chain tasks with `--after`](../how-to/chain-tasks.md)
- [One task after another](../explanation/after.md)
- [Commands](commands.md)
- [Board header and list](board-header-and-list.md)
- [Other machines](../explanation/other-machines.md)
