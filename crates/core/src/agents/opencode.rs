//! OpenCode V2's session and model selection through its API, before its full terminal UI starts.
use super::start::StartOpts;
use crate::err::Res;
use crate::json::*;
use crate::util::url_encode;
use std::path::Path;

/// Only the known V2 API is used. Other versions keep the CLI arguments, including any configured model.
pub fn v2() -> bool { super::installed::version_of("opencode").split('.').next() == Some("2") }

fn api(cwd: &Path, method: &str, path: &str, body: Option<Value>) -> Res<String> {
    let data = body.map(|b| b.to_string());
    let mut args = vec!["api", method, path];
    if let Some(data) = &data { args.extend(["--data", data.as_str()]); }
    super::models::listed::run_in(super::kinds::program("opencode"), &args, Some(cwd))?.ok_or_else(|| "opencode is not installed here".into())
}
fn data(raw: &str) -> Res<Value> {
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("OpenCode API returned invalid JSON: {e}"))?;
    v.get("data").cloned().ok_or_else(|| "OpenCode API returned no data".into())
}
fn model_ref(v: &Value) -> Res<Value> {
    if v.s("providerID").is_empty() || v.s("id").is_empty() { crate::fail!("OpenCode returned no model; choose a provider/model before setting an effort."); }
    Ok(json!({ "providerID": v.s("providerID"), "id": v.s("id") }))
}
fn at(cwd: &Path, path: &str) -> String { format!("{path}?location[directory]={}", url_encode(&cwd.display().to_string())) }

/// Create once, or update the resumed session before opening it. Retrying the UI reuses these options.
pub fn prepare(cwd: &Path, o: StartOpts) -> Res<StartOpts> { prepare_with(cwd, o, api) }
fn prepare_with(cwd: &Path, mut o: StartOpts, mut api: impl FnMut(&Path, &str, &str, Option<Value>) -> Res<String>) -> Res<StartOpts> {
    let cwd = cwd.canonicalize()?;
    let effort = !o.effort.is_empty() && o.effort != "default";
    let mut model = if !o.model.is_empty() {
        let Some((provider, id)) = o.model.split_once('/').filter(|(p, m)| !p.is_empty() && !m.is_empty()) else { crate::fail!("OpenCode V2 needs a provider/model, not \"{}\".", o.model); };
        Some(json!({ "providerID": provider, "id": id }))
    } else if effort {
        let saved = if o.resume.is_empty() { Value::Null } else { data(&api(&cwd, "get", &format!("/api/session/{}", url_encode(&o.resume)), None)?)? };
        let current = if saved.at("model").is_object() { saved.at("model").clone() } else {
            let dir = saved.at("location").so("directory").map(Path::new).unwrap_or(&cwd);
            data(&api(&cwd, "get", &at(dir, "/api/model/default"), None)?)?
        };
        Some(model_ref(&current)?)
    } else { None };
    if let Some(m) = &mut model { if effort {
        let catalog = data(&api(&cwd, "get", &at(&cwd, "/api/model"), None)?)?;
        let found = catalog.as_array().and_then(|all| all.iter().find(|x| x.s("providerID") == m.s("providerID") && x.s("id") == m.s("id")));
        let Some(found) = found else { crate::fail!("OpenCode did not list {}/{}; its effort cannot be checked.", m.s("providerID"), m.s("id")); };
        if !found.arr("variants").iter().any(|v| v.s("id") == o.effort) { crate::fail!("OpenCode's {}/{} has no effort \"{}\"; its variants are {}.", m.s("providerID"), m.s("id"), o.effort, found.arr("variants").iter().map(|v| v.s("id")).collect::<Vec<_>>().join(", ")); }
        m["variant"] = json!(o.effort);
    } }
    if o.resume.is_empty() {
        let mut body = json!({ "location": { "directory": cwd } });
        if let Some(model) = model { body["model"] = model; }
        let session = data(&api(&cwd, "post", "/api/session", Some(body))?)?;
        let id = session.s("id");
        if id.is_empty() { crate::fail!("OpenCode created no session ID."); }
        o.session = id.into(); o.resume = id.into();
    } else if let Some(model) = model {
        api(&cwd, "post", &format!("/api/session/{}/model", url_encode(&o.resume)), Some(json!({ "model": model })))?;
    }
    o.model.clear(); o.effort.clear();
    Ok(o)
}

#[cfg(test)]
mod tests;
