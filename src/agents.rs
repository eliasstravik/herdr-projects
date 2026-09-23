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

#[cfg(test)]
mod tests {
    use super::*;

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
