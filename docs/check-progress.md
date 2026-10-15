# The check's progress

**While the board runs a repo's check, the task shows how far along it is.** Nothing shown is a guess, and the
Watcher is not asked. A check can report a count by writing to one file; one that writes nothing still gets a timer.

The board runs the `check` itself: at the finish line when the agent says it is done, and again after catching up
with main before a merge ([how work gets done](how-work-gets-done.md)). The progress shows everywhere a task's
progress shows: its row, the task panel, the details page, the chat line and Herdr's sidebar.

## What the task shows

| The check | The line | Bar and percent | Minutes left |
|---|---|---|---|
| runs, and reports a count | `Running the check · tests 31 of 47` | the count: 31 of 47 is 65% | how long full runs took here, minus the time spent |
| waits for its turn | `Waiting for its turn · 2 ahead` | none | the runs ahead, then its own |
| answers at once: this code passed before | `The check already passed for this exact code` | none | none |
| runs, and reports nothing | `Running the check · 2m of about 5m` | none | what it took before, minus the time spent |
| reports nothing, and never ran here before | `Running the check · 2m` | none | none |

### The percent

Only ever the count: finished out of total. It is 100% only when all are finished.

### The minutes left

- They come from the last runs of the same command, in the same repo, on this machine, that ran in full and passed.
  The middle one of the last five is used, so one slow run on a busy machine does not move it.
- A run that had part of its work done before it began counts as that share of a full run.
- Once a run is past its usual time, the minutes follow its own pace so far (and are left out while under a twentieth
  is done). With no run timed yet, they follow its own pace too.
- Never under a minute while work is left.
- **Waiting:** one usual run for each run ahead, minus the time since the line last moved, then the check's own run.
  With no timed run, none are shown.

### When it stands still

When nothing has moved for three times as long as a full run takes (at least three minutes; ten where no run was
timed), the row says so instead of looking frozen:

- `The check stands still · nothing finished for 13m · tests 31 of 47` (the bar stays, the minutes go)
- `Waiting for its turn unusually long · nothing moved for 13m · 2 ahead`
- `The check has not ended · all finished 16m ago · tests 47 of 47`, when the count is complete but the check does not
  end.

A check that reports nothing cannot be seen standing still, only running long. Past three times its usual time (and
at least ten minutes over), the row says `The check runs far longer than usual · 16m, usually 2m`.

### The colour

The mark, bar, percent and minutes are a plain light grey while the board checks: the theme's `subtext0`, under the
name `check`. Amber is the agent's colour, blue is the reviewer's, and grey is the board's own step. It is brighter
than the greyed words beside it, and has no hue.

## How a check reports its progress

The board gives the check command one more variable, `HERDR_PROJECTS_PROGRESS`: the name of a file. The check writes
where it is to that file, as often as it likes, each time replacing the whole file. The board reads it once a second.

Write one line, as JSON or plain words:

| Written | Means |
|---|---|
| `{"done": 31, "total": 47, "unit": "tests"}` | 31 of 47 are finished. `unit` is the word shown before the count; leave it out for none |
| `31/47` or `31 of 47 tests` | the same, for a shell script |
| `{"done": 45, "total": 50, "before": 40}` | 40 of them were done before this run began (they passed earlier and did not run again): the run is timed as part of a full one |
| `{"state": "waiting", "ahead": 2}` | it has not started: two runs go first |
| `{"state": "known"}` | answered at once, nothing run: this exact code passed before |

A shell check, for example:

```sh
n=0; total=$(ls tests/*.sh | wc -l)
for t in tests/*.sh; do sh "$t" || exit 1; n=$((n+1)); [ -n "$HERDR_PROJECTS_PROGRESS" ] && echo "$n/$total tests" > "$HERDR_PROJECTS_PROGRESS"; done
```

Good to know:

- Write the file in one go (write a file beside it and rename it, or one short `echo`): the board may read at any
  moment.
- A line the board cannot read changes nothing.
- `done` past `total` counts as `total`.
- The file is the board's own, in the task's folder, and is removed when the check ends.
- Whether the check passed is its exit status, as always, never what it wrote.

This repo's own check reports this way: [testing](testing.md).
