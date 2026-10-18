# Several attempts, and one kept

This page explains how the board does one task several times at the same time, and why it reviews only the attempt
that it keeps.

## One idea for best of three and for a council

"Best of three" is one agent with three tries. A "council" is different agents, and a judge. Both are the same thing:
a number of attempts, who makes them, and who picks. `attempts.by` says how many attempts and by whom.
`attempts.pick` says who picks. The facts are in [attempts settings](../reference/attempts-settings.md).

## What happens

```
attempts fan out (attempts.by)
  each attempt: build ─► check
─► pick (attempts.pick; the reviewer's comparison)
─► the chosen one: review loop ─► push, pull request ─► merge gate (merge.by)
```

1. **Each attempt works by itself.** It has its own git worktree, its own branch and its own tab in the task's Herdr
   space. Its agent is told the task and nothing about the other attempts. Each attempt uses a slot of `board.agents`.
   With three attempts and two free slots, the third attempt waits, and the row says so.
2. **The check removes the bad attempts.** Each attempt goes through the usual first steps. Its work is committed, main
   is brought in, and the repo's check runs. If the check fails, the work goes back to its agent, up to
   `review.rounds` times. An attempt that asks you something waits for your answer like any agent, in its own tab.
3. **The picker chooses** among the attempts that passed. When it is the reviewer, it reads each change, tries each
   one, says what is good and bad about each, and recommends one.
4. **Only the chosen attempt is reviewed.** It becomes the task: its worktree, branch, agent and conversation are the
   task's from then on. It alone gets the full review loop. It is pushed, gets a pull request and is merged, exactly
   like a task done one time ([how work gets done](how-work-gets-done.md)). Its check does not run again on the same
   commit, unless `check.command` changed in between.
5. **After the pick,** the tabs of the other attempts close. Their worktrees and branches stay until the task is done.
   Then they go with the task (`workspace.cleanup`). What a losing attempt committed goes with it.

### Why only one review

The review is the most expensive step. A full review of every attempt multiplies that cost, for work that the board
then throws away. Thus the check filters the attempts, and only the chosen attempt gets the full review.

### Why some worktrees stay

If a losing attempt's worktree has a file that was never committed, the board keeps that worktree and its branch, and
the task says where. This includes ignored files: a draft, but also `node_modules` or the output of a build. Git cannot
tell these files apart. Thus the board removes nothing of the kind.

## No race, no mix

The board never puts together parts of two attempts in the repo. The attempts do not race: every attempt finishes, and
the board keeps the best one, not the first one.

## Attempts beside the other settings

Attempts stand between the start and the review.

- **A [plan](plan.md)** is made and approved one time, before the attempts start.
- **A [fallback agent](fallback.md)** takes over inside one attempt, one after another, and only on failure. Attempts
  are side by side, and always.
- **[Limits](limits.md):** the attempts are one task. `limit.time` holds for the runs of each attempt. `limit.tokens`
  holds for all the attempts and the task together.

## Related pages

- [Run several attempts](../how-to/run-several-attempts.md)
- [Attempts settings](../reference/attempts-settings.md)
- [How work gets done](how-work-gets-done.md)
- [A plan first](plan.md)
- [The fallback agent](fallback.md)
- [Limits](limits.md)
