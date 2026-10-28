//! OpenCode 2 is told its model another way than OpenCode 1 (#394): its full-screen app takes no `-m` and quits on it,
//! so the board makes the conversation first, with the model, through OpenCode's own server (`opencode api
//! session.create`, its docs at opencode.ai/v2/docs/api), and opens it with `--session`. Tried on OpenCode 2.1.4. A test
//! answers for `opencode api` (`set_fake`), so no program is started then.
use super::installed::{at_least, version_of};
use crate::err::Res;
use crate::json::*;
use std::cell::RefCell;
use std::path::Path;
use std::process::{Command, Stdio};
use std::rc::Rc;

type Fake = Rc<dyn Fn(&[String]) -> Result<String, String>>;
thread_local! { static FAKE: RefCell<Option<Fake>> = const { RefCell::new(None) }; }
/// A stand-in for `opencode`: it gets the arguments, and answers what it printed or why it failed.
pub fn set_fake(f: Option<Fake>) { FAKE.with(|x| *x.borrow_mut() = f); }
/// How long OpenCode is given to answer: its first call starts its background server.
const API_MS: u64 = 60000;

/// The installed OpenCode is version 2 or later. One whose version cannot be read is taken as version 1, as before.
pub fn is_v2() -> bool { let v = version_of("opencode"); !v.is_empty() && at_least(&v, "2") }

fn run(args: &[String]) -> Result<String, String> {
    if let Some(f) = FAKE.with(|x| x.borrow().clone()) { return f(args); }
    let child = Command::new(super::kinds::program("opencode")).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("it did not start ({e})"))?;
    let o = crate::herdr::cli::wait_with_timeout(child, API_MS).map_err(|_| format!("it did not answer within {} s", API_MS / 1000))?;
    if o.status == 0 { Ok(o.stdout) } else { Err(if o.stderr.trim().is_empty() { o.stdout } else { o.stderr }.trim().to_string()) }
}

/// A new conversation in `folder` on `model` (`<provider>/<model>`, as `opencode models` lists it): its id.
pub fn new_session(folder: &Path, model: &str) -> Res<String> {
    let (provider, id) = model.split_once('/').filter(|(p, m)| !p.is_empty() && !m.is_empty())
        .ok_or_else(|| format!("opencode could not be started on the model \"{model}\": OpenCode 2 takes it as <provider>/<model> (`opencode models` lists them)"))?;
    let body = json!({ "location": { "directory": folder.display().to_string() }, "model": { "providerID": provider, "id": id } });
    let args: Vec<String> = ["api", "session.create", "--data", &body.to_string()].iter().map(|s| s.to_string()).collect();
    let made = run(&args).map_err(|e| format!("OpenCode 2 could not make a conversation on {model} to start in: {e}"))?;
    let session = serde_json::from_str::<Value>(made.trim()).ok().map(|v| v.at("data").t("id")).unwrap_or_default();
    if session.is_empty() { crate::fail!("OpenCode 2 made no conversation on {model} to start in: it answered {}", made.trim().chars().take(300).collect::<String>()); }
    Ok(session)
}
