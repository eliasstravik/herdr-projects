# Run several attempts on one task

Use this page to do one task several times at the same time, and keep the best result. "Best of three" (one agent,
three tries) and a "council" (different agents, and a judge) are set up the same way.

**Note:** Every attempt is a whole agent with its own worktree. Each attempt costs what the task costs when it is done
one time. You can start at most 5 attempts.

## Before you start

- The task must be in a repo. A task in no repo is always done one time, because there is no worktree to give each
  attempt.

## Start several attempts

1. To have the usual agent try the task three times, and pick the result yourself, run:

   ```
   # best of three, by the usual agent; you pick
   herdr-projects delegate 12 --attempts.by 3
   ```

2. To have different agents try it, and a named judge pick, give a list of agents and `--attempts.pick reviewer`:

   ```
   # a council of claude, codex and pi, with a named judge
   herdr-projects delegate 12 --attempts.by "claude, codex, pi" --attempts.pick reviewer --reviewer.agent claude/claude-opus-5-5
   ```

3. To use two models for every task in one repo, set `attempts.by` for the repo:

   ```
   # two models for every task in one repo
   herdr-projects set attempts.by "claude, codex/gpt-6.1-sol" --repo my-app
   ```

You can also tell the [Coordinator](../explanation/coordinator.md) in plain words: "do this one three times and let me
choose", "have Claude, Codex and pi each try it and let Opus judge".

**Note:** The board reads `attempts.pick` when the attempts are through. Thus you can still change the picker while the
attempts work.

## Pick the attempt to keep

When several attempts pass the check and `attempts.pick` is `you`, the task waits under **needs you**. It says
`Pick one of 3`, with the reviewer's recommendation if there is one.

1. Select the task on the board.
2. Read the part **Pick one of 3** in the task panel: the reviewer's comparison on top, then each attempt.
3. To move to the next attempt, press `Ctrl+N`.
4. To look into the selected attempt's tab, press `Enter`.
5. To keep the selected attempt, press `Ctrl+L` (the approve key).
6. When the board asks, confirm.

You can also pick from the command line:

```
# which attempt to keep, when the task waits for you
herdr-projects pick 12 B
```

On the web board, the same keys are buttons: `Next attempt` and `Yes`.

## Talk to the attempts, or stop them

- To send words to every attempt that is still in the task, reply to the task while its attempts work.
- To stop all attempts, press `Ctrl+X`.

## If no attempt passed

The task failed with `No attempt passed`. The board keeps the worktree, branch and tab of every attempt, so that you
can look at them.

1. Look at the attempts.
2. To start again, delegate the task again.

   The board starts fresh attempts and removes the old ones.

## Related pages

- [Several attempts](../explanation/attempts.md)
- [Attempts settings](../reference/attempts-settings.md)
- [Get a plan before the build](get-a-plan-first.md)
- [Set limits](set-limits.md)
- [Make a research task](make-a-research-task.md)
- [Keys](../reference/keys.md)
