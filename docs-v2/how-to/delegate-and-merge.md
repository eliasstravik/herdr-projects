# Delegate a task and merge its work

This page tells how to put an agent on a task, follow it, and merge its work. For how each step works, see
[how work gets done](../explanation/how-work-gets-done.md).

## Delegate a task

1. On the board, select the task with `↑` or `↓`.
2. Press `Tab`.

You can also run `herdr-projects delegate <id>`.

In a git repo, the agent works in its own worktree, on its own branch. The branch starts from the default branch of
the remote.

### Start from another place

- To start from your own commit, run `herdr-projects delegate <id> --workspace.from here`.
- To start from another branch at the remote, run `herdr-projects delegate <id> --workspace.from develop`.

**Note:** A task started from a named branch needs `merge.pr` on. With `merge.pr` off, the board refuses it.

### Choose the name of the branch

1. Set `workspace.branch` for you, for one repo, or for one task. You cannot set it in the repo's own file.

   Example:

   ```
   herdr-projects set workspace.branch "feat/{key}-{slug}"
   ```

   This makes names such as `feat/KIC-7-fix-the-login`. The placeholders are in
   [how work gets done](../explanation/how-work-gets-done.md#the-name-of-the-branch).

### Make the agent ask before it acts

By default, agents run with full access.

**CAUTION:** An agent with full access can run any command as you, and it does not ask first.

1. Run `herdr-projects set worker.access ask`.

For the reviewer and the Coordinator, set `reviewer.access` and `coordinator.access`. pi cannot ask, so the board
refuses this value for pi.

## Follow the task

- The row of the task tells where the task is.
- To see the real pane of the agent, select the task and press `Enter`.
- To see the task beside the list, press `Alt+P`.

## Answer the agent

If the agent waits for you, its row shows a mark (`?`, `!` or the sign-in mark).

1. Select the task.
2. Press `Space`.
3. Type your answer.
4. Press `Enter`.

## If the task does not go on

| The row says | Do this |
| --- | --- |
| **Not started: its instructions sit unsent in its box** | Press `Ctrl+L`, or run `herdr-projects retry <id>`. You can also go into the agent and press `Enter`. |
| "Stopped without a summary" | Press `Ctrl+R` to tell the agent to continue. Or press `Ctrl+L` to take the work as it is. |
| "Finishing", but the agent is at rest | Press `Ctrl+L`, or run `herdr-projects finish <id>`. The last thing that the agent said is the summary. |
| "behind main" | Wait. The board tries again every minute. `Ctrl+L` is refused until the branch is up to date. |
| "resolving a conflict with main" | Wait. The agent repairs the conflict. |

## Read the review

When the agent finishes, the board checks the work, opens a pull request and has a reviewer review it.

- The row shows the result and a tally, for example `Pass · 2 minor`.
- The tag `R#2` tells which review it was. Its colour tells how it went.
- The task panel shows the summary of the review, and every earlier review.

If a review fails, the board sends it back to the agent by itself, at most `review.rounds` times. After that, the task
waits for you.

1. To send the findings back to the agent, press `Ctrl+R`.

## Approve and merge

1. Select the task under `ready to approve`.
2. Press `Ctrl+L`.
3. Read the question. A merge cannot be undone.
4. Press `y` to merge, or `n` to stop.

The board brings the branch up to date with main one more time. If that changed something, it checks and reviews the
work again. Then it merges.

**Note:** A task whose review failed is under `needs you`, not under `ready to approve`. To merge it anyway, press
`Ctrl+L` and then `y` (the question says `y = merge anyway`). You can also run `herdr-projects approve <id> --anyway`.

If the checks on GitHub failed after the rounds, `herdr-projects approve <id>` refuses and tells why. To merge anyway,
run `herdr-projects approve <id> --anyway`.

## After the merge

The space of the task leaves the sidebar of Herdr. The board removes its worktree and its branch.

- To keep them, set `workspace.cleanup false`.
- If an agent of unknown status keeps a command of its own running, the board never closes it by itself. Archive the
  task (`Ctrl+A`), or remove it (`Ctrl+X`).

## Related pages

- [Your first task](../tutorials/first-task.md)
- [How work gets done](../explanation/how-work-gets-done.md)
- [Merge work by itself](turn-on-auto-merge.md)
- [Rules of the gate](../reference/gate-rules.md)
- [Handing text to an agent](../explanation/handing-over.md)
- [Keys](../reference/keys.md)
