//! Is an agent's pane showing a trust screen: a "trust this folder?" dialog,
//! a restricted-folder chooser, or a hooks or settings review? Such an answer
//! is saved by the harness for every later session in that folder, so text is
//! never typed into one: a brief or prompt ending in Enter would pick the
//! preselected "trust" option. Herdr does not always report these screens as
//! `blocked` (a new harness screen may read as `idle`), so the screen itself
//! is read right before any prompt is sent.

use crate::prompt_box::{self, Draft};

/// Lines that only a trust screen draws, from the harnesses' own strings.
const PHRASES: &[&str] = &[
    // Claude Code
    "Quick safety check: Is this a project you created or one you trust",
    "Do you trust the files in this folder?",
    "Yes, I trust this folder",
    "Yes, I trust these settings",
    // Codex
    "Trust this folder?",
    "Do you trust the contents of this directory?",
    "Hooks need review",
    "Trust all and continue",
    "Your trust decision will be saved",
    // Gemini CLI and Qwen Code
    "Do you trust this folder?",
    // Cursor
    "Workspace Trust Required",
    // Copilot CLI
    "Confirm folder trust",
];

/// The trust-screen line on `screen` (`agent read --format ansi`), or `None`.
/// A screen whose input box is visible and empty is the agent's prompt, and a
/// phrase on it is only transcript text (an agent quoting a dialog); menus,
/// dialogs and unknown layouts have no empty box.
pub fn detect(kind: &str, screen: &str) -> Option<&'static str> {
    if prompt_box::check(kind, screen) == Draft::Empty {
        return None;
    }
    let plain = plain_text(screen);
    PHRASES.iter().copied().find(|phrase| plain.contains(phrase))
}

/// The screen's text with escape sequences dropped and runs of spaces joined,
/// so a phrase drawn with styled or padded words still matches.
fn plain_text(screen: &str) -> String {
    let mut out = String::new();
    let mut chars = screen.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&c) {
                        break;
                    }
                }
            } else {
                chars.next();
            }
            continue;
        }
        if ch == '\n' {
            out.push('\n');
        } else if ch.is_whitespace() || ch.is_control() {
            if !out.ends_with(' ') {
                out.push(' ');
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// The trust-screen line in pane `pane` now, read right before typing there.
/// A screen that cannot be read is an error: the caller does not type.
pub fn showing(herdr: &crate::herdr::Herdr, pane: &str, kind: &str) -> Result<Option<&'static str>, crate::herdr::HerdrError> {
    Ok(detect(kind, &herdr.agent_screen(pane)?))
}

/// Who answers trust screens in thread panes (`trust_screens`).
pub const COORDINATOR: &str = "coordinator";
pub const USER: &str = "user";

/// The refusal for text or keys aimed at a trust screen.
pub fn refusal(id: &str, pane: &str, phrase: &str, by_user: bool) -> String {
    let who = if by_user {
        "trust_screens = user: only the user answers it, in the pane; tell them in chat".to_string()
    } else {
        "answer it first (`thread read` shows it, `thread keys` answers it)".to_string()
    };
    format!("trust_screen: {id}'s pane {pane} shows a trust screen (\"{phrase}\"); nothing is typed into it; {who}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_harness_trust_screen_is_recognized() {
        let claude = "╭────╮\n│ Accessing workspace:\n│ Quick safety check: Is this a project you created or one you trust? (Like your own code)\n│ \u{1b}[1m❯\u{1b}[0m 1. Yes, I trust this folder\n│   2. No, exit\n╰────╯\n";
        assert!(detect("claude", claude).is_some());
        let codex_folder = "  Trust this folder? Codex can read, edit, and run files here.\n\u{1b}[1m›\u{1b}[0m 1. Trust and continue\n  2. Open restricted\n";
        assert_eq!(detect("codex", codex_folder), Some("Trust this folder?"));
        let codex_hooks = "  Hooks need review\n  1 hook is new or changed.\n› 1. Review hooks\n  2. Trust all and continue\n";
        assert_eq!(detect("codex", codex_hooks), Some("Hooks need review"));
        let old_codex = "  Do you trust the contents of this directory?\n\u{1b}[1m›\u{1b}[0m 1. Yes, continue\n  2. No, quit\n";
        assert!(detect("codex", old_codex).is_some());
        assert!(detect("gemini", "│ Do you trust this folder?\n│ ● 1. Trust folder (demo)\n").is_some());
        assert!(detect("copilot", "Confirm folder trust\n❯ 1. Yes\n").is_some());
        // Styling between words does not hide it.
        assert!(detect("kiro", "Hooks \u{1b}[1mneed\u{1b}[0m   review\n").is_some());
    }

    #[test]
    fn ordinary_screens_and_quoted_phrases_are_not_trust_screens() {
        let rule = "─".repeat(40);
        // A transcript quoting the dialog above an empty input box.
        let quoted = format!("the dialog said \"Do you trust the files in this folder?\"\n{rule}\n❯ \n{rule}\n");
        assert_eq!(detect("claude", &quoted), None);
        assert_eq!(detect("codex", "  Welcome to Codex\n› 1. Sign in with ChatGPT\n"), None);
        assert_eq!(detect("claude", "Allow this edit?\n❯ 1. Yes\n  2. No\n"), None);
        assert_eq!(detect("claude", ""), None);
    }
}
