# Herdr's sidebar

This page tells what the board writes in Herdr's sidebar for each live agent. For how the board writes it and why some
things are not possible, read [how the board writes Herdr's sidebar](../explanation/sidebar.md).

## What the sidebar shows

**Herdr's sidebar shows a small copy of the board row of each live agent**, with the same words:

- the number and the title of the task;
- who is on it and its state;
- the percent and the time to go.

The second line of the row (what the agent says it does, for example `Reading the list code`, or the question
itself) is not in the sidebar. It stays on the board, because row 2 of the entry already shows the state.

There is one entry for each live agent: the agent of a task, its reviewer while a review runs, and the Coordinator. A
task with nobody on it (to do, or done and cleaned up) has no pane in Herdr, so it has no entry.

**A done task leaves the sidebar automatically.** Its whole space closes when one of these occurs:

- it is merged (here or on GitHub);
- it is finished with nothing to merge;
- you mark it as done.

The setting `workspace.cleanup` controls this. It is on, unless you turned it off. A space that stays has a reason, and
the line of the task gives it (`Kept: … (uncommitted changes)`). See
[when a task is done](../explanation/how-work-gets-done-clean-up.md#the-clean-up) and
[a done task stays in the Herdr sidebar](../how-to/troubleshoot.md#a-done-task-stays-in-the-herdr-sidebar).

To get all three rows, let the board add its sidebar rows to Herdr's config on its first screen
([install and update](../how-to/install-and-update.md)).

## The three rows of an entry

```
 ◐ HSW-152 Show automations in the m…      1. Herdr's dot, the task's number and title
   agent · working · R#1 PR#113 · 6m       2. who is on it, where it stands, review round, pull request, tags, time
   8% 16m                                  3. the percent and the time to go
```

### Row 1: Herdr's dot, the number and the title

- The name is the same as on the board: `HSW-152 …`, the key of the repo and the number
  ([task numbers](task-numbers.md)).
- The number is first, and Herdr cuts a row at its end. Thus a narrow sidebar cuts the title, never the number.
- The name of the machine is before the number only where the sidebar shows more than one machine. That is Herdr's own
  `machine` token.

### Row 2: who is on it and its state

The words come from the one list of status words.

| The row on the board | Row 2 of the entry |
|---|---|
| `◐` the agent works | `agent · working` |
| `◑` under review | `reviewer · under review` on the entry of the reviewer, `agent · under review` on the entry of the agent |
| `◇` the board starts it, sets up, checks, pushes, merges, or waits on the way (the turn of its check, its turn to merge, the checks of GitHub) | `board · starting`, `board · setting up`, `board · checking`, `board · pushing`, `board · merging` |
| `◌` waits for a free slot, `Q#2` | `queued`, and row 3 shows `Q#2`: its place in the queue, where a working task has its percent |
| `◌` queued after other tasks ([one task after another](../explanation/after.md)) | `queued after SKL-6` |
| `?` asks you | `asks you` |
| `!` asks for your OK | `asks for your OK` |
| the sign-in mark | `needs a login` |
| `◆` passed, waits for your yes | `ready to approve` |
| `!` its review failed | `review failed` |
| `×` went wrong (a failure, or a merge or a pull request that failed) | `went wrong` |
| finished, but the board cannot bring its branch up to date | `behind main` |
| waits for you, with no kind (a dialog of a reviewer, stopped without a summary) | `needs you` |
| stopped, put back | `to do` |
| done, while its space is still open | `done` |

- The entry names who is on the task only while somebody is on it. A waiting task shows what it waits for.
- After the state come:
  1. the review round and the pull request, as the row shows them (`R#1 PR#113`);
  2. the tags of the task (`[research]`, `[auto]`);
  3. the time of the row: `6m` for how long it has worked, `2m ago` for how long it has waited.
- The time changes once a minute (`<1m` before the first minute). Herdr shows the last value that it got, and the board
  does not write every second. The time is last, so a narrow sidebar cuts it first.

### Row 3: the numbers alone

- The percent (or the place in the queue) and the time to go, where the row shows them: `8% 16m`.
- From a thousand minutes, the time is in hours: `20h`.
- Never the word `left`. The sidebar is narrow, and a number after a percent can only be the time to go.
- Empty when the row has no numbers.

### The reviewer, the Coordinator and the spaces

- **The entry of the reviewer** names the task in the same way, with `reviewer` as who. During a review, the task has
  two entries, each for its own pane. `reviewer · under review` is the entry of the reviewer. `agent · under review` is
  the entry of the agent whose work is under review. Their other words are the same, and each has its own dot.
- **The Coordinator** has the name `Coordinator`. Its entry shows what the line above the box of the board shows:
  `Coordinator · idle`, `Coordinator · working`, `Coordinator · asks you`. It also shows what waits to go in
  (`· 1 message waits to be sent`), and its latest message under that. Herdr's own words for it would be
  `sw-coordinator` and `done`.
- **The spaces part** has the same rule. The name of the task's space is the number and the title of the task
  (`HSW-152 Show automations…`), with the percent and the time to go of the row under it. A task that waits for you to
  pick one of its attempts has no percent and time. The entries of its attempts show `pick one`.

## With and without the sidebar rows of the plugin

Herdr's default sidebar shows the name of a space, the name of the tab and the shown name of the agent. It shows no
custom value.

| | With the rows ([install and update](../how-to/install-and-update.md)) | Herdr's default rows |
|---|---|---|
| Row 1 | `HSW-152 Show automations…` | `HSW-152 Show automations… · Agent`: also the name of the tab |
| Row 2 | `agent · working · R#1 PR#113` | the same |
| Row 3 | `8% 16m` | not shown: Herdr's default rows show no custom value |
| A space | its name, its branch, the percent and the time to go | its name and its branch |

- The rows of the plugin hide the names of the task's own tabs (`Agent`, `Reviewer #1`), because the next row gives the
  same information ([tabs](tabs.md)).
- A tab of your own with the exact name `Agent` is also hidden on row 1.
- An agent that the board did not start keeps Herdr's own entry: its space, its tab, its name.

## What is Herdr's own

- **The dot is Herdr's**, in Herdr's colours. It is Herdr's own reading of the pane: working, idle, done, blocked. The
  board writes no state to Herdr. Thus a task that asks you has Herdr's done or idle dot, and the words next to it give
  the exact state.
- **Herdr cuts the rows.** Herdr shares the width of a row between its values and ends a cut value with `…`.
  - At the default width of 26 cells, Herdr cuts a long row. `sidebar_width = 36` under `[ui]` in Herdr's config shows
    more.
  - Herdr keeps at most 80 characters of a value. The board cuts a longer line itself, with `…`.

## Limits with Herdr 0.9.3

- **The first row of the Coordinator is `~ · Coordinator`**, not `Coordinator` alone.
- **No marks of the board's own.** The `?`, `!`, `◑`, `◇`, `◌` and `◆` of the row do not show. The words give the state.
- **No colour by status.** The rows of the plugin only dim the message of the Coordinator, as the board greys it.
- **No bar.** The percent and the time to go show, but the bar of the row does not.
- **Not in the collapsed sidebar or in a window of phone width.** These keep Herdr's own compact layouts. This was not
  tested here.
- **The tasks of another machine** show here only when this Herdr connects to that machine, and that machine runs this
  version of the plugin.

For the reasons, see [how the board writes Herdr's sidebar](../explanation/sidebar.md#limits-with-herdr-093).

## For contributors

- The one list of status words: [`crates/core/src/status_words.rs`](../../crates/core/src/status_words.rs).
- The one writer: [`crates/flow/src/flow/sidebar.rs`](../../crates/flow/src/flow/sidebar.rs).
- The row of an entry: [`crates/flow/src/flow/sidebar_entry.rs`](../../crates/flow/src/flow/sidebar_entry.rs).
- The Herdr commands and tokens: see
  [how the board writes Herdr's sidebar](../explanation/sidebar.md#how-the-board-writes-the-entries).

## Related pages

- [How the board writes Herdr's sidebar](../explanation/sidebar.md)
- [A task's tabs in Herdr](tabs.md)
- [Task numbers](task-numbers.md)
- [Install and update](../how-to/install-and-update.md)
- [Troubleshoot](../how-to/troubleshoot.md)
- [When a task is done](../explanation/how-work-gets-done-clean-up.md)
