# How limits work

This page explains the two limits on what a task costs, `limit.time` and `limit.tokens`. It says what the board
measures, when, and why a limit stops a task but keeps its work.

## A cap, not a loss

Two settings put a cap on what a task can cost. When a task gets to one, the board stops its agent, keeps its work,
and the task waits for you. Nothing is merged. You decide how to continue ([set limits](../how-to/set-limits.md)).

Both limits are `none` until you set them, so a board that sets neither works as before. A repo's own
`.herdr-projects.json` cannot hold them. Thus code that an agent writes can neither set a limit nor remove one.

A stop at a limit never occurs two times by itself. Until somebody continues the task, the board checks it against no
limit.

These settings come from the research of task herdr-projects/154.

## What a run is

`limit.time` is the longest time of one run. A run is one stretch of an agent's work. It starts when the agent is set
going: by you, or by the board with the output of a failed check or the findings of a review. It ends when that turn
ends. For the reviewer, a run is one review.

The clock is the board's own. Whoever keeps the board up to date (the keeper, or an open board) looks at it every couple
of seconds. Thus the limit works when the board is closed. An agent that shows a dialog and waits for your answer is not
stopped because it waits.

## What the tokens are

`limit.tokens` counts the whole task: its agent, every review, every round, and any new chat that its agent started.
The board reads the tokens from the saved conversation of each program. It counts all that the model read and wrote,
every request added up, as the provider counts it.

**What a request reads again from the provider's cache counts too.** A request reads the whole conversation so far,
so the number grows fast. Fifty steps at 100k of context are five million tokens, and a review is easily half a
million. Choose the count with that in mind.

### Why one turn can pass the limit

The board makes the count at the end of each turn, the agent's and the reviewer's. Thus one turn can pass the limit. A
task with `limit.tokens 5m` whose agent uses 6m in its first turn stops at 6m, not 5m. `limit.time` is for a long turn.

### Why a late limit still counts all

While no limit is set for a task, the board reads no conversation at all. It only writes down which conversations the
task had. When you set a limit, the board reads all of them. Thus a limit that you set late still counts the whole
task, with every review that ended before.

A program whose tokens the board cannot count adds nothing. The board can count the three programs whose conversation it
reads: Claude Code, Codex and pi.

## Where the tokens run out

The point in the work where the tokens run out decides what stops. A review that passed does not merge when the tokens
are out, not even with `merge.by itself`. A review of work with nothing to land stops instead of ending as done. The
full list is in [limits](../reference/limits.md#where-the-tokens-run-out).

## Continue, a higher limit, or as it is

- **Continue** is your word. Thus the task has its limit one more time: the run starts at zero, and the tokens count on.
- **A higher limit** continues the task by itself, and nobody is asked. The count goes on from where it stopped.
- **As it is** treats the stopped work like any finished turn: the check, the review, then `merge.by`.

If you stop the task yourself, the limit's mark goes, and the task is simply stopped.

## Limits beside the other settings

- **[Several attempts](attempts.md):** a task done several times at the same time is one task to its limits. Each
  attempt reads its task's limits.
- **[A plan first](plan.md):** the board counts tokens for each conversation file and adds them up on the task. Thus a
  planner's conversation is one more file.
- **[The fallback agent](fallback.md):** a stop at a limit is the board's ordinary stop, with a mark on it. A task
  stopped at its limit is not handed over: it waits for you.

## Related pages

- [Set limits on time and tokens](../how-to/set-limits.md)
- [Limits](../reference/limits.md)
- [Several attempts](attempts.md)
- [A plan first](plan.md)
- [The fallback agent](fallback.md)
- [Limits code](../contributing/limits-code.md)
