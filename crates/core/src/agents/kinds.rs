//! Everything agent-specific lives in this one table: what each program accepts, how it is started, where it saves
//! its conversation and how that is read. Every entry is optional and has a stated fallback, so a new program is one
//! more entry here (and its readers in read_turn.rs, read_tokens.rs and talk.rs). Verified on claude 2.1.296, codex
//! 0.160.0, pi 1.1.0 (docs/checked-on-herdr.md).
//!
//!   access_full / access_ask  the arguments for each `<role>.access` value     no `ask`: the value is refused for it
//!   efforts      the effort levels it accepts                                   none: any word is passed on
//!   fast         it has a faster service tier (`<role>.fast`)                   none: the setting does nothing
//!   tokens       its saved conversation's tokens can be counted                 none: `limit.tokens` does nothing
//!   interrupt    the key that stops a turn                                      esc
//!   min_version  the oldest version its turn-end hook was verified on           older: runs without the hook
//!   rules_file   the file in its start folder it reads at every conversation    none
//!   since        an argument and the first version that takes it               older: started without it
pub struct Kind {
    pub name: &'static str,
    pub min_version: &'static str,
    pub efforts: &'static [&'static str],
    pub access_full: &'static [&'static str],
    pub access_ask: Option<&'static [&'static str]>,
    pub fast: bool,
    pub tokens: bool,
    pub interrupt: &'static str,
    pub rules_file: Option<&'static str>,
    /// The board's own arguments that only a newer version takes: (argument, first version that takes it). An older
    /// one quits at once on an argument it does not know, and herdr then waits in vain for it to come up.
    pub since: &'static [(&'static str, &'static str)],
    /// It can read its own conversation back (a reader in read_turn.rs).
    pub reads_turn: bool,
    /// It has a plan mode of its own, where nothing is changed (plan/).
    pub plan_mode: bool,
    /// It can be the Watcher (a headless one-shot command).
    pub one_shot: bool,
}
pub static KINDS: &[Kind] = &[
    Kind { name: "claude", min_version: "2.1.296", efforts: &["low", "medium", "high", "xhigh", "max"], access_full: &["--dangerously-skip-permissions"], access_ask: Some(&[]),
        fast: false, tokens: true, interrupt: "esc", rules_file: Some("CLAUDE.md"), since: &[], reads_turn: true, plan_mode: true, one_shot: true },
    // --no-daemon: a shared Codex background server of another version shows a restart dialog that herdr reads as
    // idle, so a prompt would be typed into it. Each agent runs on its own instead. Codex before 0.156.0 has no shared
    // server and no such flag (HP-11).
    Kind { name: "codex", min_version: "0.160.0", efforts: &["minimal", "low", "medium", "high", "xhigh"], access_full: &["--dangerously-bypass-approvals-and-sandbox", "--no-daemon"], access_ask: Some(&["--no-daemon"]),
        fast: true, tokens: true, interrupt: "esc", rules_file: Some("AGENTS.md"), since: &[("--no-daemon", "0.156.0")], reads_turn: true, plan_mode: false, one_shot: true },
    // pi never asks before running something: it has no `ask`.
    Kind { name: "pi", min_version: "1.1.0", efforts: &["off", "minimal", "low", "medium", "high", "xhigh"], access_full: &[], access_ask: None,
        fast: false, tokens: true, interrupt: "esc", rules_file: Some("AGENTS.md"), since: &[], reads_turn: true, plan_mode: false, one_shot: true },
];
pub const KIND_NAMES: &[&str] = &["claude", "codex", "pi"];
pub fn kind(name: &str) -> Option<&'static Kind> { KINDS.iter().find(|k| k.name == name) }
