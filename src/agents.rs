//! What the binary knows about Herdr agent kinds: the 24 kinds `agent start`
//! accepts and the per-kind resume arguments from Herdr's session-state page
//! (herdr.dev, "Native agent session restore", 0.9.1).

pub const KINDS: [&str; 24] = [
    "pi", "claude", "codex", "gemini", "cursor", "devin", "agy", "cline", "omp", "mastracode", "opencode", "copilot", "kimi", "kiro", "droid",
    "amp", "grok", "hermes", "kilo", "qodercli", "qwen", "letta", "maki", "muse",
];

pub fn is_kind(kind: &str) -> bool {
    KINDS.contains(&kind)
}

/// The arguments that resume a native session `id` for `kind`, or `None` for
/// a kind whose resume command Herdr does not document.
pub fn resume_args(kind: &str, id: &str) -> Option<Vec<String>> {
    if id.is_empty() || id.starts_with('-') {
        return None;
    }
    let args: Vec<String> = match kind {
        "claude" | "cursor" | "grok" | "devin" | "droid" | "qodercli" | "qwen" | "hermes" => vec!["--resume".into(), id.into()],
        "codex" => vec!["resume".into(), id.into()],
        "omp" | "copilot" => vec![format!("--resume={id}")],
        "pi" | "opencode" | "kimi" | "kilo" => vec!["--session".into(), id.into()],
        "agy" | "letta" => vec!["--conversation".into(), id.into()],
        "mastracode" => vec!["--thread".into(), id.into()],
        _ => return None,
    };
    Some(args)
}

/// Whether `value` looks like a model name: `opus`, `gpt-5.5`,
/// `anthropic/claude-sonnet-5`, `claude-opus-5-5[1m]`. Never a flag.
fn is_model_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with('-')
        && value.chars().all(|c| c.is_ascii_alphanumeric() || "._-/:@+[]".contains(c))
}

/// Splits a thread's or coordinator's `--agent-arg` values into the model
/// flags they may carry and everything else, read as pairs: `--model NAME`
/// and `--model=NAME` for every kind, plus `-m NAME` for Codex. Any other
/// launch flag belongs in the user's own `*_agent_args` safety settings, so
/// an agent can never widen another agent's powers.
pub fn split_model_args(kind: &str, args: &[String]) -> (Vec<String>, Vec<String>) {
    let (mut allowed, mut refused) = (Vec::new(), Vec::new());
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let is_flag = arg == "--model" || (kind == "codex" && arg == "-m");
        if is_flag && args.get(i + 1).is_some_and(|v| is_model_name(v)) {
            allowed.extend([arg.clone(), args[i + 1].clone()]);
            i += 2;
            continue;
        }
        if arg.strip_prefix("--model=").is_some_and(is_model_name) {
            allowed.push(arg.clone());
        } else {
            refused.push(arg.clone());
        }
        i += 1;
    }
    (allowed, refused)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|a| a.to_string()).collect()
    }

    #[test]
    fn agent_args_carry_only_a_model_flag() {
        for kind in ["claude", "codex", "gemini", "opencode", "cursor", "copilot"] {
            for args in [&["--model", "opus"][..], &["--model=gpt-5.5"], &["--model", "anthropic/claude-sonnet-5"], &["--model", "claude-opus-5-5[1m]"], &[]] {
                assert_eq!(split_model_args(kind, &strings(args)), (strings(args), vec![]), "{kind} {args:?}");
            }
        }
        assert_eq!(split_model_args("codex", &strings(&["-m", "gpt-5.5"])), (strings(&["-m", "gpt-5.5"]), vec![]));
        assert_eq!(split_model_args("claude", &strings(&["-m", "opus"])), (vec![], strings(&["-m", "opus"])));

        let refused = |args: &[&str]| split_model_args("claude", &strings(args));
        assert_eq!(refused(&["--dangerously-skip-permissions"]), (vec![], strings(&["--dangerously-skip-permissions"])));
        assert_eq!(split_model_args("codex", &strings(&["--yolo"])), (vec![], strings(&["--yolo"])));
        assert_eq!(refused(&["--model"]), (vec![], strings(&["--model"])));
        assert_eq!(refused(&["--model", "--foo"]), (vec![], strings(&["--model", "--foo"])));
        assert_eq!(refused(&["--model", "x", "--extra"]), (strings(&["--model", "x"]), strings(&["--extra"])));
        assert_eq!(refused(&["--model="]), (vec![], strings(&["--model="])));
        assert_eq!(refused(&["--model=-x"]), (vec![], strings(&["--model=-x"])));
        assert_eq!(refused(&["--model", "a b"]), (vec![], strings(&["--model", "a b"])));
        assert_eq!(refused(&["--model", "$(id)"]), (vec![], strings(&["--model", "$(id)"])));
        assert_eq!(refused(&["opus"]), (vec![], strings(&["opus"])));
    }

    #[test]
    fn resume_arguments_follow_herdrs_table() {
        assert_eq!(resume_args("claude", "abc").unwrap(), ["--resume", "abc"]);
        assert_eq!(resume_args("codex", "abc").unwrap(), ["resume", "abc"]);
        assert_eq!(resume_args("opencode", "abc").unwrap(), ["--session", "abc"]);
        assert_eq!(resume_args("copilot", "abc").unwrap(), ["--resume=abc"]);
        assert_eq!(resume_args("gemini", "abc"), None);
        assert_eq!(resume_args("claude", ""), None);
        assert_eq!(resume_args("claude", "--dangerous"), None);
        assert!(is_kind("claude") && !is_kind("chatgpt"));
        assert_eq!(KINDS.len(), 24);
    }
}
