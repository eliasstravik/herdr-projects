# A turn cut off by the provider

**When the model's provider fails mid-turn (at capacity, overloaded, a rate limit, a dropped connection), the board waits
and tries again by itself**: after 1, 2, 4 and 8 minutes, so five tries over fifteen minutes. Only then does it give up
and tell you. Errors that waiting cannot fix go to you at once.

Such a turn is neither finished nor failed: the agent did not stop, its provider did.

## How the board knows

The agent's own saved conversation records the error for the turn, and the board reads it there:

| Program | Where the error is |
|---|---|
| Codex | the `error` on `task_complete` |
| Claude Code | an entry with `isApiErrorMessage` |
| pi | a message with `stopReason` `error` |

The program's own code for the error decides whether it passes (Codex's `codex_error_info`, Claude Code's `error` and HTTP
status). Where there is none, its words do.

## The reviewer

The board closes the review, waits, and starts it again.

- Meanwhile the row says why and when: `Reviewer's model is at capacity, trying again in 2m`.
- **A review that was cut off is not a review.** It uses up none of the task's `rounds`, has no number (the next one that
  runs is still the "1st"), and is not in the list of reviews.
- **If the last try is cut off too**, the task fails with the real reason and the provider's own words:
  `Review failed: the reviewer's model is at capacity, still after 5 tries over 15m. Its error: Selected model is at capacity…`. `Ctrl+L` reviews again.

## The working agent

The same waits. Then the board tells the agent to carry on where it left off, as a turn of the plugin's own: counters and
an approved merge survive it.

- The row stays under `working` and says `Agent's connection dropped, trying again in 2m`.
- **You can skip the wait.** A reply, `Ctrl+R` or `Ctrl+L` does it right away. Stopping the task ends the wait.
- After the last try, the task is "Agent stopped", with the real reason.
- A reply starts the count again.
- An agent whose program exited with the error is started again in the same conversation, and told the same.

## Errors that waiting cannot fix

Never retried, and sent to you at once: a login, a model that does not exist or is not offered, a usage limit, a request
the provider refuses, and any error the board does not recognise. A reviewer that hits one fails the review with "the
reviewer stopped:" and the error; that, too, is not on the list of reviews.

## With a fallback agent

With `worker.fallback` or `reviewer.fallback` set, another agent takes over where the board would otherwise give up or go
to you: after the last try, for an error that passes; at once, for one that waiting cannot fix. So the order is: **the
tries first, then the fallback, then you** ([the fallback agent](fallback.md)).

## Details

The waits are fixed. What was seen in real conversation files is in [checked on Herdr](checked-on-herdr.md).

- `provider_error` in `crates/core/src/agents/provider_error.rs`: which errors pass;
- `crates/core/src/store/retries.rs`: the wait;
- `crates/flow/src/finish/turn_ended.rs` and `crates/flow/src/finish/review.rs`: the retries;
- tests: `tests/retry.rs`, `tests/retry_agent.rs` and `tests/endings.rs`, on recorded endings in `tests/fixtures/endings`.
