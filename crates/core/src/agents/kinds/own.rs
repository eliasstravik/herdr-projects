//! The three programs whose saved conversation the board reads itself: Claude Code, Codex and pi. Verified on claude
//! 2.1.296, codex 0.160.0, pi 1.1.0 (docs/checked-on-herdr.md). What they do beyond these facts is code of their own
//! in adapter.rs (the turn-end hook, the conversation files, the Watcher's lean start, Codex's rules and fast mode,
//! pi's provider in its model).
use super::{Kind, Models, Read, BASE, NO_MODELS};

pub static KINDS: &[Kind] = &[
    Kind { name: "claude", min_version: "2.1.296", efforts: &["low", "medium", "high", "xhigh", "max"], access_full: &["--dangerously-skip-permissions"],
        tokens: true, rules_file: Some("CLAUDE.md"), reads_turn: true, plan_mode: true, model: &["--model", "{}"],
        // Claude Code has no command that lists its models: its short names (each the newest of its kind) and the
        // full names Claude Code 2.1.301 knows. `[1m]` after one asks for the long context. On a cloud or a gateway
        // of its own (models/listed.rs `own_provider`) its names are that provider's, and not checked.
        models: Models { aliases: &["default", "fable", "opus", "sonnet", "haiku", "opusplan"], known: CLAUDE_MODELS, suffix: &["[1m]"], example: "claude/claude-opus-5-5", ..NO_MODELS },
        effort: &["--effort", "{}"], resume: &["--resume", "{}"], session: &["--session-id", "{}"], standing: &["--append-system-prompt-file", "{}"], integration: "claude",
        checked: "everything the board does, on 2.1.296 to 2.1.300", ..BASE },
    // --no-daemon: a shared Codex background server of another version shows a restart dialog that herdr reads as
    // idle, so a prompt would be typed into it. Each agent runs on its own instead. Codex before 0.156.0 has no shared
    // server and no such flag (HP-11).
    Kind { name: "codex", min_version: "0.160.0", efforts: &["minimal", "low", "medium", "high", "xhigh"], access_full: &["--dangerously-bypass-approvals-and-sandbox", "--no-daemon"], access_ask: Some(&["--no-daemon"]),
        fast: true, tokens: true, rules_file: Some("AGENTS.md"), since: &[("--no-daemon", "0.156.0")], reads_turn: true, model: &["-m", "{}"],
        models: Models { list: &["debug", "models"], read: Read::Catalog, example: "codex/gpt-6.1-sol", ..NO_MODELS },
        effort: &["-c", "model_reasoning_effort=\"{}\""], resume: &["resume", "{}"], integration: "codex",
        checked: "everything the board does, on 0.153.4 to 0.162.1", ..BASE },
    // pi never asks before running something: it has no `ask`.
    Kind { name: "pi", min_version: "1.1.0", efforts: &["off", "minimal", "low", "medium", "high", "xhigh"], access_ask: None,
        tokens: true, rules_file: Some("AGENTS.md"), reads_turn: true, model: &["--model", "{}"], effort: &["--thinking", "{}"],
        // pi/<provider>/<model>; `:high` after it is a thinking level of its own.
        models: Models { list: &["--list-models"], read: Read::Table, suffix: &[":"], example: "pi/anthropic/claude-opus-5-5", ..NO_MODELS },
        resume: &["--session-id", "{}"], session: &["--session-id", "{}"], standing: &["--append-system-prompt", "{}"], integration: "pi",
        checked: "everything the board does, on 1.1.0", ..BASE },
];
/// Claude Code's full model names, newest first; a new one is one more entry.
const CLAUDE_MODELS: &[&str] = &[
    "claude-fable-5-1", "claude-fable-5", "claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5", "claude-opus-5", "claude-sonnet-5",
    "claude-opus-4-8", "claude-opus-4-7", "claude-opus-4-6", "claude-sonnet-4-6", "claude-opus-4-5", "claude-opus-4-5-20251101",
    "claude-sonnet-4-5", "claude-sonnet-4-5-20250929", "claude-haiku-4-5", "claude-haiku-4-5-20251001", "claude-opus-4-1", "claude-opus-4-1-20250805",
    "claude-opus-4-0", "claude-opus-4-20250514", "claude-sonnet-4-0", "claude-sonnet-4-20250514",
];
