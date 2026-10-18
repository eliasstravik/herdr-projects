# A turn cut off by the provider

This page tells what the board does when the provider of the model fails during a turn, and why.

## What the board does

The provider of the model can fail during a turn: the model is at capacity, the servers are overloaded, a rate limit
applies, or the connection dropped. Then the board waits and tries again by itself, after 1, 2, 4 and 8 minutes. That
is five tries in fifteen minutes. Only after the last try does the board stop and tell you. The board sends errors that
waiting cannot fix to you at once.

Such a turn is not finished and is not failed. The agent did not stop: its provider did.

The waits are fixed. You cannot change them.

## How the board knows

The saved conversation of the agent records the error for the turn. The board reads it there:

| Program | Where the error is |
|---|---|
| Codex | the `error` on `task_complete` |
| Claude Code | an entry with `isApiErrorMessage` |
| pi | a message with `stopReason` `error` |

The code that the program gives for the error tells whether the error stops by itself (the `codex_error_info` of Codex,
the `error` and the HTTP status of Claude Code). Where there is no code, the words of the error tell it.

The board reads no conversation of the other programs. Thus, for them, it cannot tell a passing error from a lasting one
([agent programs](../reference/agent-programs.md#what-the-other-programs-do-not-get)).

## The reviewer

The board closes the review, waits, and starts the review again.

- While the board waits, the row tells why and when: `Reviewer's model is at capacity, trying again in 2m`.
- **A review that the provider cut off is not a review.** It uses none of the `rounds` of the task. It has no number:
  the next review that runs is still the "1st". It is not in the list of reviews.
- **If the provider also cuts off the last try**, the task fails with the real reason and the words of the provider:
  `Review failed: the reviewer's model is at capacity, still after 5 tries over 15m. Its error: Selected model is at capacity…`.
  `Ctrl+L` reviews again.

## The working agent

The board uses the same waits. Then it tells the agent to continue where it stopped. This is a turn of the plugin
itself, so the counters and an approved merge stay as they were.

- The row stays under `working` and says `Agent's connection dropped, trying again in 2m`.
- **You can skip the wait.** A reply, `Ctrl+R` or `Ctrl+L` skips it at once. If you stop the task, the wait ends.
- After the last try, the task is "Agent stopped", with the real reason.
- A reply starts the count again.
- If the program of the agent exited with the error, the board starts it again in the same conversation, and tells it
  the same.

## Errors that waiting cannot fix

The board never tries these errors again, and sends them to you at once:

- a login;
- a model that does not exist or that the provider does not offer;
- a usage limit;
- a request that the provider refuses;
- each error that the board does not know.

A reviewer that gets one of these errors fails the review with "the reviewer stopped:" and the error. That review is
also not on the list of reviews.

## With a fallback agent

If `worker.fallback` or `reviewer.fallback` is set, another agent takes over where the board otherwise stops or sends
the task to you:

- for an error that stops by itself, after the last try;
- for an error that waiting cannot fix, at once.

Thus the order is: **the tries first, then the fallback agent, then you** ([the fallback agent](fallback.md)).

## Related pages

- [The fallback agent](fallback.md)
- [How work gets done](how-work-gets-done.md)
- [Agent programs](../reference/agent-programs.md)
- [Troubleshoot the board](../how-to/troubleshoot.md)
- [The code of the retries and the fallback](../contributing/fallback.md)
- [Facts checked on Herdr 0.9.3](../contributing/herdr-0.9.3-facts.md): what the project saw in real conversation files
