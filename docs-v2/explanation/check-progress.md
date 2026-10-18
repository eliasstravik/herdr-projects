# The progress of the check

This page tells what a task shows while the board runs the check of a repo, and where the numbers come from.

## What the board shows, and where

While the board runs the check of a repo, the task shows how far the check has gone. Nothing that the task shows is a
guess, and the board does not ask the Watcher. A check can report a count when it writes to one file. If a check
writes nothing, the task still gets a timer.

The board runs the `check` itself at two times (see [how work gets done](how-work-gets-done.md)):

- at the finish line, when the agent says that it is done;
- after the task catches up with main, before a merge.

The progress shows in each place where the progress of a task shows: its row, the task panel, the details page, the
chat line and the sidebar of Herdr.

## What the task shows

| The check | The line | Bar and percent | Minutes left |
| --- | --- | --- | --- |
| Runs, and reports a count | `Running the check · tests 31 of 47` | The count: 31 of 47 is 65%. | How long full runs took here, minus the time spent. |
| Waits for its turn | `Waiting for its turn · 2 ahead` | None | The runs ahead, then its own run. |
| Answers at once: this code passed before | `The check already passed for this exact code` | None | None |
| Runs, and reports nothing | `Running the check · 2m of about 5m` | None | What it took before, minus the time spent. |
| Reports nothing, and never ran here before | `Running the check · 2m` | None | None |

### The percent

The percent is only the count: the finished items out of the total. It is 100% only when all items are finished.

### The minutes left

- The minutes come from the last runs of the same command, in the same repo, on this machine, that ran in full and
  passed. The board uses the middle value of the last five runs. Thus one slow run on a busy machine does not change
  it.
- If a run had a part of its work done before it started, it counts as that part of a full run.
- When a run is past its usual time, the minutes follow the pace of the run so far. While less than a twentieth of the
  work is done, the board does not show these minutes. If no run was timed yet, the minutes also follow the pace of the
  run.
- While work is left, the minutes are never less than one minute.
- **Waiting:** the board counts one usual run for each run ahead, minus the time since the line last moved. Then it
  adds the run of the check itself. If no run was timed, the board shows no minutes.

### When the check stands still

The board shows a different line when nothing moved for three times as long as a full run takes. This time is at least
three minutes, or ten minutes where no run was timed. Thus the row does not look frozen:

- `The check stands still · nothing finished for 13m · tests 31 of 47`. The bar stays and the minutes go.
- `Waiting for its turn unusually long · nothing moved for 13m · 2 ahead`.
- `The check has not ended · all finished 16m ago · tests 47 of 47`. The board shows this when the count is complete
  but the check does not end.

The board cannot see that a check that reports nothing stands still. It can only see that the check runs long. Past
three times its usual time, and at least ten minutes over, the row says
`The check runs far longer than usual · 16m, usually 2m`.

### The colour

While the board checks, the mark, the bar, the percent and the minutes are a plain light grey. This grey is the
`subtext0` of the theme, under the name `check`. Amber is the colour of the agent, and blue is the colour of the
reviewer. Grey is the colour of the board's own step. It is brighter than the grey words beside it, and it has no hue.

## How a check gives its progress

The board gives the check command one more variable, `HERDR_PROJECTS_PROGRESS`. It holds the name of a file. The check
writes where it is to that file, as often as it wants. Each time, it replaces the whole file. The board reads the file
one time each second.

The progress file is only for the display. Whether the check passed is always its exit status, never what it wrote. To
make a check of your own report its progress, see [make a check report its progress](../how-to/check-progress-report.md).

The check of this repo reports in this way (see
[how the tests work](../contributing/testing.md#the-check-tells-the-board-where-it-is)).

## Related pages

- [Make a check report its progress](../how-to/check-progress-report.md)
- [Shorter checks on fix rounds](shorter-checks.md)
- [How work gets done](how-work-gets-done.md)
- [Colours](colours.md)
- [The task panel](../reference/task-panel.md)
- [How the tests work](../contributing/testing.md)
