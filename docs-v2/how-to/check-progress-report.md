# Make a check report its progress

This page tells you how to make the check of your repo tell the board how far it has gone. The board then shows a
count, a bar and a percent on the task.

## Before you start

The board gives the check command the variable `HERDR_PROJECTS_PROGRESS`. It holds the name of a file. The board reads
that file one time each second. For what the board shows, see [the progress of the check](../explanation/check-progress.md).

**Note:** A check that writes nothing still gets a timer on the task.

## Write the progress

**CAUTION:** The board can read the file at any moment. Write the file in one operation: write a file beside it and
rename it, or use one short `echo`.

1. In your check, find the places where the work moves on.
2. At each of these places, test that `HERDR_PROJECTS_PROGRESS` is set.
3. Write one line to the file that `HERDR_PROJECTS_PROGRESS` names. Replace the whole file each time.

You can write the file as often as you want.

## What to write

Write one line, as JSON or as plain words:

| Written | Means |
| --- | --- |
| `{"done": 31, "total": 47, "unit": "tests"}` | 31 of 47 are finished. `unit` is the word shown before the count. Leave it out for no word. |
| `31/47` or `31 of 47 tests` | The same, for a shell script. |
| `{"done": 45, "total": 50, "before": 40}` | 40 of them were done before this run started: they passed earlier and did not run again. The board times the run as a part of a full run. |
| `{"state": "waiting", "ahead": 2}` | The check has not started: two runs go first. |
| `{"state": "known"}` | The check answered at once and ran nothing: this exact code passed before. |

## An example

A shell check, for example:

```sh
n=0; total=$(ls tests/*.sh | wc -l)
for t in tests/*.sh; do sh "$t" || exit 1; n=$((n+1)); [ -n "$HERDR_PROJECTS_PROGRESS" ] && echo "$n/$total tests" > "$HERDR_PROJECTS_PROGRESS"; done
```

## What the board does with the file

- If the board cannot read a line, nothing changes.
- If `done` is more than `total`, the board counts it as `total`.
- The file belongs to the board. It is in the folder of the task. The board removes it when the check ends.
- Whether the check passed is its exit status, as always. It is never what the check wrote.

## Related pages

- [The progress of the check](../explanation/check-progress.md)
- [Shorter checks on fix rounds](../explanation/shorter-checks.md)
- [How the tests work](../contributing/testing.md)
