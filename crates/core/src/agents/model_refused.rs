//! A model the agent's program refused, at its start or at a turn's end, told apart from its other errors and said in
//! plain words: "gpt-6.1 can't be used with your ChatGPT sign-in; try gpt-6.1-sol" (docs/models.md). The name to try
//! is the nearest one the program listed last for that sign-in: these words never wait for a program.
use super::models::{listed::kept_names, nearest};
use std::sync::LazyLock;

/// What was refused: the model as the error names it ("" when it names none), the sign-in it was refused for ("ChatGPT
/// sign-in", "API key"; "" when it says none), and the reason in plain words.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Refused { pub model: String, pub signin: String, pub plain: String }

static REFUSED: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)\bmodel\b[^\n]{0,80}\b(not supported|unsupported|not found|does not exist|doesn't exist|may not exist|not available|isn't available|no access|not allowed|unknown|invalid)\b|\b(unknown|invalid|unsupported) model\b|\bmodel_not_found\b").unwrap());
static NAMED: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r#"(?i)['"`‘“]([^'"`’”\s]{1,80})['"`’”] model\b|\bmodel[ :]+['"`‘“(]([^'"`’”)\s]{1,80})['"`’”)]|\bmodel ([a-z0-9][\w.:/\[\]=-]{0,79}[\w\]]) (?:is|was|does|may)\b"#).unwrap());
static SIGNIN: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)\bwith an? (ChatGPT account|API key)\b").unwrap());

/// The refusal in an error, when it is one. `kind`: the program, for the words.
pub fn read(kind: &str, error: &str) -> Option<Refused> {
    if !REFUSED.is_match(error) || error.to_lowercase().contains("at capacity") { return None; }
    let model = NAMED.captures(error).and_then(|c| c.iter().skip(1).flatten().next().map(|m| m.as_str().to_string())).filter(|m| m != "<synthetic>").unwrap_or_default();
    let signin = SIGNIN.captures(error).map(|c| if c[1].to_lowercase().starts_with("chatgpt") { "ChatGPT sign-in".to_string() } else { "API key".to_string() }).unwrap_or_default();
    let k = if kind.is_empty() { "the agent's program" } else { kind };
    let plain = match (model.is_empty(), signin.is_empty()) {
        (false, false) => format!("{model} can't be used with your {signin}"),
        (false, true) => format!("{k} can't use the model {model}"),
        (true, false) => format!("{k}'s model can't be used with your {signin}"),
        (true, true) => format!("{k} can't use its model"),
    };
    Some(Refused { model, signin, plain })
}
/// A name to try instead: the nearest one the program listed last (for its sign-in), else its first; "" when it has
/// listed none.
pub fn try_instead(kind: &str, model: &str) -> String {
    let Some(n) = kept_names(kind) else { return String::new() };
    let shown: Vec<String> = (if n.shown.is_empty() { n.taken } else { n.shown }).into_iter().filter(|x| x != model).collect();
    if model.is_empty() { return shown.first().cloned().unwrap_or_default(); }
    nearest(model, &shown, 1).into_iter().next().or_else(|| shown.first().cloned()).unwrap_or_default()
}
/// The reason, and the name to try where there is one: "gpt-6.1 can't be used with your ChatGPT sign-in; try gpt-6.1-sol".
pub fn say(kind: &str, r: &Refused) -> String {
    let x = try_instead(kind, &r.model);
    if x.is_empty() { r.plain.clone() } else { format!("{}; try {x}", r.plain) }
}
/// Where the model came from when the board gave the program none: its own settings.
pub fn own_default(kind: &str) -> String {
    match kind {
        "codex" => format!("codex's own default (`model` in {})", super::homes::codex_file().display()),
        "" => "the program's own default".into(),
        k => format!("{k}'s own default, from its own settings"),
    }
}
