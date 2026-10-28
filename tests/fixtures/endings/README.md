The last lines of saved agent conversations, as test input (`test/retry.test.js`; the fake herdr replays them).

Recorded from real files on 2026-10-12 (Codex 0.160.0, Claude Code 2.1.296, pi 1.1.0), with long text cut, and
signatures, request ids and folder names taken out:

- `codex-at-capacity`: the review of herdr-projects/37 that this was built for. One opening message, one command,
  then `task_complete` with no last message and the error "Selected model is at capacity" (`server_overloaded`).
- `codex-unauthorized`, `codex-model-not-supported`: `task_complete` with a 401 and with a 400 (`other`).
- `pi-socket-closed`, `pi-overloaded`, `pi-gateway-502`, `pi-stream-ended`, `pi-unknown-provider`, `pi-flagged`:
  a message with `stopReason: "error"` and its `errorMessage`.
- `claude-model-not-found`: an `isApiErrorMessage` entry (`model_not_found`, 404).

Not recorded, because none could be found or provoked: the files named `-made` are written by hand in the shape of
a recorded one. `claude-overloaded-made` and `claude-rate-limit-made` have the recorded Claude entry's shape with
the API's documented 529 and 429 texts; the two `-finished-made` files are an ordinary finished turn.
- `claude-review-60k-made`, `claude-review-60k-again-made`, `claude-review-1510-made` (tests/limits_review.rs): a reviewer's
  finished turn that used that many tokens.
- `codex-helper-running-made`, `claude-helper-running-made`, `claude-helper-back-made` (tests/helpers.rs): a turn that
  ends while a helper it started is still at work, in the shape of the recorded ones (Codex 0.162.1's
  `SubAgentActivity`, Claude Code 2.1's "Async agent launched" result and `<task-notification>`), and the turn its
  helper's answer starts.
- `claude-helper-back-mid-turn-made` (tests/finish_missed.rs): a helper whose answer comes back while the agent's turn
  is still going, in the shape recorded on HSW-218 (Claude Code 2.1.300): its `<task-notification>` is only a
  `queue-operation` and an `attachment` (a queued command taken into the running turn), never a message of its own.
- `claude-idle-compaction` (tests/turns/ended_late.rs): recorded on 2026-10-24 (Claude Code 2.1.312), summary and file
  names cut: what Claude Code writes when it compacts a session left idle an hour after its turn ended, with no prompt.
