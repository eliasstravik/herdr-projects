# Behind main

This page tells how the board brings main into the branch of a task, and what happens when it cannot fetch main. To fix
a task that is `behind main`, read [fix a task that is behind main](../how-to/troubleshooting-behind-main.md).

## When the board fetches main

The board fetches main and merges it into the branch of the task at two times:

- before the board reviews a finished task;
- again immediately before the task merges.

## When the fetch fails

If the fetch fails, the board fails nothing. It checks and reviews the task as usual. Then the task waits as
`behind main`. You cannot approve the task until it is up to date. The board tries again by itself each minute.

The remote can be unavailable. Then the board cannot push the branch either. The task waits in the same way before its
push and its review. The push and the review run by themselves when the remote answers.

A merge that you approved waits in the same way, and the board merges nothing. When the row is ready again, you approve
the merge again.

## Many tasks of one repo at the same time

Many tasks of one repo can finish at the same time. They never stop each other:

- All the worktrees of a repo write the same `origin/main`. Thus the tasks share one copy of main.
- The board fetches main for one task at a time.
- A task that waited for the fetch of another task uses the result of that fetch.

Another program can fetch at the same time (the `git fetch` of an agent, your own, or that of an editor). If git
refuses a fetch for this reason, the board tries again after 2 seconds and after 4 seconds. A copy that the other
program just fetched counts.

The main branch of a remote can have a different name than `main`, and your clone can have no name for it (no
`origin/HEAD`, as after `git remote add`). Then the board asks the remote which branch it is, and fetches that branch,
one time.

## Related pages

- [Fix a task that is behind main](../how-to/troubleshooting-behind-main.md)
- [How work gets done](how-work-gets-done.md)
- [The gate](gate.md)
