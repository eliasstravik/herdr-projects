# One task after another: `--after`

**A task can wait for other tasks to be done.** Use it for a chain of work: change `gtm-skills` first, then take that
into `gtm-agent`, then into the copies downstream. Each is its own task, with its own review and merge.

```sh
herdr-projects add "Take the new skill in" --repo gtm-agent --after gtm-skills/12             # only the condition
herdr-projects add "Take the new skill in" --repo gtm-agent --after gtm-skills/12 --delegate  # queued after it
herdr-projects delegate gtm-agent/4 --after gtm-skills/12,gtm-skills/13
```

Or tell the Coordinator: "then", "once that is merged", "after HSW-12", "after ELA-3 on vm-str" become an `--after` on
the task it adds or delegates. It adds `--delegate` (queues it) only when you asked for the task to be started.

`--after` is only a condition: the task may not start before the tasks it names are done. Starting it is still `Tab`
(`delegate`), as for any task.

| You did | The task |
|---|---|
| `--after` without `Tab` | stays under **to do**, and **never starts by itself** |
| `--after` and `Tab` (`delegate`) | is **queued**, and starts by itself once they are all done |

**Done** is whatever the board calls done: its work merged, a research task finished with its report, or a task marked
done (`report <id> --state done`, with nothing merged). A task archived after it was done still counts as done.

## Not tabbed: a condition only

A task with `--after` that nobody tabbed (`add --after` without `--delegate`, or `after <id> <id>` on a task in to do)
stays under **to do** with its `○`. It never starts by itself, not even once those tasks are done.

- `Tab` starts it any time. If they are not all done yet, it is queued (below). If they are, it starts at once.
- Its row says nothing of the wait. Neither do the web Board and the Herdr sidebar rows.
- The task panel has a `To wait for` line (`To wait for  SKL-6`), where you see and change it.
- `list` shows it beside the title: `after SKL-6`, or `after SKL-4, SKL-5`, naming the ones not done yet (none once
  they all are).

## Tabbed: queued after them

A task tabbed with `--after` while those tasks are not all done is queued, exactly like a task waiting for a free slot
([the board](board.md)):

- the pink `◌` and the queue's colour, under **working** with the queued tasks (after those waiting for a slot);
- its line says `queued after AGT-2` (or `queued after SKL-4, SKL-5`), naming the ones not done yet by their key, as
  everywhere else (`vm-str:ELA-3` for a task of another machine);
- it has no place in the queue (`Q#2`) yet: it waits for tasks, not for a slot;
- the task panel's State says `queued after AGT-2`, and a `Starts after` line lists each task (`(done)` beside those
  that are). `details` has an `after` part, and `list` shows it beside the title;
- the web Board, another machine's board and the Herdr sidebar (`queued after AGT-2`) show the same. Another machine's
  board shows the line and names as the task's own machine says them (its keys, which that board may not know), never
  the bare id (`gtm-agent/2`).

Tasks added with `--after … --delegate` before this existed count as tabbed and show as queued.

While it waits:

- It holds no agent slot (`board.agents`), and nothing runs for it: no worktree, no space, no agent.
- The ids may be tasks of **other repos** (any repo on this machine) and of **other machines** the board knows (below).
- An id that does not exist is refused. So is a wait in a circle (`a` waits for `b`, which waits for `a`): the answer
  shows the circle, `proj/1 → proj/3 → proj/2 → proj/1`, and nothing is changed. An `--after` refused on `add` adds no
  task.

## When the wait is over

Once every task it waits for is done, a tabbed task goes on to a free slot by itself, like any delegated task. With no
slot free, it waits for one: `waiting for a free slot, Q#2`.

Its agent's first prompt says what came before, for each task it waited for: its id, title and repo, its closing note,
and the pull request and merge commit where its work merged, or the report's link for research. This is taken when the
wait ends and kept on the task (`details`: `after.handedOver`), so the other task's clean-up loses none of it. A task
done several times at once (`attempts.by`) hands the same to each attempt's agent.

## When it cannot go on by itself

If a task it waits for fails, is stopped, is removed, or is archived without being done, the tabbed task does not
start. It waits for you under **needs you**, with the reason on its row (`gtm-skills/12 was archived without being done`), and you are told as for any task that needs you (`board.notify`).

Nothing of that is final: get the other task going again, and once it is done the wait goes on by itself.

## Changing it, and starting early

- `herdr-projects after <id> <id>[,<id>…]` sets what a task waits for. On a waiting task, the new list counts from now
  (all done already: it starts). On one not delegated yet, it is kept for when it is.
- `after <id>` says what it waits for.
- `after <id> --clear` drops the wait; a task that was waiting starts at once. `delegate <id> --after none` does the same
  while delegating.
- `delegate` on a task queued after others says it still is, and how to go on: `delegate <id> --now` starts it anyway.
  Its agent is told the tasks were not all done, and what there is of them. On the board (and the web Board), `Tab` on
  such a task asks first: `Start HSW-4 "…" now?`.
- `stop <id>` on a task queued after others stops the waiting (on the board: the stop key, as for any queued task). It
  goes back to to do, and what it waits for is kept as its condition for its next `Tab`. A task that waits for this one
  is told it was stopped, as for any stopped task, until it is delegated again.
- A task an agent is already on cannot be made to wait: the wait is before the start.

## Waiting for a task on another machine

A task can wait for tasks on any other machine the board knows (`board.machines`, or Herdr's own list of machines:
[other machines](other-machines.md)). Keys alone may clash across machines, so name such a task with its machine first:
`vm-str:ELA-3`, `vm-str:ela/3` or `vm-str#3`. It is kept as `vm-str:ela/3`, and shown as `vm-str:ELA-3` everywhere the
board says what a task waits for (its row, its panel, `details`, `list`, another machine's board).

```sh
herdr-projects delegate notably/3 --after vm-str:ELA-3
herdr-projects add "Use the new API" --after vm-str:ELA-3,HSW-12 --delegate
```

- **When you set the wait**, the board asks that machine whether the task is there. It refuses an unknown machine, a
  task that machine does not have, and a machine that does not answer right then, and changes nothing.
- **Circles across machines are refused too.** The board follows what the task there waits for in turn, on that machine
  and any other one it names, back to this one. If that cannot be followed to the end (a machine on the way does not
  answer, or names a machine this board does not know), the wait is refused as well: an unseen circle would wait
  forever.
- **While it waits**, this machine asks the other one about it every 15 seconds, in a job of its own (so a slow machine
  holds nothing else up). The other machine needs only this version of herdr-projects, not an open board. An older
  version cannot answer, and the board says so, with "update it there".
- **A machine that does not answer does not end the wait.** The task keeps waiting and says so on its row:
  `queued after vm-str:ELA-3 · waiting for vm-str: not reachable since 10:42` (`details`: `after.notReachable`). Once it
  answers again, the line is back to normal.
- **Done, failed, stopped, removed or archived without being done** are what that machine says of its task, and do here
  what they do for a task of this machine: done counts, the others make the task wait for you with the reason
  (`vm-str:ELA-3 was stopped`).
- **When the wait is over,** the agent is told what that task did, as for one of this machine (its closing note, pull
  request, merge commit or report, and which machine it was on), from what the other machine said last.
