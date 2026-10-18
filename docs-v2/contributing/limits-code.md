# Limits code

This page says where the code and the tests of the time and token limits are. Use it when you change how limits work.

## The code

| Path | What it holds |
|---|---|
| [`crates/flow/src/limits/`](../../crates/flow/src/limits/) | The limits: `run.rs` (the light part: a run and what counts), `spent.rs` (counting), `reached.rs` (the stop), `tick.rs` (the pass that looks), `shown.rs` (what is shown) |
| [`crates/flow/src/actions/limits.rs`](../../crates/flow/src/actions/limits.rs) | Continue, and as it is |
| [`crates/core/src/agents/read_tokens.rs`](../../crates/core/src/agents/read_tokens.rs) | One counter for each program |
| [`crates/flow/src/finish/turn_ended.rs`](../../crates/flow/src/finish/turn_ended.rs), [`crates/flow/src/finish/pipeline.rs`](../../crates/flow/src/finish/pipeline.rs) | The check of the token limit at the end of a turn |
| [`crates/flow/src/finish/auto_merge.rs`](../../crates/flow/src/finish/auto_merge.rs) | The check of the token limit before anything merges by itself |

## The tests

- [`tests/limits.rs`](../../tests/limits.rs)
- [`tests/limits_more.rs`](../../tests/limits_more.rs)
- [`tests/limits_tokens.rs`](../../tests/limits_tokens.rs)

## Related pages

- [How limits work](../explanation/limits.md)
- [Limits](../reference/limits.md)
- [Crates](crates.md)
- [Testing](testing.md)
