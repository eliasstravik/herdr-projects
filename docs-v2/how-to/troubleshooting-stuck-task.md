# Fix a task that seems stuck

This page tells what to do when a task does not move. Find the words on the row of the task, then do the steps under
them.

## `Waiting for a free slot` (`Q#2`)

All the slots of `board.agents` are in use.

1. Wait for a free slot.
2. Or set a higher value for `board.agents`.

## `queued after HSW-12`

The task waits until other tasks are done ([one task after another](../explanation/after.md)).

1. Wait for the other tasks.
2. Or start the task now: `herdr-projects delegate <id> --now`.

## `Has not started yet: pressing enter again`

The text of the task is in the box of the agent.

1. Wait a moment. The board presses enter again.

## `Not started: its instructions sit unsent in its box`

The text did not go in ([handing over](../explanation/handing-over.md)).

1. Press `Ctrl+L` (retry).
2. Or go into the agent and press enter.

## `Finishing` for a long time, with the agent at rest

The board did not see the end of the turn of the agent.

1. Press `Ctrl+L`.
2. Or run `herdr-projects finish <id>`.

## `Stopped without a summary`

The agent stopped, and the board cannot tell if it finished.

1. To tell the agent to continue, press `Ctrl+R`.
2. To take the work as it is, press `Ctrl+L`.

## `Agent's connection dropped, trying again in 2m`

The provider cut off the turn. The board tries again by itself
([a turn cut off by the provider](../explanation/provider-errors.md)).

1. Do nothing.

## `Agent stopped`

An error that waiting cannot fix stopped the agent, for example a usage limit or a refusal.

1. Read the reason in the task panel.
2. Reply to the agent, or press `Ctrl+R`.

## `Agent stopped: … can't be used with your ChatGPT sign-in`

The program of the agent refused its model.

1. Choose a model ([troubleshoot the board](troubleshoot.md#cant-be-used-with-your-chatgpt-sign-in-while-the-agent-works)).
2. Press `Ctrl+L`.

## `Needs a login`

The program of the agent is logged out.

1. Select the task and press `Enter` to go into the agent.
2. Log in.
3. Reply to the agent, or press `Ctrl+R`.

## `stopped at its 2h limit`

The task reached `limit.time` or `limit.tokens` ([limits](../explanation/limits.md)).

1. To continue, press `Ctrl+R`.
2. Or set a higher limit.
3. Or, to take the work as it is, press `Ctrl+L`.

## `resolving a conflict with main`

Main changed under the task. Its agent fixes the conflict.

1. Wait.

## `behind main`

The board could not bring main in. The work itself is correct. The board tries again each minute. The task panel gives
the reason.

1. Read [fix a task that is behind main](troubleshooting-behind-main.md).

## `Waiting for GitHub checks · 2 of 3 passed`

The checks of the pull request still run.

1. Wait. The task merges by itself when the checks pass.

## `Could not read GitHub's checks`

`gh` is not logged in, or its token cannot read Actions and Commit statuses.

1. Log in to `gh`.
2. Or give the token the read permissions for Actions and Commit statuses
   ([a fine-grained GitHub token is enough](../explanation/how-work-gets-done-github-checks.md#a-fine-grained-github-token-is-enough)).

## `The check stands still · nothing finished for 13m`

The check of the repo hangs.

1. Look at what the check runs ([the progress of the check](../explanation/check-progress.md)).

## The agent shows a dialog

A dialog is, for example, a permission prompt or a question with choices. The chat tells you about it.

1. Press `Enter` to go into the agent.
2. Answer the dialog there.

## The agent asked a question in plain words and stopped

The board takes that as finished work and reviews it ([an agent that waits for you](../explanation/waiting-for-you.md)).

1. Reply to the agent. The agent then continues.

## Related pages

- [Troubleshoot the board](troubleshoot.md)
- [Fix a task that is behind main](troubleshooting-behind-main.md)
- [A turn cut off by the provider](../explanation/provider-errors.md)
- [Limits](../reference/limits.md)
- [Keys](../reference/keys.md)
