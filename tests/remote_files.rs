//! Files that go with words to another machine: a file of this machine the words name is sent over the board's SSH
//! link first (`remote keep`, the file on its stdin), and the words name it where it landed there; a file that cannot
//! go stops the words, and says why. Also that machine's side: where its door keeps the file.
use crate::common;
use crate::far;
use common::*;
use far::Far;
use serde_json::{json, Value};
use sw::board::job_far::{dispatch, far as far_do};
use sw::json::*;
use sw::remote::Machine;

fn vm_a() -> Machine { Machine { label: "vm-a".into(), target: "me@vm-a".into(), ..Default::default() } }
/// What vm-a was asked, in order, without the machine: the request and what came with it.
fn asked(far: &Far) -> Vec<(String, Value)> { far.asked().into_iter().map(|(_, what, arg)| (what, arg)).collect() }
/// A file of this machine, in a folder of the test's own.
fn file(w: &World, name: &str, bytes: &[u8]) -> String {
    let dir = w.base.join("desk");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(name), bytes).unwrap();
    dir.join(name).display().to_string()
}
fn tell(text: &str) -> sw::board::job::Done { far_do(&vm_a(), "vm-a", "tell", json!({ "id": "far/1", "text": text }), false) }
fn said(d: &sw::board::job::Done) -> String { d.flash.as_ref().map(|f| f.0.clone()).unwrap_or_default() }

#[test]
fn a_reply_to_another_machines_agent_takes_the_file_it_names_along_and_names_it_where_it_landed() {
    let w = world();
    let far = Far::new(&w.base);
    let shot = file(&w, "shot.png", b"\x89PNG picture");
    far.answers("vm-a", "keep", &json!({ "ok": true, "path": "/home/me/proj/.herdr-projects/files/paste-1.png" }));
    far.answers("vm-a", "do", &json!({ "ok": true, "msg": "sent" }));
    let d = tell(&format!("look at @{shot} and @{shot} please"));
    assert!(d.ok, "{}", said(&d));
    let a = asked(&far);
    assert_eq!(a.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["keep", "do"], "named twice, sent once: {a:?}");
    assert_eq!(a[0].1, json!({ "name": "shot.png", "kind": "image/png", "size": 12, "id": "far/1" }));
    assert_eq!(far.fed("vm-a", "keep"), b"\x89PNG picture");
    assert_eq!(a[1].1.s("text"), "look at @/home/me/proj/.herdr-projects/files/paste-1.png and @/home/me/proj/.herdr-projects/files/paste-1.png please");
}

#[test]
fn a_dropped_path_goes_too_bare_escaped_or_quoted_and_a_path_that_is_no_file_here_is_left_as_it_is() {
    let w = world();
    let far = Far::new(&w.base);
    let shot = file(&w, "Screen Shot.png", b"pic");
    let notes = file(&w, "notes.txt", b"words");
    let dir = w.base.join("desk").display().to_string();
    far.answers("vm-a", "keep", &json!({ "ok": true, "path": "/far/kept-1.png" }));
    far.answers("vm-a", "do", &json!({ "ok": true, "msg": "sent" }));
    // A terminal drop: `\ ` for a space; a `.` after a path is not its own.
    let escaped = shot.replace(' ', "\\ ");
    assert!(tell(&format!("see {escaped} and {notes}.")).ok);
    let a = asked(&far);
    assert_eq!(a.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["keep", "keep", "do"], "{a:?}");
    assert_eq!((a[0].1.s("name"), a[0].1.s("kind")), ("Screen Shot.png", "image/png"));
    assert_eq!((a[1].1.s("name"), a[1].1.s("kind"), a[1].1.at("size")), ("notes.txt", "", &json!(5)));
    assert_eq!(a[2].1.s("text"), "see /far/kept-1.png and /far/kept-1.png.");
    // In quotes.
    std::fs::remove_file(far.dir.join("asked")).unwrap();
    assert!(tell(&format!("see '{shot}' now")).ok);
    assert_eq!(asked(&far)[1].1.s("text"), "see /far/kept-1.png now");
    // A folder, a missing file, `/` alone, a system file both machines have: no file goes, the words go as typed.
    std::fs::remove_file(far.dir.join("asked")).unwrap();
    let words = format!("in {dir} and @{dir}/gone.png, / and /etc/hosts");
    assert!(tell(&words).ok);
    let a = asked(&far);
    assert_eq!(a.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["do"], "{a:?}");
    assert_eq!(a[0].1.s("text"), words);
}

#[test]
fn a_picture_herdr_pasted_goes_too_and_the_words_name_it_where_it_landed() {
    // The board's box sends `[Image 1]` as the path herdr pasted: a file of this machine, in herdr's own temp folder.
    let w = world();
    let far = Far::new(&w.base);
    let dir = w.base.join("T").join("herdr-clipboard-images-501");
    std::fs::create_dir_all(&dir).unwrap();
    let shot = dir.join("client-327-clipboard-1792608324073248000-0.png");
    std::fs::write(&shot, b"\x89PNG").unwrap();
    far.answers("vm-a", "keep", &json!({ "ok": true, "path": "/home/me/proj/.herdr-projects/files/paste-1.png" }));
    far.answers("vm-a", "do", &json!({ "ok": true, "msg": "sent" }));
    assert!(tell(&format!("what is this {} ", shot.display())).ok);
    let a = asked(&far);
    assert_eq!(a.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["keep", "do"], "{a:?}");
    assert_eq!(a[0].1.s("kind"), "image/png");
    assert_eq!(a[1].1.s("text"), "what is this /home/me/proj/.herdr-projects/files/paste-1.png ");
}

#[test]
fn a_file_that_cannot_go_stops_the_words_and_says_why() {
    let w = world();
    let far = Far::new(&w.base);
    let shot = file(&w, "shot.png", b"pic");
    // The other machine refused it.
    far.answers("vm-a", "keep", &json!({ "ok": false, "msg": "far/1 is not a task here" }));
    let d = tell(&format!("look @{shot}"));
    assert!(!d.ok);
    assert_eq!(said(&d), "shot.png could not be sent to vm-a: far/1 is not a task here. Nothing was sent.");
    assert!(!asked(&far).iter().any(|x| x.0 == "do"), "no words went");
    // An older herdr-projects there, which does not know the request.
    far.answers("vm-a", "keep", &json!({ "ok": false, "msg": "Unknown request keep" }));
    assert_eq!(said(&tell(&format!("look @{shot}"))), "shot.png could not be sent to vm-a: herdr-projects on vm-a is too old to take files: update it there. Nothing was sent.");
    // It cannot be reached.
    far.ends("vm-a", "keep", "", "ssh: connect to host vm-a port 22: Connection refused\n", 255);
    assert_eq!(said(&tell(&format!("look @{shot}"))), "shot.png could not be sent to vm-a: vm-a did not answer: cannot connect (it refused the connection). Nothing was sent.");
    // Too big: never sent at all.
    std::fs::remove_file(far.dir.join("asked")).unwrap();
    let big = file(&w, "big.mov", &vec![0u8; sw::store::attached::MOST + 1]);
    assert_eq!(said(&tell(&format!("look @{big}"))), "Too large to send to vm-a: big.mov. A file can be up to 8 MB. Nothing was sent.");
    assert!(asked(&far).is_empty());
    // The words you typed are never lost: the board puts them back, as you typed them.
    let text = format!("look @{shot}");
    let d = sw::board::job::run(sw::board::job::Job::Far { machine: vm_a(), host: "vm-a".into(), action: "tell".into(), req: json!({ "id": "far/1", "text": text }), quiet: false });
    assert!(d.then.iter().any(|t| matches!(t, sw::board::job::Then::Unsent { text: x, to } if *x == text && to == "vm-a:far/1")), "{:?}", d.then);
}

#[test]
fn words_for_another_machines_coordinator_take_their_files_along_and_a_file_that_cannot_go_leaves_the_row_saying_why() {
    let w = world();
    let far = Far::new(&w.base);
    let shot = file(&w, "shot.jpeg", b"jpeg");
    far.answers("vm-a", "keep", &json!({ "ok": true, "path": "/far/coord/paste-2.jpg" }));
    far.answers("vm-a", "dispatch", &json!({ "ok": true, "msg": "sent to the vm-a coordinator" }));
    dispatch(vm_a(), format!("what is this? @{shot} "), false, String::new(), "machine".into());
    let a = asked(&far);
    assert_eq!(a.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["keep", "dispatch"], "{a:?}");
    assert_eq!(a[0].1, json!({ "name": "shot.jpeg", "kind": "image/jpeg", "size": 4, "to": "machine" }));
    assert_eq!((a[1].1.s("text"), a[1].1.s("to")), ("what is this? @/far/coord/paste-2.jpg ", "machine"));
    assert!(sw::store::inbox::get_dispatch("dispatch/1").is_none(), "taken: its row goes");
    // Its top-level Coordinator, and the file refused: the row stays, red, and says why; the words never went.
    std::fs::remove_file(far.dir.join("asked")).unwrap();
    far.answers("vm-a", "keep", &json!({ "ok": false, "msg": "The disk is full" }));
    dispatch(vm_a(), format!("and this @{shot}"), false, String::new(), String::new());
    let a = asked(&far);
    assert_eq!(a.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["keep"], "{a:?}");
    assert_eq!(a[0].1.at("to"), &json!(""));
    let row = sw::store::inbox::get_dispatch("dispatch/2").expect("its row stays");
    assert_eq!((row.s("state"), row.s("msg")), ("error", "shot.jpeg could not be sent to vm-a: The disk is full. Nothing was sent."));
}

#[test]
fn the_door_keeps_a_file_where_its_reader_reads() {
    let w = world();
    let id = w.add("Theirs");
    let meta = |extra: Value| { let mut m = json!({ "name": "shot.png", "kind": "image/png", "size": 3 }); for (k, v) in extra.as_object().unwrap() { m[k] = v.clone(); } m };
    // For a task's agent: in its working folder.
    let a = sw::remote::door_with("keep", &meta(json!({ "id": id })), b"pic").unwrap();
    let p = std::path::PathBuf::from(a.s("path"));
    assert!(a.b("ok") && p.starts_with(sw::store::repos::cwd_of(&task(&id)).join(".herdr-projects/files")), "{a}");
    assert!(p.file_name().unwrap().to_string_lossy().starts_with("paste-") && p.extension().unwrap() == "png");
    assert_eq!(std::fs::read(&p).unwrap(), b"pic");
    // For its Coordinator when none can run here (the words become a task): the board's own folder.
    let a = sw::remote::door_with("keep", &meta(json!({ "to": "" })), b"pic").unwrap();
    assert!(a.s("path").starts_with(&sw::store::files::data_dir().join("files").display().to_string()), "{a}");
    // Cut short, too big, no such task: nothing kept, and why.
    let no = |m: Value, b: &[u8]| sw::remote::door_with("keep", &m, b).unwrap_err().msg;
    assert_eq!(no(meta(json!({ "id": id })), b"pi"), "shot.png did not arrive whole (2 of 3 bytes).");
    assert!(no(meta(json!({ "id": "nope/9" })), b"pic").contains("nope/9"));
    assert!(no(meta(json!({ "to": "machine" })), b"pic").contains("has no coordinators"));
    // Through the door as another machine asks it, without the bytes: refused, never an empty file.
    assert_eq!(sw::remote::door("keep", &meta(json!({ "id": id }))).unwrap_err().msg, "shot.png did not arrive whole (0 of 3 bytes).");
}

#[test]
fn the_door_keeps_a_file_for_a_coordinator_that_runs_in_its_own_folder() {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    let a = sw::remote::door_with("keep", &json!({ "name": "notes.txt", "kind": "", "size": 2, "to": "" }), b"hi").unwrap();
    unsafe { std::env::remove_var("HERDR_PROJECTS_COORDINATOR_DIR") };
    let p = std::path::PathBuf::from(a.s("path"));
    assert!(p.starts_with(home.join(".herdr-projects/files")), "{a}");
    assert!(p.file_name().unwrap().to_string_lossy().starts_with("notes-") && p.extension().unwrap() == "txt");
    assert_eq!(read(&home.join(".herdr-projects/files/.gitignore")), "*\n");
}
