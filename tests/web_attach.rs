//! A file attached in the web board's chat box, as in the herdr web UI's own: kept where whoever gets the words can
//! read it (the Coordinator's folder, a task's working folder out of git's way, the board's folder with no
//! Coordinator; a heading's coordinator's own folder), under a name of its own, its path answered for the box to put in
//! the message; never for a machine the board cannot reach, never over the web UI's limit, never from a device that may
//! only watch (another machine's: web_attach_far.rs). And the page's code, read as text:
//! the add button, the tiles, paste and drop, and the path at the start of the words.
use crate::common;
use common::board::{web, web_as};
use common::*;
use serde_json::{json, Value};
use sw::json::*;

const FILES: &str = include_str!("../crates/web/src/webui/page/files.js");
const FILES_CSS: &str = include_str!("../crates/web/src/webui/page/files.css");
const COMPOSE: &str = include_str!("../crates/web/src/webui/page/compose.js");
const CHAT: &str = include_str!("../crates/web/src/webui/page/chat.js");
const START: &str = include_str!("../crates/web/src/webui/page/start.js");

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nnot really a picture";
fn file(name: &str, kind: &str, bytes: &[u8], to: Value) -> Value { json!({ "name": name, "type": kind, "data": sw::util::base64(bytes), "to": to }) }
fn kept(out: &Value) -> std::path::PathBuf { std::path::PathBuf::from(out.s("path")) }

#[test]
fn a_file_for_the_new_task_box_is_kept_in_the_boards_folder_under_a_name_of_its_own() {
    let w = world();
    let out = web(&w, "attach", file("", "image/png", PNG, json!({ "id": "", "reply": false })));
    let p = kept(&out);
    assert!(p.starts_with(sw::store::files::data_dir().join("files")), "{}", p.display());
    let name = p.file_name().unwrap().to_string_lossy().to_string();
    assert!(name.starts_with("paste-") && name.ends_with(".png"), "a picture is kept as a paste: {name}");
    assert_eq!(std::fs::read(&p).unwrap(), PNG);
    assert_eq!(std::fs::read_to_string(p.parent().unwrap().join(".gitignore")).unwrap(), "*\n", "git leaves the folder alone");
    // Any other file keeps its own name, made safe; a second one never takes the first one's place.
    let a = kept(&web(&w, "attach", file("my notes (v2).txt", "text/plain", b"hello", json!({ "id": "", "reply": false }))));
    let b = kept(&web(&w, "attach", file("my notes (v2).txt", "text/plain", b"again", json!({ "id": "", "reply": false }))));
    let an = a.file_name().unwrap().to_string_lossy().to_string();
    assert!(an.starts_with("my_notes__v2_-") && an.ends_with(".txt"), "{an}");
    assert_ne!(a, b);
    assert_eq!((std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap()), (b"hello".to_vec(), b"again".to_vec()));
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(std::fs::metadata(&a).unwrap().permissions().mode() & 0o777, 0o600, "readable only by you");
}

#[test]
fn a_file_for_the_coordinator_is_kept_in_its_own_folder() {
    let w = world();
    web(&w, "board", json!({}));
    let c = sw::webui::web::client("test");
    let mut g = c.lock().unwrap();
    g.s.data.can_dispatch = true;
    let out = sw::webui::attach::attach(&g, &file("shot.png", "image/png", PNG, json!({ "id": "", "reply": false }))).unwrap();
    let home = sw::coordinator::state::home(&sw::herdr::cli::current_socket());
    assert!(kept(&out).starts_with(home.join(".herdr-projects").join("files")), "{}", out.s("path"));
    // this machine's heading: its machine coordinator, in its own folder
    let out = sw::webui::attach::attach(&g, &file("shot.png", "image/png", PNG, json!({ "id": "head:machine\t", "reply": true, "scope": "" }))).unwrap();
    let home = sw::coordinator::state::home(sw::coordinator::co::Co::of(&sw::herdr::cli::current_socket(), "machine"));
    assert!(kept(&out).starts_with(home.join(".herdr-projects").join("files")), "{}", out.s("path"));
    // where no coordinator can run, a heading's gets none: its words would not be sent either
    g.s.data.can_dispatch = false;
    let err = sw::webui::attach::attach(&g, &file("shot.png", "image/png", PNG, json!({ "id": "head:machine\t", "reply": true }))).unwrap_err();
    assert!(err.starts_with("No coordinator can run here"), "{err}");
}

#[test]
fn a_file_for_a_tasks_agent_is_kept_in_its_working_folder_where_git_does_not_see_it() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Look at the picture");
    delegate(&id); settle(&id);
    let p = kept(&web(&w, "attach", file("", "image/jpeg", PNG, json!({ "id": id, "reply": true }))));
    let cwd = sw::store::repos::cwd_of(&task(&id));
    assert!(p.starts_with(cwd.join(".herdr-projects").join("files")) && p.extension().unwrap() == "jpg", "{}", p.display());
    let status = std::process::Command::new("git").args(["status", "--porcelain", "--untracked-files=all"]).current_dir(&cwd).output().unwrap();
    assert!(!String::from_utf8_lossy(&status.stdout).contains(".herdr-projects"), "{}", String::from_utf8_lossy(&status.stdout));
}

#[test]
fn no_file_for_a_machine_out_of_reach_a_task_that_is_gone_an_empty_or_too_big_one_or_a_device_that_only_watches() {
    let w = world();
    let err = |args: Value| web_as(&w, "test", "attach", args).unwrap_err();
    assert_eq!(err(file("a.png", "image/png", PNG, json!({ "id": "", "reply": false, "scope": "vm-a" }))), "vm-a is not among the machines you can reach now.");
    assert_eq!(err(file("a.png", "image/png", PNG, json!({ "id": "head:machine\tvm-a", "reply": true }))), "vm-a is not among the machines you can reach now.");
    assert_eq!(err(file("a.png", "image/png", PNG, json!({ "id": "proj/99", "reply": true }))), "The task those words are for is gone.");
    assert_eq!(err(file("a.png", "image/png", b"", json!({}))), "That file is empty.");
    let big = vec![0u8; sw::webui::attach::MOST + 1];
    assert!(err(file("big.png", "image/png", &big, json!({}))).starts_with("Too large to attach: big.png."));
    let mut watch = file("a.png", "image/png", PNG, json!({})); watch["role"] = json!("watch");
    assert_eq!(err(watch), "This device can only watch.");
    assert!(!sw::store::files::data_dir().join("files").exists() || std::fs::read_dir(sw::store::files::data_dir().join("files")).unwrap().count() == 0, "nothing was kept");
    // The front door lets a file through whole, as a change only a device that may drive makes.
    assert!(sw::webui::door::CHANGES.contains(&"attach"));
}

#[test]
fn the_box_attaches_as_the_web_uis_does_the_add_button_tiles_paste_drop_and_the_path_first_in_the_words() {
    // the add button in the lower left, the web UI's quiet round icon button, and its file picker
    assert!(FILES.contains(r#"h("button.icon-button.sw-attach", { type: "button", "aria-label": "Attach files", title: "Attach files" }, icon("plus"))"#));
    assert!(FILES.contains(r#"h("input", { type: "file", multiple: true, hidden: true })"#));
    assert!(COMPOSE.contains(r#"h("div.sw-card.sw-b-text", grip, files.strip, box, h("div.sw-controls.sw-controls-left", files.input, files.add, voice.el, call, to), h("div.sw-controls.sw-controls-right", also, send))"#));
    // a tile: a picture's thumbnail or the file's name, where it stands, a cross that takes it out (and its path)
    assert!(FILES.contains(r#"f.state === "uploading" ? "Uploading" : f.state === "error" ? "Failed" : "Attached""#));
    assert!(FILES.contains("if (m) box.value = box.value.replace(m, \"\");"));
    // a picture pasted, files dropped on the card
    assert!(FILES.contains(r#"filter((i) => i.kind === "file")"#) && FILES.contains(r#"card.addEventListener("drop""#) && COMPOSE.contains("files.dropOn(card);"));
    // the web UI's limits: 8 MB, three at a time, its own Files at once
    assert!(FILES.contains("const MOST = 8 * 1024 * 1024, AT_ONCE = 3") && FILES.contains(r#"localStorage.getItem("herdr-web-ui:settings") || "{}").attachLimit"#));
    // the path goes in at the start, as the web UI's `@path `, after a /command and the files picked before it
    assert!(FILES.contains("export const mention = (path) => `@${path} `;") && FILES.contains(r#"let at = /^\/\S+\s*/u.exec(text)?.[0].length ?? 0;"#));
    // sent to the board for whoever the words are for; attaching locks that as typing does; no send while one is on its way
    assert!(FILES.contains(r#"post("attach", { name: file.name || "", type: file.type || "", data: base64("#) && FILES.contains("to: whom() });"));
    assert!(CHAT.contains("holding = typing || files.count() > 0;") && CHAT.contains("if (!holding) to = null; else if (!to && now) to = { ...now };"));
    assert!(CHAT.contains("send.disabled = !typing || busy || files.busy();") && CHAT.contains("if (!text.trim() || busy || files.busy()) return;"));
    // a file attached while a message is on its way is not part of it: its path waits, and its tile stays for the next
    assert!(CHAT.contains("const sent = files.ids();") && CHAT.contains("files.clear(sent);") && CHAT.contains("hold: () => busy"));
    assert!(FILES.contains("const gone = ids ? list.filter((f) => ids.includes(f.id)) : list;"));
    // and while a lone /command is being typed, as the web UI's: the command stays first
    assert!(FILES.contains(r#"if (hold() || box.dataset.composing || /^\/\S*$/u.test(box.value)) return;"#));
    // the styles: the web UI's tile, the accent while files are over the card, a band when one did not go; loaded
    assert!(FILES_CSS.contains(".sw-file { position: relative;") && FILES_CSS.contains("calc(var(--control-h, 34px) * 2)"));
    assert!(FILES_CSS.contains(".sw-card[data-dragging] { border-color: var(--accent,") && FILES_CSS.contains(".sw-files[hidden] { display: none; }"));
    assert!(START.contains(r#""files""#) && sw::webui::page::file("files.js").is_some() && sw::webui::page::file("files.css").is_some());
}
