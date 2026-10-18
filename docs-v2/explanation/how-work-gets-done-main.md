# Staying current with main

This page tells how the board keeps the branch of each task up to date with main. It also tells when a review stays
valid after a merge of main.

## Why the board does it

Keeping branches up to date is the job of the board, not of the agent. No rule of an agent asks for it. Tasks run in
parallel and touch the same files. Thus the board itself fetches main and merges it into the branch of each task, in
the task's own worktree.

## When the board does it

- **Before the review.** When an agent finishes, the board does it before the check and the review. Thus the reviewer
  sees the work as it will really land.
- **When main moves.** Main moves when another task merges, when someone merges on GitHub, or when you push. The board
  asks the remote every two minutes. The board then updates every task that waits for your approval. It updates a task
  under review when its review ends.
- **After you approve**, one more time, as the last safety net. It almost never finds something.

## A clean update

A clean update is silent. If it changed the commit, the check runs again on the result.

The review does not run again in these cases:

- taking in main is the only change to work that already passed
  ([below](#a-review-that-passed-is-not-repeated-for-a-merge-of-main));
- the update touched none of the files that the task itself changed.

Your verdict stays, and the task does not ask for your attention again.

## A real conflict

The board rolls back a real conflict at once: your branch and the branch of the task stay as they were. The conflict
goes to the agent of the task as a plain message. The message gives the files in conflict and the target commit. It
tells the agent to resolve them in its own worktree and finish again.

- Until the agent did that, the task does not wait for approval. Its row says "resolving a conflict with main".
- If `review.rounds` repairs in a row still conflict, the board asks you. It keeps the details.
- With `review.rounds 0` on a task, the board keeps its conflicts for you. You send them with `Ctrl+R`.
- When a conflict is repaired, the board writes one line about it in `data/conflicts.jsonl` in
  [the home folder](../reference/home-folder.md). The line gives the task, the files, and the minutes from "found" to "up
  to date again". It also gives how many times the agent was sent back, and if a new review followed. The code is
  `crates/flow/src/sync/conflict_log.rs`.

## Only a task at rest

The board updates only a task at rest: its worktree is clean, and it is on exactly the commit that the reviewer saw.

- A fetch that fails never counts as a sync, and it never fails finished work. The board checks and reviews the turn.
  Then it holds the task as "behind main" until a fetch works
  ([fix a task that is behind main](../how-to/troubleshooting-behind-main.md)).
- A finished turn can leave its worktree on a branch that is not the task's own. The board saves it as "Task is on
  the wrong branch".
- The board holds a task that waits as "behind main" in two cases: it cannot fetch the main of the task, or the
  task was not at rest when main moved. Its row says so. The board refuses `Ctrl+L` and gives the reason. It tries
  again every minute until the branch is really up to date.

## What "main" is

"Main" is where the task will land:

- the main of the remote, when the task lands through a pull request;
- the branch that the task started from by name (`workspace.from develop`);
- in other cases (no remote, or `pr` off), the branch that your checkout was on when the task first finished.

For the last case, if you check out another branch, the board merges nothing into tasks that wait. It holds them as
"behind main" until your checkout is on that branch again.

## A review that passed is not repeated for a merge of main

Sometimes the last review passed, and the only change since the commit that it saw is a merge of main. Then the board
runs the `check` of the repo on the result, in place of a new review.

- **The tests pass.** The task goes on to approval or to the merge, as a pass.
- **The tests fail.** The failure goes back to the agent, as a failed check always does. The board reviews the fix.

### The rule for "only the merge of main"

The rule is fixed. The board checks it in git, from the commit that the reviewer saw to the branch as it is now:

1. Every commit since then is a merge commit with two parents: the branch as it was, and a commit of main. An ordinary
   commit on top, a rebase, or a merge of another branch is new work.
2. Each of those merges is exactly what git makes of its two parents by itself. Only the files that git could not merge
   (the conflicts) can be different, because the merge needed their resolution. Any other file that the merge commit
   touches is new work.

Thus the review stays when the agent resolved a conflict in its merge commit and touched only the files in conflict.
The board tells the agent this with the conflict.

The board reviews all other changes as before. It also reviews again everything that it cannot read for sure: a last
review that failed or gave no verdict, and work outside a worktree.

### A repo with no check

A repo with no `check` has no tests that can prove a merge into the files of the task. There, a review stays only
when a clean update touched none of the files that the task changed (the narrower rule above). The task then says
`no check is set` in place of `tests pass`.

### What the task says

The task says clearly what happened:

- its message reads `Re-merged with main, tests pass, review 2 still stands`;
- the box of that review has the same line, with when and how many times;
- the row continues to show the review that passed (`Pass · 2nd review`);
- `details` lists each time under that review as `kept`.

This is fixed behaviour. It is not a setting.

## Related pages

- [How work gets done](how-work-gets-done.md)
- [The gate](gate.md)
- [Rules of the gate](../reference/gate-rules.md)
- [Fix a task that is behind main](../how-to/troubleshooting-behind-main.md)
- [Troubleshooting](../how-to/troubleshoot.md)
