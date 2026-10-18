# The rows of the board

This page gives the facts about one row of the board list: its parts, its spacing, its lines, the row of a task that
waits for a free slot, and the row while the board runs the check.

## The parts of a row

A row shows these parts, from left to right:

1. The mark ([marks and colours on a row](board-marks-and-colours.md)).
2. The tag of the task, greyed. Example: `HSW-134`. The tag is the short key of the repo and the number of the task
   ([task numbers and repo keys](task-numbers.md)).
3. The name of the task.
4. On the right, two groups, with a wider gap between them:
   - the progress: the bar, the percent, and the minutes left;
   - the review, the pull request and the time.

```
 needs you
 ! #98  Add an undo for clear                                         R#2 PR#18 12m ago
        Review failed · 1 critical, 1 major
 ready to approve
 ◆ #99  Max length for task titles                                    R#3 PR#60  2m ago
        Pass · 1 minor
 working
 ◐ #134 Tidy the first-run questions            ███░░░░░ 40% 12m left  R#1 PR#17  5m
        Running the tests
 done
 ✓ #76  Blue while the reviewer works                                 R#1 PR#76  1h ago
```

- `R#2` is the review that the task is on, or the last review that it had.
- `PR#18` is the pull request.
- The minutes left say what they are: `25h left`.
- The time after them is how long the task has worked. A time with `ago` is how long since the task stopped.
- A row never repeats what its heading says.

## Spacing

- There is one cell between two parts.
- There is one wider gap, of two cells, to the right of the progress. This gap shows where the progress ends. No other
  gap is as wide.
- Each part is a column down the whole list. Each column is as wide as its widest value in view, and no wider.
- The percent is at least three cells wide (`45%`). Thus it does not move when `9%` changes to `10%`.
- The numbers end under each other. Thus the only other blank cells are before a shorter number: `5m` under `12m`,
  `R#1` under `R#12`.
- A pull request starts under the other pull requests.
- If a row has nothing for a column, that column stays blank on that row.
- The last line of a row on a narrow screen obeys the same rules ([narrow screens](board-narrow-window.md#narrow-screens)).

## Live rows have two lines

A live task is every task under `needs you`, `ready to approve` and `working`. An agent, the reviewer, or the check of
the board works on it, or it waits for you.

- **Line 1**: the mark, the name and the columns.
- **Line 2**: what the task says, greyed. It starts where the name starts. It is one of these:
  - what the task does now;
  - how its review came out, with a short tally, the result in its own colour: `Pass · 2 minor`,
    `Review failed · 1 critical, 1 major`;
  - what the task waits for.
- While the reviewer works, line 2 shows only what the reviewer does.
- Line 2 is in the brighter of the two greys of the board, the `subtext0` of Herdr. The fainter grey, `overlay0`, is
  for the parts that you can miss: times, counts and tags.
- What a task asks of you is in the plain text colour.
- In the view of all machines, the machine is the heading above its tasks, never a part of a row
  ([list headings](list-headings.md)). Thus line 2 starts under the name of the task, as in all other views.
- Line 2 has the whole width of the row, so the board rarely cuts it. If line 2 is longer than the window, the board
  cuts its end with `…`. On a wide screen, line 2 never wraps. On a narrow screen, it wraps
  ([narrow screens](board-narrow-window.md#narrow-screens)).
- Line 2 is there from the start. Thus a row does not grow, and nothing under it moves, when its words arrive.
- Line 2 is never empty and never only `…`. A task with no words of its own shows a short true text:
  - `Just started`;
  - `Yours: no agent on it`, for a task that you marked as your own;
  - `Not reviewed`, for finished work that nobody reviewed;
  - or the plain word for its state.

Done tasks and to-do tasks have one line. If a to-do task has a note, the note shows after its name, on that one line.

## A task that waits for a free slot

At most `board.agents` agents work at the same time. If you start a task while all slots are in use, the task waits
for its turn. Its row shows:

- the mark `◌`, in pink;
- `Waiting for a free slot` on its second line;
- its place in the queue, where a working task shows its percent. `Q#1` gets the next free slot. `Q#2` gets the slot
  after that.

```
 working
 ◐ HSW-151 Rewrite the board in Rust on Ratatui           █████░░░  65%  3m       9m
           Reading the list code
 ◌ HSW-152 Show automations in the main list                         Q#1           2m
        Waiting for a free slot
 ◌ #153 Fix the footer                                               Q#2          40s
        Waiting for a free slot
```

- **The order is the order in which the board starts the tasks.** The first task to wait is the first task to start.
  When a slot becomes free, `Q#1` starts, and the other tasks move up.
- Each attempt of a task waits for a slot of its own. The board counts attempts in the same way.
- **The waiting tasks are last under `working`, in their order.** They have no heading of their own
  ([why](../explanation/board-design.md#tasks-that-wait-for-a-slot)). Their mark, their colour and their `Q#` make them
  different from the tasks at work. Their order shows which task is next.
- If you group by repo, there is no status heading. The waiting tasks are among the tasks of their repo.
- **All places show the same place in the queue:**
  - the State line of the task panel: `◌ waiting for a free slot, Q#2`;
  - `list`: `· waiting for a free slot, Q#2`;
  - `list --json`: `queue: 2`;
  - `details`: `queue: { place, words }`;
  - the message when you start the task;
  - the sidebar of Herdr ([sidebar](sidebar.md));
  - the web board.
- **A narrow board** also shows the place. There, `Q#2` stands with the time at the end of the second line of the row.
  Thus the row of a waiting task has two lines, not three.

## While the board runs the check

At the finish line, and again after a catch-up with main, the row shows real progress, not a guess of the Watcher.

- The bar and the percent are the count that the check reports.
- The minutes left come from how long the last full runs of the check took on this machine.
- How a check reports, and what the own check of a repo can do: [the check's progress](../explanation/check-progress.md).

```
 ◇ Add a greeting                                    █████░░░  65%  3m       9m
   running the check · tests 31 of 47
 ◇ Fix the footer                                                     6m       2m
   waiting for its turn · 2 ahead
```

- **Waiting for its turn**: only one test run at a time runs on a machine. The row shows how many runs are ahead, and
  the minutes. It shows no bar, because nothing of its own moves yet.
- **Code that passed before**: the check answers at once, with `the check already passed for this exact code`. The row
  shows no bar, and the task continues.
- **A check that reports no count**: the row shows the time that the check ran, against the time of the last run:
  `running the check · 2m of about 5m`. It never shows a percent.
- **Nothing moves for much longer than a run takes**: the row says so. Examples:
  - `the check stands still · nothing finished for 13m · tests 31 of 47`
  - `waiting for its turn unusually long · …`
  - `the check runs far longer than usual · …`
- **The check has its own colour.** While the board checks, the mark, the bar, the percent and the minutes left are a
  plain light grey (the `subtext0` of the theme).

The same line, bar, percent and minutes show in the task panel, on the details page, on the chat line, and in the
sidebar of Herdr. The rows of a group still hide the same parts together
([the board in a narrow window](board-narrow-window.md)).

While the board runs its check, the bar, the percent and the minutes left also show on the chat line. This is the chat
line of the task whose agent you talk to. They stand after the words, in the colour of the row. The board never cuts
them. If the line is short, the board removes these, in this order:

1. The words: the board cuts them down to twenty cells.
2. The bar.
3. The numbers.

## Related pages

- [The header and the list](board-header-and-list.md)
- [Marks and colours on a row](board-marks-and-colours.md)
- [The board in a narrow window](board-narrow-window.md)
- [What a key shows at once](../explanation/under-way.md)
- [Task numbers and repo keys](task-numbers.md)
- [The task panel](task-panel.md)
- [The check's progress](../explanation/check-progress.md)
- [One task after another](../explanation/after.md)
- [Limits](limits.md)
- [Why the board looks as it does](../explanation/board-design.md)
