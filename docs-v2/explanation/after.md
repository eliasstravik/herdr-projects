# One task after another

This page explains how a task waits for other tasks (`--after`), what it gets from them, and what occurs when one of
them is not done.

## A condition, not a start

A task can wait for other tasks to be done. This is good for a chain of work, where each step is its own task, with
its own review and merge.

`--after` is only a condition: the task cannot start before the tasks that it names are done. The start is still your
`Tab` (`delegate`), as for any task. Thus there are two kinds of wait:

- **A condition only.** Nobody tabbed the task (`add --after` without `--delegate`, or `after <id> <id>` on a task in
  to do). It stays under **to do** with its `○`. It never starts by itself, not even when those tasks are done. If you
  press `Tab` before they are all done, it is queued. If they are all done, it starts at once.
- **Queued.** Somebody tabbed the task while those tasks were not all done. It is queued, the same as a task that
  waits for a free slot ([the board](../reference/board-header-and-list.md)). It starts by itself when they are all
  done.

**Done** is what the board calls done: merged work, a research task finished with its report, or a task marked done.
What the board shows for each kind is in [`--after` options](../reference/after-options.md).

## While the task waits

- It holds no agent slot (`board.agents`). Nothing runs for it: no worktree, no space, no agent.
- It can wait for tasks of **other repos** (any repo on this machine), and of **other machines** that the board knows.
- It has no place in the queue yet, because it waits for tasks, not for a slot.

The board refuses an id that does not exist. It also refuses a wait in a circle, because such a task would wait
forever.

## When the wait is over

When every task that it waits for is done, a queued task goes to a free slot by itself, like any delegated task. If no
slot is free, it waits for one.

Its agent's first prompt says what came before. For each task that it waited for, it gives:

- its id, title and repo;
- its closing note;
- the pull request and the merge commit where its work merged, or the link to the report for research.

The board takes this when the wait ends, and keeps it on the task. Thus the clean-up of the other task loses none of
it. A task done several times at the same time (`attempts.by`) gives the same to the agent of each attempt.

## When the task cannot continue by itself

If a task that it waits for fails, is stopped, is removed, or is archived without being done, the queued task does not
start. It waits for you under **needs you**, with the reason on its row. The board tells you, as for any task that
needs you (`board.notify`).

None of this is final. If you get the other task going again, the wait continues by itself when that task is done.

## Waiting for a task on another machine

Keys alone can be the same on two machines. Thus you name a task on another machine with its machine first. The board
keeps it in one form and shows it in one form everywhere ([names of tasks](../reference/after-options.md#names-of-tasks)).

- **When you set the wait**, the board asks that machine if the task is there. If the machine is unknown, does not have
  the task, or does not answer at that time, the board refuses the wait and changes nothing.
- **Circles across machines are refused too.** The board follows what the task there waits for in turn, on that
  machine and on any other machine that it names, back to this one. If the board cannot follow it to the end, it
  refuses the wait too: a circle that nobody sees would wait forever.
- **While the task waits**, this machine asks the other machine about it every 15 seconds. It does this in a job of its
  own, so a slow machine stops nothing else. The other machine needs only this version of herdr-projects, not an open
  board. An older version cannot answer, and the board says so.
- **A machine that does not answer does not end the wait.** The task continues to wait, and its row says so. When the
  machine answers again, the line is normal again.
- **Done, failed, stopped, removed or archived without being done** are what that machine says of its task. They have
  the same effect here as for a task of this machine. Done counts. The others make the task wait for you, with the
  reason.
- **When the wait is over,** the agent is told what that task did, as for a task of this machine: its closing note,
  pull request, merge commit or report, and which machine it was on. This comes from what the other machine said last.

## Related pages

- [Chain tasks with `--after`](../how-to/chain-tasks.md)
- [`--after` options](../reference/after-options.md)
- [Run long work in one repo](../how-to/run-long-work-in-one-repo.md)
- [Other machines](other-machines.md)
- [Several attempts](attempts.md)
