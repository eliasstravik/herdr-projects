//! Spaces and tabs on Rex. A project is one Rex session, named `hp-<repo>` (never somebody else's session); one of the board's spaces is a list of
//! windows in it (its first window's id is the space's id), a tab is a window. Every block is made with the board's
//! environment and `HERDR_PROJECTS_SOCKET` naming this server, so the commands an agent runs find their board.
use super::{registry, run, Rex};
use crate::herdr::cli::{args, HRes};
use crate::json::*;

/// Each block's shell, started with these variables set (Rex hands a new block no environment of its own).
pub fn shell_cmd(env: &[(String, String)], socket: &str) -> Vec<String> {
    let mut c = vec!["--shell".to_string(), "none".into(), "--".into(), "/usr/bin/env".into(), format!("HERDR_PROJECTS_SOCKET={socket}")];
    c.extend(env.iter().filter(|(k, _)| k != "HERDR_PROJECTS_SOCKET").map(|(k, v)| format!("{k}={v}")));
    c.extend([std::env::var("SHELL").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| "/bin/zsh".into()), "-l".into()]);
    c
}
/// The project a folder belongs to: its repo's main folder (a worktree counts as its repo's), else the folder itself.
/// Its key is that whole path, so two repos of one name are two projects.
pub fn project_of(cwd: &str) -> String { super::worktrees::repo_root(cwd).unwrap_or_else(|| cwd.to_string()) }
/// A project's session's label: `hp-<folder name>`.
fn session_label(project: &str) -> String { format!("hp-{}", std::path::Path::new(project).file_name().map(|n| n.to_string_lossy().into_owned()).filter(|n| !n.is_empty()).unwrap_or_else(|| "root".into())) }

impl Rex {
    pub fn sessions(&self) -> HRes<Vec<Value>> { Ok(run::rex_json(self.srv(), &args(&["ls", "--json"]), 15000)?.arr("sessions").to_vec()) }
    /// A session's windows and blocks (`session.view`).
    pub fn view(&self, session: &str) -> HRes<Value> { run::api(self.srv(), "session.view", &json!({ "session_id": session })) }
    /// Every window of a view, with its blocks: (window id, label, block ids).
    pub fn windows_of(view: &Value) -> Vec<(String, String, Vec<String>)> {
        view.arr("windows").iter().map(|w| (w.t("window_id"), w.t("label"), w.arr("layers").iter().flat_map(|l| l.arr("blocks").iter().map(|b| b.t("block_id"))).collect())).collect()
    }
    /// The session of a project, made when there is none: (session id, the window it was made with). Only a session
    /// the board made is used: never one of the same name somebody else has.
    fn project_session(&self, project: &str, cwd: &str, label: &str, env: &[(String, String)]) -> HRes<(String, Option<(String, String)>)> {
        let live = self.sessions()?;
        let known = registry::read(self.srv()).at("projects").t(project);
        if let Some(s) = live.iter().find(|s| !known.is_empty() && s.s("session_id") == known) { return Ok((s.t("session_id"), None)); }
        let mut a = args(&["new", &session_label(project), "-c", cwd, "--window", label, "--json"]);
        a.extend(shell_cmd(env, &self.socket()));
        let made = run::rex_json(self.srv(), &a, 30000)?;
        let s = made.t("session_id");
        let w = made.arr("initial_windows").first().cloned().unwrap_or(Value::Null);
        registry::change(self.srv(), |r| r["projects"][project] = json!(s))?;
        Ok((s, Some((w.t("window_id"), w.arr("block_ids").first().and_then(|b| b.as_str()).unwrap_or_default().to_string()))))
    }
    /// A window in a session: (window, its block).
    fn new_window(&self, session: &str, cwd: &str, label: &str, env: &[(String, String)]) -> HRes<(String, String)> {
        let mut a = args(&["window", "new", "-s", session, if label.is_empty() { "shell" } else { label }, "-c", cwd, "--focus=false", "--json"]);
        a.extend(shell_cmd(env, &self.socket()));
        let w = run::rex_json(self.srv(), &a, 30000)?;
        Ok((w.t("window_id"), w.arr("block_ids").first().and_then(|b| b.as_str()).unwrap_or_default().to_string()))
    }
    fn label_or(cwd: &str, label: &str) -> String { if label.is_empty() { std::path::Path::new(cwd).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default() } else { label.into() } }
    /// A new space in its project's session: herdr's answer of `workspace create` (workspace, tab, root_pane).
    pub fn space_create(&self, cwd: &str, label: &str, env: &[(String, String)], worktree: Value) -> HRes<Value> {
        let label = Self::label_or(cwd, label);
        let project = project_of(cwd);
        let (session, first) = self.project_session(&project, cwd, &label, env)?;
        let (window, block) = match first { Some(f) => f, None => self.new_window(&session, cwd, &label, env)? };
        let space = json!({ "session": session, "label": label, "cwd": cwd, "worktree": worktree, "windows": [window] });
        registry::change(self.srv(), |r| { r["spaces"][&window] = space.clone(); r["blocks"][&block] = json!({ "window": window, "cwd": cwd }); })?;
        let ws = self.space_out(&window, &space, 1);
        let tab = json!({ "tab_id": window, "workspace_id": window, "label": label, "pane_count": 1, "focused": true });
        Ok(json!({ "workspace": ws, "tab": tab, "root_pane": self.pane_out(&block, &window, &window, cwd, "") }))
    }
    pub fn space_out(&self, id: &str, s: &Value, panes: usize) -> Value {
        let mut o = json!({ "workspace_id": id, "label": s.at("label"), "pane_count": panes, "cwd": s.at("cwd") });
        if s.at("worktree").is_object() { o["worktree"] = s.at("worktree").clone(); }
        o
    }
    /// The spaces still there: a space whose windows are all gone is forgotten.
    pub fn spaces(&self) -> HRes<Vec<Value>> {
        let reg = registry::read(self.srv());
        let live: Vec<String> = self.sessions()?.iter().map(|s| s.t("session_id")).collect();
        let mut out = vec![]; let mut gone = vec![];
        for (id, s) in reg.at("spaces").as_object().cloned().unwrap_or_default() {
            let view = if live.contains(&s.t("session")) { self.view(s.s("session")).ok() } else { None };
            let wins = view.as_ref().map(Self::windows_of).unwrap_or_default();
            let mine: Vec<_> = wins.iter().filter(|w| s.arr("windows").iter().any(|x| x.as_str() == Some(&w.0))).collect();
            if mine.is_empty() { if view.is_some() || !live.contains(&s.t("session")) { gone.push(id); } continue; }
            out.push(self.space_out(&id, &s, mine.iter().map(|w| w.2.len()).sum()));
        }
        if !gone.is_empty() { registry::change(self.srv(), |r| for g in &gone { if let Some(o) = r["spaces"].as_object_mut() { o.remove(g); } })?; }
        Ok(out)
    }
    pub fn space_close(&self, id: &str) -> HRes<Value> {
        let Some(s) = registry::space(&registry::read(self.srv()), id) else { return Err(run::err("workspace_not_found", "no such space")) };
        let live: Vec<String> = Self::windows_of(&self.view(s.s("session"))?).into_iter().map(|w| w.0).collect();
        let open: Vec<String> = strings(s.at("windows")).into_iter().filter(|w| live.contains(w)).collect();
        if !open.is_empty() { let mut a = args(&["window", "close"]); a.extend(open); run::rex(self.srv(), &a, "", 15000)?; }
        registry::change(self.srv(), |r| if let Some(o) = r["spaces"].as_object_mut() { o.remove(id); })?;
        Ok(json!({ "type": "ok" }))
    }
    /// Its first window still open (the one its id names may have closed).
    pub fn space_focus(&self, id: &str) -> HRes<Value> {
        let first = self.windows(id)?.into_iter().next().ok_or_else(|| run::err("workspace_not_found", "the space has no window left"))?;
        self.window_focus(first.s("tab_id"))
    }
    pub fn window_focus(&self, id: &str) -> HRes<Value> { run::rex(self.srv(), &args(&["window", "focus", id]), "", 15000)?; Ok(json!({ "type": "ok" })) }
    /// A new tab of a space: a window in its session (`tab create`'s answer).
    pub fn window_create(&self, space: &str, cwd: &str, label: &str, env: &[(String, String)]) -> HRes<Value> {
        let Some(s) = registry::space(&registry::read(self.srv()), space) else { return Err(run::err("workspace_not_found", "no such space")) };
        let cwd = if cwd.is_empty() { s.t("cwd") } else { cwd.to_string() };
        let (window, block) = self.new_window(s.s("session"), &cwd, label, env)?;
        registry::change(self.srv(), |r| { if let Some(a) = r["spaces"][space]["windows"].as_array_mut() { a.push(json!(window)); } r["blocks"][&block] = json!({ "window": window, "cwd": cwd }); })?;
        let tab = json!({ "tab_id": window, "workspace_id": space, "label": label, "pane_count": 1, "focused": false });
        Ok(json!({ "type": "tab_created", "tab": tab, "root_pane": self.pane_out(&block, space, &window, &cwd, "") }))
    }
    /// The tabs of a space, in Rex's order.
    pub fn windows(&self, space: &str) -> HRes<Vec<Value>> {
        let Some(s) = registry::space(&registry::read(self.srv()), space) else { return Err(run::err("workspace_not_found", "no such space")) };
        let view = self.view(s.s("session"))?;
        let active = view.t("active_window_id");
        Ok(Self::windows_of(&view).into_iter().filter(|w| s.arr("windows").iter().any(|x| x.as_str() == Some(&w.0)))
            .map(|(id, label, blocks)| json!({ "tab_id": id, "workspace_id": space, "label": label, "pane_count": blocks.len(), "focused": id == active })).collect())
    }
    pub fn window_get(&self, id: &str) -> Option<Value> {
        let (space, _) = registry::space_of_window(&registry::read(self.srv()), id)?;
        self.windows(&space).ok()?.into_iter().find(|t| t.s("tab_id") == id)
    }
    pub fn window_rename(&self, id: &str, label: &str) -> Option<Value> { run::rex(self.srv(), &args(&["window", "rename", id, label]), "", 15000).ok()?; self.window_get(id) }
    pub fn window_close(&self, id: &str) -> Option<Value> {
        run::rex(self.srv(), &args(&["window", "close", id]), "", 15000).ok()?;
        let _ = registry::change(self.srv(), |r| for (_, s) in r["spaces"].as_object_mut().into_iter().flatten() { if let Some(a) = s["windows"].as_array_mut() { a.retain(|w| w.as_str() != Some(id)); } });
        Some(json!({ "type": "ok" }))
    }
    /// The board's own window, in the session of the folder it was started in: made once, then found again.
    pub fn board(&self) -> HRes<String> {
        let cwd = std::env::current_dir().map(|d| d.display().to_string()).unwrap_or_else(|_| "/".into());
        let env = crate::store::files::board_env();
        let (session, first) = self.project_session(&project_of(&cwd), &cwd, "board", &env)?;
        let found = if first.is_some() { None } else { Self::windows_of(&self.view(&session)?).into_iter().find(|w| w.1 == "board" && !w.2.is_empty()).map(|w| (w.0, w.2[0].clone())) };
        let (window, block) = match found {
            Some(f) => f,
            None => {
                let (window, block) = match first { Some(f) => f, None => self.new_window(&session, &cwd, "board", &env)? };
                run::rex(self.srv(), &args(&["send", "-b", &block]), &format!("{} board\n", super::agents::shq(&crate::store::files::bin().display().to_string())), 15000)?;
                (window, block)
            }
        };
        self.window_focus(&window)?;
        Ok(block)
    }
    /// The few herdr command lines the board sends as they are (`workspace get`, `rename`, `report-metadata`).
    pub fn herdr_line(&self, a: &[String]) -> HRes<Value> {
        let w = |i: usize| a.get(i).map(String::as_str).unwrap_or("");
        match (w(0), w(1)) {
            ("workspace", "get") => self.spaces()?.into_iter().find(|s| s.s("workspace_id") == w(2)).map(|s| json!({ "type": "workspace_info", "workspace": s })).ok_or_else(|| run::err("workspace_not_found", "no such space")),
            ("workspace", "rename") => { let label = a[3..].join(" "); registry::change(self.srv(), |r| if r["spaces"][w(2)].is_object() { r["spaces"][w(2)]["label"] = json!(label); })?; let _ = run::rex(self.srv(), &args(&["window", "rename", w(2), &label]), "", 15000); Ok(json!({ "type": "ok" })) }
            ("workspace", "report-metadata") => Ok(json!({ "type": "ok" })), // Rex has no sidebar to show it in
            _ => Err(run::err("unsupported", format!("Rex has no `{} {}` (docs/rex.md).", w(0), w(1)))),
        }
    }
}
