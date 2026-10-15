//! Everything agent-specific lives in this one table: what each program accepts, how it is started, where it saves
//! its conversation and how that is read. Every entry is optional and has a stated fallback, so a new program is one
//! more entry in kinds/ (and, for one whose conversation the board reads, its readers in read_turn.rs, read_tokens.rs
//! and talk.rs). The programs are herdr's own (`herdr agent start --kind`); what was tried on which version is each
//! entry's `checked` (docs/agents.md, docs/checked-on-herdr.md).
//!
//!   program      the executable                                                 the name
//!   aliases      other names a setting may give it (herdr's integration name)   none
//!   access_full / access_ask  the arguments for each `<role>.access` value     no `ask`: the value is refused for it
//!   efforts      the effort levels it accepts                                   none: any word is passed on
//!   effort       the arguments that give it an effort, `{}` the level           none: an effort is refused
//!   model        the arguments that give it a model, `{}` the model             none: a model is refused
//!   models       the model names it takes: its own command that lists them, or  none: a name is not checked
//!                its short and full names
//!   resume       the arguments that reopen a conversation, `{}` its id          none: it starts afresh
//!   session      the arguments that name a new conversation, `{}` a new uuid    none: herdr's integration tells it
//!   fresh        the arguments that start a new conversation, not its last one  none
//!   standing     the arguments that add a rules file to its instructions        none: the rules go in its first prompt
//!   lead         arguments that come first, before everything else              none
//!   headless     its one-shot command, for the Watcher                          none: it cannot be the Watcher
//!   fast         it has a faster service tier (`<role>.fast`)                   none: the setting does nothing
//!   tokens       its saved conversation's tokens can be counted                 none: `limit.tokens` does nothing
//!   interrupt    the key that stops a turn                                      esc
//!   min_version  the oldest version its turn-end hook was verified on           older: runs without the hook
//!   rules_file   the file in its start folder it reads at every conversation    none: it is told to read the rules
//!   since        an argument and the first version that takes it               older: started without it
//!   integration  herdr's own integration for it, and whether herdr sees its state only through it
//!   cannot       roles it cannot do, and why                                    none
pub struct Kind {
    pub name: &'static str,
    pub program: &'static str,
    pub aliases: &'static [&'static str],
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
    pub lead: &'static [&'static str],
    pub model: &'static [&'static str],
    /// The model names it takes (models.rs).
    pub models: Models,
    pub effort: &'static [&'static str],
    /// Why an effort is refused, where the program takes it some other way.
    pub effort_note: &'static str,
    pub resume: &'static [&'static str],
    pub session: &'static [&'static str],
    pub fresh: &'static [&'static str],
    pub standing: &'static [&'static str],
    pub headless: Option<Headless>,
    /// herdr's integration for it (`herdr integration install <name>`): "" for none.
    pub integration: &'static str,
    /// herdr sees whether it works only through that integration: without it, it is never started.
    pub needs_integration: bool,
    /// (role, why): a role it cannot do well. The role is a setting's first part (worker, reviewer, coordinator,
    /// plan, watcher); `worker` also stands for the fallbacks and the attempts.
    pub cannot: &'static [(&'static str, &'static str)],
    /// What was tried, on which version (docs/agents.md).
    pub checked: &'static str,
}
/// A headless one-shot: `args`, then the model, then the instructions (`system`, else written above the prompt),
/// then `prompt` (`{}` the prompt; none: it goes in on stdin).
pub struct Headless {
    pub args: &'static [&'static str],
    pub model: &'static [&'static str],
    pub system: &'static [&'static str],
    pub prompt: &'static [&'static str],
}
/// The model names a program takes: those its own command lists, else its short names and the full names written
/// here. With neither, a name is not checked: it goes to the program as written.
pub struct Models {
    /// The program's own command that prints the names it takes (its arguments), and how that is read.
    pub list: &'static [&'static str],
    pub read: Read,
    /// Short names it always takes (claude's `opus`).
    pub aliases: &'static [&'static str],
    /// Its full names, where it has no command that lists them (Claude Code): a newer one is one more line here.
    pub known: &'static [&'static str],
    /// What may follow a name and is not part of it: from this mark on, it is cut off before the name is looked up
    /// (`[`: cursor's parameters; `:`: pi's thinking level). A longer one is a whole ending (claude's `[1m]`).
    pub suffix: &'static [&'static str],
    /// How a model is written after the program, as the board's pages and refusals say it.
    pub example: &'static str,
}
/// How a program's list of models is read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Read {
    /// No list.
    None,
    /// Codex's catalog: JSON, `{"models": [{"slug", "visibility"}]}`; a hidden one is taken, not shown.
    Catalog,
    /// pi's table: a header, then a provider and a model on each line, taken as provider/model.
    Table,
    /// `name - What it is`, one a line (cursor).
    Dashed,
    /// One name a line (opencode's provider/model).
    Lines,
}
pub const NO_MODELS: Models = Models { list: &[], read: Read::None, aliases: &[], known: &[], suffix: &[], example: "" };
/// What an entry has that it does not name.
pub const BASE: Kind = Kind {
    name: "", program: "", aliases: &[], min_version: "0", efforts: &[], access_full: &[], access_ask: Some(&[]), fast: false, tokens: false,
    interrupt: "esc", rules_file: None, since: &[], reads_turn: false, plan_mode: false, lead: &[], model: &[], models: NO_MODELS, effort: &[], effort_note: "",
    resume: &[], session: &[], fresh: &[], standing: &[], headless: None, integration: "", needs_integration: false, cannot: &[], checked: "",
};
mod herdr;
mod own;

/// Every program, in the order they are named: the board's three first, then herdr's others.
pub fn all() -> impl Iterator<Item = &'static Kind> { own::KINDS.iter().chain(herdr::KINDS.iter()) }
/// The programs' names, as a setting gives them.
pub static KIND_NAMES: std::sync::LazyLock<Vec<&'static str>> = std::sync::LazyLock::new(|| all().map(|k| k.name).collect());
pub fn kind(name: &str) -> Option<&'static Kind> { all().find(|k| k.name == name) }
/// A program by its name or one of its other names.
pub fn kind_named(name: &str) -> Option<&'static Kind> { let n = name.to_lowercase(); all().find(|k| k.name == n || k.aliases.contains(&n.as_str())) }
/// The executable a kind is started as.
pub fn program(name: &str) -> &str { kind(name).map(|k| if k.program.is_empty() { k.name } else { k.program }).unwrap_or(name) }
/// The arguments of a template, `{}` given its value.
pub fn fill(tpl: &[&str], v: &str) -> Vec<String> { tpl.iter().map(|a| a.replace("{}", v)).collect() }
