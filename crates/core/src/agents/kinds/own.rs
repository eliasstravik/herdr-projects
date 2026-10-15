//! The three programs whose saved conversation the board reads itself: Claude Code, Codex and pi. Verified on claude
//! 2.1.296, codex 0.160.0, pi 1.1.0 (docs/checked-on-herdr.md). What they do beyond these facts is code of their own
//! in adapter.rs (the turn-end hook, the conversation files, the Watcher's lean start, Codex's rules and fast mode,
//! pi's provider in its model).
use super::{Kind, Models, Read, BASE, NO_MODELS};

pub static KINDS: &[Kind] = &[
    Kind { name: "claude", min_version: "2.1.296", efforts: &["low", "medium", "high", "xhigh", "max"], access_full: &["--dangerously-skip-permissions"],
        tokens: true, rules_file: Some("CLAUDE.md"), reads_turn: true, plan_mode: true, model: &["--model", "{}"],
        // Claude Code lists no models: it takes its short names and every full name (claude-opus-5-5, and a cloud's
        // us.anthropic.claude-opus-5-5); `[1m]` after one asks for the long context.
        models: Models { aliases: &["default", "fable", "opus", "sonnet", "haiku", "opusplan"], prefix: "claude-", suffix: &["["], example: "claude/claude-opus-5-5", ..NO_MODELS },
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
