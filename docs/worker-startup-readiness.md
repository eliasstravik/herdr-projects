# Worker initial-brief readiness

A detected `idle`/`done` state is not enough to send a worker's first brief.
For example, Codex daemon-install output can read as idle before the folder or
hooks trust chooser appears. Sending then can leave buffered input that answers
that chooser (#63).

Every initial-brief path now uses the same gate: normal ticker, fast brief poll,
manual `thread brief`, and adoption. The gate requires:

- A matching agent with a nonempty terminal id and an idle/done state.
- A recognized, visible empty input box using the existing styled-screen parser.
  Loading screens, drafts, unknown layouts and recognized trust/hook dialogs hold
  the brief. A stale composer above a trust dialog is also refused.
- The same terminal, native session, kind, state and state-change sequence across
  a three-second settle period. Observations older than 90 seconds are discarded.
- A fresh agent/screen check immediately before claiming the pending brief under
  the project lock. Concurrent senders cannot both claim it.

Missing panes, non-ready states, read failures, identity changes and launch/restart
attempts clear readiness. `thread brief` does not bypass settling. The fast poll
continues while a brief remains pending, rather than stopping after the first
idle observation. The normal ticker no longer uses its own unguarded sender.

Only the user grants folder or hooks trust in the native pane. `thread keys`
refuses recognized trust dialogs before sending either text or keys, and the
coordinator skill no longer instructs an agent to accept them. Sandbox and
permission flags are unchanged. Unknown layouts remain pending for inspection;
there is no timeout that automatically approves a dialog or sends anyway.

## Verification

Run:

```sh
cargo test --all-targets --locked
cargo build --release --locked
HERDR_PROJECTS_TEST_BINARY="$PWD/target/release/herdr-projects" \
  cargo test --locked --test cli worker_startup -- --nocapture
```

Deterministic scenarios cover loading → trust/hooks chooser → stable empty
composer, a late dialog on the final recheck, drafts, blocked/unknown/working
states, a closed pane, terminal replacement, state sequence changes, expired
observations, a new launch attempt and screen-read failure. Existing normal/fast
poll and manual-brief scenarios now assert settling before delivery.

The compiled-binary regression runs against a private fake Herdr executable in a
scrubbed environment. It asserts zero prompts or approval keys during loading and
trust, then races two real `thread brief` processes after readiness and verifies
exactly one submission. It creates only temporary fixture project records. No
native Claude/Codex process, live Herdr server or paid inference is used.

These are synthetic regression tests, not a production native-agent smoke test.
Herdr's CLI has separate read and prompt operations, so this is not an atomic
screen-and-submit transaction; new native UI layouts should be verified when
upgrading. Unsupported layouts fail closed. Existing transport-error behavior is
unchanged: this patch addresses startup readiness, not exactly-once task execution
across an ambiguous transport failure.

## Adapter rollout

Before enabling a launch adapter's `worker_startup_verified`, deploy this patched
binary for **all** project command prefixes and the ticker for that projects
root. Pointing only the adapter at it while an older ticker still runs is not
sufficient. An already-running old ticker does not acquire this fix when a binary
file is replaced. The rollout owner must manage that handoff; this change does
not restart shared services or rewrite live adapter configuration.

The adapter's Herdr client/server >=0.9.1 requirement still applies. Folder/hook
trust remains a user decision. Runtime/quota selection still occurs only when a
new native process is needed; this patch adds no mid-run model switching.
