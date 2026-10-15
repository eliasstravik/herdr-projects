# Shorter checks on fix rounds

**A repo can name a shorter check for fix rounds**: `check.again`, beside `check.command`.

```json
{ "check": { "command": "npm run check -- --all", "again": "npm run check -- --since $HERDR_PROJECTS_CHECKED" } }
```

The board runs the repo's check every time an agent finishes: the first time, and again on every fix round (when a
review, a failed check or GitHub's checks sent the work back). Running the full check every round repeats what already
passed. When GitHub runs the full suite on the pull request anyway, and the merge waits for it, a shorter check is
enough on the later rounds.

The default is `none`: the full check every round.

## When the shorter check runs

The first time a task finishes, the full check (`check.command`) runs. On a later round of the same task, the shorter
check runs instead **only when both of these hold**. Otherwise, or when the board cannot tell, the full check runs.

1. **The full check passed on this branch before**, with the same command, and the branch still holds the commit it
   passed on.
   - If `check.command` changed since, the full check runs.
   - If the agent rebuilt the branch (rebased, started again), the commit is gone and the full check runs.
2. **GitHub's checks guard the merge**: `merge.checks` is `github`, the task has its pull request (`merge.pr`), and a
   workflow of the branch runs for that pull request. Then the full suite runs on GitHub on the last commit, and
   nothing merges before it passes ([how work gets done](how-work-gets-done.md)). Without that, every round runs the
   full check, since it is the only full run before the merge.

## What the shorter check is told

`HERDR_PROJECTS_CHECKED` holds the commit the full check last passed on, so the shorter check can run only what the
change since then reaches.

- A shorter check that passes does not move that commit. So each round covers the whole change since the last full
  run, the earlier fix rounds included.
- A full check that passes does move it.

## While it runs, and after

- The task's row says `Running the shorter check of a fix round`.
- A failed shorter check goes back to the agent like any failed check, within `review.rounds`, with the command that
  ran (the commit filled in for the variable).
- The reviewer is told which check passed on the commit it reviews, and that the full one runs on GitHub before the
  merge.

## This repo's shorter check

`npm run check -- --since <commit>` does what the whole check does (builds the program, writes the made tables, checks
the repo's rules, runs the tests beside the code), but runs only the test files the change since that commit reaches
([testing](testing.md#the-shorter-check-of-a-fix-round)). Where it cannot tell what a change reaches, it runs every
test.
