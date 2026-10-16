//! The session id of a Command Code pane, bound to its herdr socket and terminal across restarts and pane moves.
use crate::err::Res;
use crate::json::*;
use std::path::PathBuf;

fn path(socket: &str, terminal: &str) -> PathBuf {
    let key = sha1_smol::Sha1::from(format!("{socket}|{terminal}")).digest().to_string();
    crate::store::files::data_dir().join("commandcode").join(format!("{key}.json"))
}
pub fn args(socket: &str, terminal: &str) -> Res<Vec<String>> {
    let file = path(socket, terminal);
    std::fs::create_dir_all(file.parent().unwrap())?;
    let _ = std::fs::remove_file(&file);
    let module = file.with_extension("mjs");
    let source = format!("// This terminal's Command Code session, including new chats and resumes.\nimport {{ writeFileSync }} from 'node:fs';\nexport default function(cmd) {{ cmd.hooks({{ onSessionStart: ({{sessionId}}) => {{ if (sessionId) writeFileSync({}, JSON.stringify({{session: sessionId, cwd: cmd.cwd}}), {{mode: 0o600}}); }} }}); }}\n", serde_json::to_string(&file.display().to_string())?);
    super::super::adapter::write_private(&module, &source)?;
    Ok(vec!["--mod".into(), module.display().to_string()])
}
pub fn session(socket: &str, terminal: &str, cwd: &str) -> Option<String> {
    if terminal.is_empty() { return None; }
    let j: Value = serde_json::from_str(&std::fs::read_to_string(path(socket, terminal)).ok()?).ok()?;
    (j.s("cwd") == cwd && j.b("session")).then(|| j.t("session"))
}
