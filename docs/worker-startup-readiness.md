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

By default, the user grants folder or hooks trust in the native pane. The opt-in
folder/MCP startup policy below is the only automated exception. `thread keys`
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
trust requires the user or the explicit folder/MCP policy below. Runtime/quota selection still occurs only when a
new native process is needed; this patch adds no mid-run model switching.

## Opt-in folder trust and MCP enablement

The default remains manual. A user can persist authorization in
`~/.config/herdr-projects/config.toml` (not PROJECT.md or agent prompts):

```toml
[startup]
folder_trust = true
mcp_enablement = true
```

The two independent options apply to managed project coordinators and pending
workers, including the ticker's normal, fast-brief, and remote-worker paths.
The adapter can poll the same handler with:

```sh
herdr-projects --root /path/to/projects native-startup demo --pane w1:p1
```

The JSON `state` is `disabled`, `waiting`, `accepted`, `held`, `manual`, or
`complete`. `accepted` means one navigation/confirmation key was sent; it does
not mean startup completed. The handler checks project membership, waits three
seconds for a stable chooser, then rechecks the terminal and exact screen before
sending one key. Claims are persisted under the project lock in
`.state/startup.json`, keyed by socket/machine, pane, and terminal identity.
Screens are hashed, not logged or saved. Repeated/ambiguous sends are held for
manual inspection rather than replayed. Do not delete claims to blindly retry.
Automation ends after a settled empty composer or observed working state.

Supported menus are Codex folder access (verified against native 0.157.1 source)
and Claude folder trust, single-server MCP enablement, and fully visible MCP
checkbox lists (verified against installed Claude Code 2.1.283 UI definitions).
The single-server menu selects “Use this and all future MCP servers in this
project”; checkbox lists explicitly check every visible server before selecting
“Enable selected”. Truncated lists and unknown layouts require manual handling.
Native pretrust settings continue to work without any chooser actions.

Hooks-only review, managed-settings approval, OAuth login/reauthentication,
individual tool execution and destructive-action approvals are excluded.
`thread keys` still refuses trust dialogs; only the scoped native handler applies
this policy. Permission and sandbox flags are unchanged. Herdr does not expose
an atomic compare-screen-and-send operation, so the final fresh read narrows
but cannot eliminate the native UI race.

For Buzz, set `"startup_policy": true` on each opted-in binding in its projects
launcher JSON. Both that binding option and the Herdr Projects user policy are
required. Install the patched CLI at the adapter's `projects_bin` and every
existing command prefix, and replace the old ticker. No live rollout is performed
by building or testing this branch. Synthetic tests cover exact chooser selection,
excluded prompts, project isolation, settling, ticker integration, failed reads,
failed/ambiguous sends, restart deduplication and concurrent binary calls.
