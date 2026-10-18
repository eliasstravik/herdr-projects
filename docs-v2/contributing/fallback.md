# The code of the retries and the fallback

This page tells where the code is that tries a turn again after a provider error, and the code that hands a task over
to a fallback agent.

## The retries after a provider error

| File | What it holds |
|---|---|
| `crates/core/src/agents/provider_error.rs` | `provider_error`: which errors stop by themselves. |
| `crates/core/src/store/retries.rs` | The wait. |
| `crates/flow/src/finish/turn_ended.rs` | The retries for the working agent. |
| `crates/flow/src/finish/review.rs` | The retries for the reviewer. |

The tests are `tests/retry.rs`, `tests/retry_agent.rs` and `tests/endings.rs`. They run on recorded endings in
`tests/fixtures/endings`. What the project saw in real conversation files is in
[facts checked on Herdr 0.9.3](herdr-0.9.3-facts.md).

## The hand-over

The hand-over is one function, `hand_over` in `crates/flow/src/finish/hand_over.rs`. Each step can call it with its
reason.

| File | What it holds |
|---|---|
| `crates/core/src/store/handovers.rs` | What the board keeps on the task, the next agent on the list, and the words. |
| `crates/flow/src/finish/hand_over.rs` | The hand-over of the working agent. |
| `crates/flow/src/finish/review_fallback.rs` | The hand-over of the reviewer. |

The tests are `tests/fallback.rs`, `tests/fallback_more.rs` and `tests/fallback_settings.rs`.

## Related pages

- [The fallback agent](../explanation/fallback.md)
- [A turn cut off by the provider](../explanation/provider-errors.md)
- [The code of the agent programs](agents-code.md)
- [The crates](crates.md)
- [Testing](testing.md)
