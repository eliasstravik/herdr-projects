# Shorter checks on fix rounds

This page tells why a repo can name a shorter check for fix rounds, and when the board runs it in place of the full
check.

## Why a shorter check

The board runs the check of the repo each time an agent finishes. It runs it the first time, and again on each fix
round. A fix round starts when a review, a failed check or the checks of GitHub send the work back.

If the full check runs on each round, it runs again what already passed. Sometimes GitHub runs the full suite on the
pull request, and the merge waits for it. Then a shorter check is sufficient on the later rounds.

A repo names its shorter check in `check.again`, beside `check.command`. The default is `none`: the full check runs on
each round. To set it, see [set a shorter check for fix rounds](../how-to/shorter-checks-set-up.md).

## When the shorter check runs

The first time that a task finishes, the full check (`check.command`) runs. On a later round of the same task, the
shorter check runs **only when the two conditions that follow are true**. In all other cases, the full check runs. If
the board cannot tell, the full check also runs.

1. **The full check passed on this branch before**, with the same command, and the branch still holds the commit that
   it passed on.
   - If `check.command` changed since then, the full check runs.
   - If the agent made the branch again (with a rebase, or a new start), the commit is gone. Then the full check runs.
2. **The checks of GitHub guard the merge.** This is true when all of these are true:
   - `merge.checks` is `github`;
   - the task has its pull request (`merge.pr`);
   - a workflow of the branch runs for that pull request.

   Then the full suite runs on GitHub on the last commit, and nothing merges before it passes (see
   [how work gets done](how-work-gets-done.md)). If this is not true, each round runs the full check, because it is the
   only full run before the merge.

## What the shorter check gets

`HERDR_PROJECTS_CHECKED` holds the commit that the full check last passed on. Thus the shorter check can run only what
the change since then reaches.

- A shorter check that passes does not move that commit. Thus each round covers the whole change since the last full
  run, with the earlier fix rounds.
- A full check that passes moves that commit.

## While it runs, and after

- The row of the task says `Running the shorter check of a fix round`.
- If the shorter check fails, the work goes back to the agent, as for any failed check, within `review.rounds`. The
  agent gets the command that ran, with the commit in place of the variable.
- The board tells the reviewer which check passed on the commit that it reviews. It also tells the reviewer that the
  full check runs on GitHub before the merge.

## The shorter check of this repo

`npm run check -- --since <commit>` does what the whole check does: it builds the program, writes the made tables,
checks the rules of the repo and runs the tests beside the code. But it runs only the test files that the change since
that commit reaches (see [how the tests work](../contributing/testing.md#the-shorter-check-of-a-fix-round)). If it
cannot tell what a change reaches, it runs all the tests.

## Related pages

- [Set a shorter check for fix rounds](../how-to/shorter-checks-set-up.md)
- [The progress of the check](check-progress.md)
- [How work gets done](how-work-gets-done.md)
- [The gate](gate.md)
- [Settings](../reference/settings.md)
- [How the tests work](../contributing/testing.md)
