//! Herdr-native and user-reported programs. The board reads none of their conversations: Herdr's state says when a turn
//! ended, and the agent's closing note says what it said. Entries document interactive starts only; headless fields stay
//! unset. User-reported CLIs rely on their installed Herdr reporting plugins.
use super::{Headless, Kind, Login, Models, Read, BASE, NO_MODELS};

const ANSWER: &[&str] = &["{}"];
pub static KINDS: &[Kind] = &[
    Kind { name: "agy", aliases: &["antigravity-cli", "antigravity"], integration: "antigravity-cli",
        model: &["--model", "{}"], efforts: &["low", "medium", "high", "xhigh", "max"], effort: &["--effort", "{}"],
        access_full: &["--dangerously-skip-permissions"], resume: &["--conversation", "{}"],
        headless: Some(Headless { args: &["--disable-slash-commands"], model: &["--model", "{}"], system: &[], prompt: &["-p", "{}"] }),
        checked: "flags from agy --help 1.3.6; started by herdr 0.9.3 on the board's flags (it came up at its login)", ..BASE },
    Kind { name: "copilot", integration: "copilot", model: &["--model", "{}"], efforts: &["none", "minimal", "low", "medium", "high", "xhigh", "max"],
        effort: &["--reasoning-effort", "{}"], access_full: &["--yolo"], resume: &["--resume={}"], session: &["--session-id", "{}"], rules_file: Some("AGENTS.md"),
        headless: Some(Headless { args: &["-s", "--no-custom-instructions", "--no-ask-user", "--allow-all-tools"], model: &["--model", "{}"], system: &[], prompt: &["-p", "{}"] }),
        checked: "flags from copilot --help 1.0.98; started by herdr 0.9.3 on the board's flags (its folder-trust dialog)", ..BASE },
    // Started on a task through its program, cursor-agent (herdr never starts the plain `agent` it also installs).
    Kind { name: "cursor", program: "cursor-agent", aliases: &["cursor-agent"], integration: "cursor", model: &["--model", "{}"],
        models: Models { list: &["models"], read: Read::Dashed, suffix: &["["], example: "cursor/claude-opus-4-8[effort=high]", ..NO_MODELS },
        effort_note: "cursor takes its effort in the model's name: cursor/claude-opus-4-8[effort=high]",
        access_full: &["--yolo"], resume: &["--resume", "{}"], rules_file: Some("AGENTS.md"),
        login: Some(Login { status: &["status"], how: "cursor-agent login" }), // says "Not logged in", and still exits 0
        checked: "interactive flags from cursor-agent --help 2026.10.01; headless mode excluded", ..BASE },
    Kind { name: "freebuff", resume: &["--continue", "{}"],
        checked: "interactive flags from freebuff --help; Herdr state supplied by the user's installed Freebuff plugin", ..BASE },
    Kind { name: "codebuddy", model: &["--model", "{}"], access_full: &["--dangerously-skip-permissions"], resume: &["--resume", "{}"],
        checked: "interactive flags from codebuddy --help; Herdr state supplied by the user's installed CodeBuddy plugin", ..BASE },
    Kind { name: "cline", program: "cline", lead: &["--tui"], model: &["--model", "{}"], efforts: &["none", "low", "medium", "high", "xhigh"],
        effort: &["--thinking", "{}"], access_full: &["--auto-approve", "true"], access_ask: Some(&["--auto-approve", "false"]), resume: &["--id", "{}"],
        checked: "interactive TUI, model, effort, permission, and session flags from cline --help; Herdr supports Cline pane detection", ..BASE },
    Kind { name: "gemini", model: &["--model", "{}"], access_full: &["--yolo"], resume: &["--resume", "{}"], rules_file: Some("GEMINI.md"),
        checked: "interactive start and resume from Gemini CLI reference; Herdr 0.9.3 supports Gemini CLI panes", ..BASE },
    Kind { name: "kiro", program: "kiro-cli", lead: &["chat"], efforts: &["low", "medium", "high", "xhigh", "max"], effort: &["--effort", "{}"],
        access_full: &["--trust-all-tools"], resume: &["--resume-id", "{}"],
        checked: "interactive start, effort, and resume from Kiro CLI reference; Herdr supports Kiro CLI panes", ..BASE },
    Kind { name: "devin", integration: "devin", model: &["--model", "{}"], access_full: &["--permission-mode", "dangerous"], resume: &["--resume", "{}"],
        headless: Some(Headless { args: &["--respect-workspace-trust", "false"], model: &["--model", "{}"], system: &[], prompt: &["-p", "{}"] }),
        checked: "flags from devin --help 3000.11.92; started by herdr 0.9.3 on the board's flags (it came up at its login)", ..BASE },
    // Started for a task, droid takes no model and no effort: those are its own settings. Its one-shot does.
    Kind { name: "droid", integration: "droid", access_full: &["--auto", "high"], resume: &["--resume", "{}"], standing: &["--append-system-prompt-file", "{}"],
        rules_file: Some("AGENTS.md"),
        headless: Some(Headless { args: &["exec"], model: &["-m", "{}"], system: &["--append-system-prompt", "{}"], prompt: ANSWER }),
        checked: "flags from droid --help 0.235.0; a task on 0.242.0 in herdr 0.9.3: started, prompted and its turn's end seen", ..BASE },
    Kind { name: "grok", integration: "grok", model: &["-m", "{}"], effort: &["--reasoning-effort", "{}"], access_full: &["--always-approve"],
        resume: &["--resume", "{}"], session: &["--session-id", "{}"],
        checked: "interactive flags from grok --help 1.0.52; headless mode excluded", ..BASE },
    // `hermes chat`: the interactive chat, whose model can be given at start.
    Kind { name: "hermes", integration: "hermes", lead: &["chat"], model: &["-m", "{}"], access_full: &["--yolo"], resume: &["--resume", "{}"],
        rules_file: Some("AGENTS.md"),
        headless: Some(Headless { args: &["--ignore-rules"], model: &["-m", "{}"], system: &[], prompt: &["-z", "{}"] }),
        checked: "flags from hermes --help 0.19.0; started by herdr 0.9.3 on the board's flags (its first setup)", ..BASE },
    Kind { name: "kilo", integration: "kilo", model: &["-m", "{}"], access_full: &["--auto"], resume: &["--session", "{}"], rules_file: Some("AGENTS.md"),
        headless: Some(Headless { args: &["run"], model: &["-m", "{}"], system: &[], prompt: ANSWER }),
        checked: "flags from kilo --help 7.8.12; started by herdr 0.9.3 on the board's flags as its own program (npm's launcher hides it from herdr)", ..BASE },
    // Kimi Code (the `kimi` of `~/.kimi-code`, not the older kimi-cli).
    Kind { name: "kimi", integration: "kimi", model: &["-m", "{}"], access_full: &["--auto"], resume: &["--session", "{}"], rules_file: Some("AGENTS.md"),
        headless: Some(Headless { args: &[], model: &["-m", "{}"], system: &[], prompt: &["-p", "{}"] }),
        checked: "flags from kimi --help 2.2.0; started by herdr 0.9.3 on the board's flags (its folder-trust dialog)", ..BASE },
    // A Letta agent keeps one conversation per folder unless told to start a new one.
    Kind { name: "letta", integration: "letta", model: &["-m", "{}"], access_full: &["--yolo"], resume: &["--conversation", "{}"], fresh: &["--new"],
        headless: Some(Headless { args: &["--ephemeral"], model: &["-m", "{}"], system: &[], prompt: &["-p", "{}"] }),
        checked: "flags from letta --help 0.34.15; started by herdr 0.9.3 on the board's flags (its first setup held it)", ..BASE },
    Kind { name: "mastracode", integration: "mastracode", needs_integration: true,
        cannot: &[("worker", MASTRA), ("reviewer", MASTRA), ("coordinator", MASTRA), ("plan", MASTRA)],
        headless: Some(Headless { args: &["--permission-mode", "deny"], model: &["-m", "{}"], system: &[], prompt: &["--prompt", "{}"] }),
        checked: "flags from mastracode's own source 0.47.0", ..BASE },
    Kind { name: "omp", integration: "omp", needs_integration: true, model: &["--model={}"], efforts: &["off", "minimal", "low", "medium", "high", "xhigh", "max", "auto"],
        effort: &["--thinking={}"], access_full: &["--auto-approve"], access_ask: Some(&["--approval-mode=write"]), resume: &["--resume={}"],
        standing: &["--append-system-prompt={}"], rules_file: Some("AGENTS.md"),
        headless: Some(Headless { args: &["-p", "--no-session", "--no-tools", "--no-skills", "--no-rules"], model: &["--model={}"], system: &["--append-system-prompt={}"], prompt: ANSWER }),
        checked: "flags from omp --help 18.11.1; started by herdr 0.9.3 on the board's flags (its first setup)", ..BASE },
    Kind { name: "opencode", integration: "opencode", model: &["-m", "{}"],
        models: Models { list: &["models"], read: Read::Lines, example: "opencode/anthropic/claude-opus-5-5", ..NO_MODELS }, access_full: &["--auto"], resume: &["--session", "{}"], rules_file: Some("AGENTS.md"),
        checked: "interactive flags from opencode --help 1.18.34; headless mode excluded", ..BASE },
    Kind { name: "qodercli", integration: "qodercli", model: &["-m", "{}"], effort: &["--reasoning-effort", "{}"], access_full: &["--dangerously-skip-permissions"],
        resume: &["--resume", "{}"], session: &["--session-id", "{}"],
        checked: "interactive flags from qodercli --help 1.1.72; headless mode excluded; Herdr pane detection needs live verification", ..BASE },
    Kind { name: "qwen", integration: "qwen", model: &["-m", "{}"], access_full: &["--yolo"], resume: &["--resume", "{}"], session: &["--session-id", "{}"],
        rules_file: Some("QWEN.md"),
        headless: Some(Headless { args: &[], model: &["-m", "{}"], system: &["--append-system-prompt", "{}"], prompt: &["-p", "{}"] }),
        checked: "flags from qwen --help 0.25.2; started by herdr 0.9.3 on the board's flags", ..BASE },
];
const MASTRA: &str = "mastracode can be told neither its model nor to work without asking when it starts (only through environment \
variables, which herdr's start cannot set), and it reopens its folder's last conversation, so it cannot work on a task: it can be the Watcher";
