# Merge work by itself

This page tells how to make finished work merge without your yes, and how to stop it. By default, finished work waits
for your yes (`merge.by: you`).

## Before you start

With `merge.by: itself`, a task merges at the moment that it is green. Green means all of these:

- the check passed;
- the review passed the gate, or the task has no review;
- the branch is up to date with main, with no conflict;
- the pull request has no failed check or requested change.

**CAUTION:** The setting counts at once, also for tasks under way. If you turn it on for every task, every green task
that waits for your yes merges at once. Look at what waits before you turn it on.

## Turn it on

1. Choose who it is for.
2. Run the command for that choice:

```
herdr-projects set merge.by itself                  every task
herdr-projects set merge.by itself --repo <name>    one repo's tasks
herdr-projects set merge.by itself --task <id>      one task (or: delegate <id> --merge.by itself)
herdr-projects unset merge.by                       take the choice back
```

You can also tell the Coordinator "auto-merge this one" or "always auto-merge".

The task panel shows the setting as `Merge`.

## Turn it on for one repo

You cannot turn it on in the committed file of the repo (`.herdr-projects.json`). Do this:

1. Run `herdr-projects set merge.by itself --repo <name>`.
2. To see what a task really runs on, run `herdr-projects show <id>`. It also tells where each value comes from.

## Let a task go on after you stepped in

A task that you stepped into after it asked you asks you one more time. To tell the board that you do not have to be
asked:

1. Run `herdr-projects set merge.by itself --task <id>` on that task.

## Turn it off

1. Run `herdr-projects unset merge.by`.

A task that waits to merge by itself (for the checks on GitHub or for a merge queue) then waits for your yes again. So
does a task whose merge the board started but did not complete.

## What still asks you

Some things still stop the task and ask you, for example a review that fails after its rounds, or a question from the
agent. See [what still stops and asks you](../reference/gate-rules.md#what-still-stops-and-asks-you).

## Related pages

- [The gate](../explanation/gate.md)
- [Rules of the gate](../reference/gate-rules.md)
- [Delegate a task and merge its work](delegate-and-merge.md)
- [Change a setting](change-a-setting.md)
