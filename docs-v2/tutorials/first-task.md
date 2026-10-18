# Your first task

In this lesson, you take one task from start to end. You add the task, put an agent on it, follow it, answer the
agent, read the review, and approve and merge the work.

## Before you start

- Do the lesson [Your first board](first-board.md) first. At its end, the board is open.
- Open the board in a git repo. Then the agent works in a worktree of its own, not in your checkout.
- For a pull request, the repo must have an `origin` remote, and `gh` must be installed.

## Step 1: Add the task

1. In the text box at the bottom of the board, type what must be done. For example, type `Fix the login`.
2. Press `Enter`.

   The Coordinator makes it a task. The task shows as a row in the list.

**Note:** If you press `Tab` in place of `Enter`, the board also puts an agent on the task at once.

## Step 2: Put an agent on the task

1. Press `↑` or `↓` until the task is selected.

   The hint line at the bottom says `tab put an agent on this task`.

2. Press `Tab`.

The board now does these things for you:

- It asks Herdr for a new worktree space, on a branch of its own, for example `work/7-fix-the-login`.
- It starts the agent in that space, with the task and its rules.

**CAUTION:** By default, the agent has full access. It can run any command as you, and it does not ask first.

## Step 3: Follow the agent

The row of the task tells what the agent does. The hint line now says
`enter open this task's agent · space talk to the agent · ctrl+x stop the agent`.

1. Press `Enter`.

   You see the real pane of the agent in Herdr. The board continues to run in its own tab.

2. Press the board key (`prefix+a`).

   You are back on the board, as you left it.

3. Press `Alt+P`.

   The side panel shows the selected task beside the list.

## Step 4: Answer the agent

An agent can stop to ask you a question. If it does, the row of the task shows the mark `?`. The hint line says
`space answer this task's agent`. If your agent does not ask, go to step 5.

1. Press `Space`.

   The text box now sends your words to the agent of the task.

2. Type your answer.
3. Press `Enter`.

   The agent gets your answer as its next message, and continues to work.

**Note:** An agent that waits for you is not finished. While it waits, the board does not check, review or merge
the work.

## Step 5: Read the review

When the agent says that it is done, the board does the finish line by itself:

- it commits what is left, and brings the branch up to date with main;
- it runs the `check` of the repo;
- it pushes the branch and opens a draft pull request;
- it has a second agent, the reviewer, review the work.

The reviewer lists its findings. The board then decides if the work passes the gate.

1. Look at the row of the task.

   The row shows the result and a tally, for example `Pass · 2 minor`. The tag `R#2` tells which review it was. Its
   colour tells how it went.

2. In the side panel, read the summary of the review.

If the review failed, the board sends the findings back to the agent by itself. The agent fixes them, and the board
reviews the work again. You do not have to do anything.

## Step 6: Approve and merge

When the work passed, the task is under `ready to approve`. The hint line says
`ctrl+l approve and merge this task`.

1. Press `Ctrl+L`.

   The board asks a question, for example:

   ```
    Merge HSW-18 "Add an undo for clear" (PR#18) into main?
    A merge cannot be undone.
    y = merge · n = do not merge
   ```

2. Read the question.
3. Press `y`.

The board brings the branch up to date with main one last time. Then it merges the pull request, at the commit that
the reviewer saw.

## Step 7: See the clean-up

The task is now done. Its space leaves the sidebar of Herdr by itself. The board removes its worktree and its branch.
The board never removes work that was not merged or not saved.

## What you learned

- You added a task with `Enter`, and put an agent on it with `Tab`.
- You opened the pane of the agent with `Enter`, and came back with the board key.
- You answered the agent with `Space`.
- You read the result of the review on the row and in the side panel.
- You approved and merged the work with `Ctrl+L` and `y`.

## Next steps

- To let work merge without your yes, see [merge work by itself](../how-to/turn-on-auto-merge.md).
- To get a report and no change to the repo, see [make a research task](../how-to/make-a-research-task.md).
- To learn what happens at each step, read [how work gets done](../explanation/how-work-gets-done.md).

## Related pages

- [Your first board](first-board.md)
- [Delegate a task and merge its work](../how-to/delegate-and-merge.md)
- [How work gets done](../explanation/how-work-gets-done.md)
- [The gate](../explanation/gate.md)
- [An agent that waits for you is not finished](../explanation/waiting-for-you.md)
- [Keys](../reference/keys.md)
