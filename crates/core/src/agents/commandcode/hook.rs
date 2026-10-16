//! Command Code mods passed only to this launch: persistent standing rules and the board's turn-end callback.
use crate::agents::adapter::Hook;
use crate::err::Res;
use std::path::Path;

pub fn rules(text: &str, dir: &Path) -> Res<Vec<String>> {
    let file = dir.join("commandcode-rules.mjs");
    super::super::adapter::write_private(&file, &format!("// The board's standing rules for this launch.\nexport default function(cmd) {{ cmd.hooks({{ appendSystemPrompt: () => {} }}); }}\n", serde_json::to_string(text)?))?;
    Ok(vec!["--mod".into(), file.display().to_string()])
}
pub fn args(h: &Hook) -> Res<Vec<String>> {
    let file = h.dir.join(format!("commandcode-hook-{}.mjs", h.role));
    let source = include_str!("hook.js").replace("__SW_COMMAND__", &serde_json::to_string(&super::super::adapter::hook_command(h))?);
    super::super::adapter::write_private(&file, &source)?;
    Ok(vec!["--mod".into(), file.display().to_string()])
}
