//! The `az` program, as gh is run for GitHub (flow/git.rs): never a prompt, at most a set time, its answer as gh's is
//! (`GhOut`). A test answers for az itself (`set_fake_az`), so no program is started then. az missing here, its
//! azure-devops extension missing, or no login: said as a login the human must give (store/failures.rs reads it so).
use super::url::Azure;
use crate::clock::now;
use crate::flow::git::GhOut;
use crate::json::*;
use crate::store::files::{data_dir, read_obj, write_json};
use std::path::Path;
use std::process::{Command, Stdio};

thread_local! { static FAKE_AZ: std::cell::RefCell<Option<std::rc::Rc<dyn Fn(&[String]) -> GhOut>>> = const { std::cell::RefCell::new(None) }; }
pub fn set_fake_az(f: Option<std::rc::Rc<dyn Fn(&[String]) -> GhOut>>) { FAKE_AZ.with(|x| *x.borrow_mut() = f); }

/// What to do where az cannot reach Azure DevOps here.
pub const LOGIN: &str = "Install the Azure CLI and its azure-devops extension (az extension add --name azure-devops) where the board runs, and log in (az login), then try again.";
/// az's own words for "no login", "no extension" and "no az", turned into a line the board takes for a login.
fn explained(stderr: &str) -> String {
    let e = stderr.to_lowercase();
    let why = if e.contains("az login") || e.contains("az devops login") || e.contains("please run 'az login'") || e.contains("refresh token has expired") || e.contains("tf400813") { "not logged in to Azure DevOps here" }
        else if e.contains("is not in the 'az' command group") || e.contains("misspelled or not recognized") || e.contains("extension add") { "az has no azure-devops extension here, so it is not logged in to Azure DevOps" }
        else { return stderr.to_string() };
    format!("Azure DevOps: {why}: authentication required. {LOGIN}\n{stderr}")
}
pub fn az(args: &[&str], cwd: &Path, timeout_ms: u64) -> GhOut {
    let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    if let Some(f) = FAKE_AZ.with(|x| x.borrow().clone()) { let mut r = f(&a); if r.status != 0 { r.stderr = explained(&r.stderr); } return r; }
    crate::runner::user_env();
    let child = Command::new("az").args(&a).current_dir(cwd).env("AZURE_CORE_ONLY_SHOW_ERRORS", "true").env("AZURE_CORE_NO_COLOR", "true").env("AZURE_CORE_COLLECT_TELEMETRY", "false").env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    match child {
        Err(e) => GhOut { status: -1, stdout: String::new(), stderr: format!("Azure DevOps: az is not installed here ({e}): authentication required. {LOGIN}") },
        Ok(c) => match crate::herdr::cli::wait_with_timeout(c, timeout_ms) {
            Ok(r) => GhOut { status: r.status, stdout: r.stdout, stderr: if r.status != 0 { explained(&r.stderr) } else { r.stderr } },
            Err(e) => GhOut { status: -1, stdout: String::new(), stderr: e.message },
        },
    }
}
/// What az said, both of its outputs.
pub fn said(r: &GhOut) -> String { [r.stdout.trim(), r.stderr.trim()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n") }
/// The organization every call names, never one az would guess from the folder.
fn on(a: &Azure) -> Vec<String> { vec!["--org".into(), a.org_url(), "--detect".into(), "false".into(), "--output".into(), "json".into()] }
/// `az <args> --org … --output json`, read as JSON. Err: what az said.
pub fn json_of(a: &Azure, cwd: &Path, args: &[&str], timeout_ms: u64) -> Result<Value, String> {
    let mut all: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    all.extend(on(a));
    let r = az(&all.iter().map(String::as_str).collect::<Vec<_>>(), cwd, timeout_ms);
    if r.status != 0 { return Err(said(&r)); }
    if r.stdout.trim().is_empty() { return Ok(Value::Null); }
    serde_json::from_str(r.stdout.trim()).map_err(|_| said(&r))
}
/// The same, keeping az's answer as it is (`GhOut`).
pub fn run(a: &Azure, cwd: &Path, args: &[&str], timeout_ms: u64) -> GhOut {
    let mut all: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    all.extend(on(a));
    az(&all.iter().map(String::as_str).collect::<Vec<_>>(), cwd, timeout_ms)
}
/// One call of Azure DevOps' REST API through `az devops invoke`, for what the az commands do not say: a build's
/// timeline and log, a pull request's threads, its completion by merge strategy. `body`: what is sent (a file of its own).
pub fn invoke(a: &Azure, cwd: &Path, area: &str, resource: &str, route: &[(&str, &str)], method: &str, body: Option<&Value>) -> Result<Value, String> {
    let params: Vec<String> = std::iter::once(format!("project={}", a.project)).chain(route.iter().map(|(k, v)| format!("{k}={v}"))).collect();
    let mut args: Vec<String> = vec!["devops".into(), "invoke".into(), "--area".into(), area.into(), "--resource".into(), resource.into(), "--route-parameters".into()];
    args.extend(params);
    args.extend(["--http-method".into(), method.into(), "--api-version".into(), "7.1".into()]);
    let file = body.map(|b| { let f = data_dir().join(format!("az-{}-{}.json", std::process::id(), now())); let _ = write_json(&f, b); f });
    if let Some(f) = &file { args.extend(["--in-file".into(), f.display().to_string()]); }
    let r = json_of(a, cwd, &args.iter().map(String::as_str).collect::<Vec<_>>(), 120000);
    if let Some(f) = file { let _ = std::fs::remove_file(f); }
    r
}
/// Who is logged in to Azure DevOps here: the human. Asked once a day; "" when az cannot say.
pub fn user(cwd: &Path) -> String {
    let file = data_dir().join("az-user.json");
    if let Some(k) = read_obj(&file) { if now() - k.i("at") < 24 * 3600 * 1000 { return k.t("login"); } }
    let r = az(&["account", "show", "--query", "user.name", "--output", "tsv"], cwd, 20000);
    let login = if r.status == 0 { r.stdout.trim().lines().next().unwrap_or("").to_string() } else { String::new() };
    let _ = write_json(&file, &json!({ "login": login, "at": if login.is_empty() { now() - 23 * 3600 * 1000 } else { now() } }));
    login
}
