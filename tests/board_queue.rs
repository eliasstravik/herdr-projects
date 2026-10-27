//! Tasks that wait for a free slot: their own mark (◷, a clock) in a magenta no other state has, and their place in the queue (Q#1 gets the next
//! slot) the same on the row, the task panel, `list` and `details`, herdr's sidebar and the web Board; last under
//! `working`, in the order they start; and the places move up when a slot frees.
use crate::common;
use common::board::{open, web};
use common::*;
use serde_json::json;
use sw::json::*;
use sw::shown::ink::Ink;

fn line_of<'a>(lines: &'a [String], title: &str) -> (usize, &'a String) { lines.iter().enumerate().find(|(_, l)| l.contains(title)).unwrap_or_else(|| panic!("no row {title}:\n{}", lines.join("\n"))) }

#[test]
fn tasks_that_wait_for_a_free_slot_have_their_own_mark_and_place_in_the_queue_everywhere() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.agents": 1, "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "1.txt": "1\n" }, "commit": "1", "note": "One." }, { "status": "working" }] }));
    let (a, b, c) = (w.add("First at work"), w.add("Second waits"), w.add("Third waits"));
    delegate(&a); sw::clock::advance(1000); delegate(&b); sw::clock::advance(1000); delegate(&c);
    assert_eq!((task(&b).t("stage"), task(&c).t("stage")), ("queued".into(), "queued".into()));
    // The place is the order they get a slot: the first to wait first.
    assert_eq!((row(&b).i("queuePlace"), row(&c).i("queuePlace")), (1, 2));
    assert!(!row(&a).has("queuePlace"));
    // Their own mark, a clock, in a magenta of their own, and their place where a task at work has its percent.
    let r = sw::shown::states::as_row(&row(&c));
    assert_eq!(sw::shown::states::status_ink(&r), Ink::Queue);
    assert_eq!(sw::shown::row_says::row_fields(&r, sw::clock::now()).percent.text(), "Q#2");
    assert_eq!(sw::shown::states::standing(&r), "waiting for a free slot, Q#2");
    let mut bd = open(&w.root, 40);
    let lines = bd.screen(120);
    let (at_b, lb) = line_of(&lines, "Second waits");
    let (at_c, lc) = line_of(&lines, "Third waits");
    let (at_a, _) = line_of(&lines, "First at work");
    assert!(lb.contains('◷') && lb.trim_end().contains("Q#1") && lc.contains('◷') && lc.contains("Q#2"), "{lb}\n{lc}");
    assert!(at_a < at_b && at_b < at_c, "the waiting ones last under working, Q#1 first:\n{}", lines.join("\n"));
    assert!(lines[at_b + 1].contains("Waiting for a free slot"), "{}", lines[at_b + 1]);
    // A narrow board says the place with the time, at the end of the second line: the row stays two lines.
    assert!(bd.screen(44).iter().any(|l| l.contains("Waiting for a free slot") && l.contains("Q#2")), "{}", bd.screen(44).join("\n"));
    // The task panel's State line.
    let panel = sw::board::view::panel::panel(&mut bd.s, &bd.theme, &r, 100, false, 0).iter().map(|l| l.text()).collect::<Vec<_>>().join("\n");
    assert!(panel.contains("◷ waiting for a free slot, Q#2"), "{panel}");
    // `list` and `details`.
    let (_, said) = sw::cli::tasks::command("list", &sw::cli::flags::parse(&["list".to_string()])).unwrap();
    assert!(said.lines().any(|l| l.contains("Third waits") && l.contains("waiting for a free slot, Q#2")), "{said}");
    let d = sw::actions::finish_line::details(&c).unwrap();
    assert_eq!((d.at("queue").i("place"), d.at("queue").s("words")), (2, "waiting for a free slot, Q#2"));
    // herdr's sidebar: where it stands, then its place, without the row's second line.
    let e = sw::flow::sidebar_entry::entry_of(&row(&c), "", sw::clock::now());
    assert_eq!((e.place.as_str(), e.line.as_str(), e.progress.as_str()), ("queued", "", "Q#2"));
    // The web Board: the same mark, colour and place.
    let s = web(&w, "board", json!({}));
    let rows: Vec<Value> = s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").iter().cloned()).collect();
    let wc = rows.iter().find(|x| x.s("id") == c).unwrap();
    assert_eq!((wc.s("mark"), wc.s("color"), wc.s("percent")), ("◷", "queue", "Q#2"));
    // A slot frees: the first in the queue starts, and the one after it moves up.
    settle(&a);
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    assert_ne!(task(&b).s("stage"), "queued");
    assert_eq!(row(&c).i("queuePlace"), 1);
    assert!(!row(&b).has("queuePlace"));
}

#[test]
fn the_queue_colour_is_a_magenta_of_its_own_in_every_theme() {
    use sw::board::theme::picks::{is_magenta, QUEUE_APART, QUEUE_BESIDE};
    use sw::board::theme::{greys::{apart, contrast, LEAST}, resolve, Seen};
    for name in sw::herdr::palette::THEME_NAMES {
        let cfg: toml::Table = format!("[theme]\nname = \"{name}\"\n").parse().unwrap();
        let r = resolve(&cfg, None, "herdr", Seen::default());
        let q = r.pal.queue;
        assert!(is_magenta(q), "{name}: {q:?} is a magenta");
        for k in QUEUE_BESIDE { assert!(apart(q, r.pal.get(k)) >= QUEUE_APART, "{name}: queue and {k} are {} apart", apart(q, r.pal.get(k)).round()); }
        if let (Some(x), Some(g)) = (q.real(), r.ground.real()) { assert!(contrast(x, g) >= LEAST, "{name}: the magenta is seen on the ground"); }
    }
    // With the terminal's own colours it is still one of the board's magentas: the sixteen's own magenta is the automations'.
    let t = resolve(&"[theme]\nname = \"terminal\"\n".parse().unwrap(), None, "herdr", Seen::default()).pal;
    assert!(is_magenta(t.queue) && t.queue != t.auto);
    // As docs/colours.md has them: herdr's default and the terminal's own colours.
    assert_eq!((resolve(&toml::Table::new(), None, "herdr", Seen::default()).pal.queue, t.queue), (sw::board::theme::C::Rgb(0xe056fd), sw::board::theme::C::Rgb(0xd946ef)));
}

fn resource_reading() -> sw::admission::Reading {
    sw::admission::Reading { schema: 1, sampled_at: sw::clock::now(), total_bytes: 40 << 30,
        available_bytes: 10 << 30, cores: 14, load1: 8.0, load5: 8.0, pressure: 2,
        free_percent: 45.0, swap_growth_bytes_per_second: 0.0, swapout_bytes_per_second: 0.0 }
}
fn resource_gate() {
    sw::store::settings::change::set_value("board.admission", &json!("/host/probe"), &sw::store::settings::lookup::Scope::you()).unwrap();
}

#[test]
fn resource_admission_serializes_concurrent_claims_and_keeps_pending_reservations() {
    let w = world(); resource_gate();
    let a = task(&w.add("First claim")); let b = task(&w.add("Second claim"));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let threads: Vec<_> = [a.clone(), b.clone()].into_iter().map(|t| {
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            sw::admission::with_probe(std::rc::Rc::new(|| Ok(resource_reading())), || {
                barrier.wait(); sw::admission::run(&t, "worker", || Ok(())).is_ok()
            })
        })
    }).collect();
    let admitted = threads.into_iter().map(|t| t.join().unwrap()).filter(|ok| *ok).count();
    assert_eq!(admitted, 1, "one memory budget must not be spent by two callers");
    sw::admission::with_probe(std::rc::Rc::new(|| Ok(resource_reading())), || {
        let ledger = sw::store::files::read_json(&sw::store::files::data_dir().join("admission.json")).unwrap();
        assert_eq!(ledger.at("reservations").as_object().unwrap().len(), 1);
        let pending = ledger.at("reservations").as_object().unwrap().values().next().unwrap();
        let other = if pending.s("task") == a.s("id") { &b } else { &a };
        assert!(sw::admission::run(other, "worker", || Ok(())).is_err(), "another invocation cannot forget a pending start");
    });
}

#[test]
fn resource_admission_rechecks_between_claim_and_actual_program_start() {
    let w = world(); resource_gate();
    let count = std::rc::Rc::new(std::cell::Cell::new(0)); let seen = count.clone();
    sw::admission::with_probe(std::rc::Rc::new(move || {
        let n = seen.get(); seen.set(n + 1); let mut r = resource_reading();
        if n > 0 { r.available_bytes = 1; } Ok(r)
    }), || {
        w.script(json!({ "worker": [{ "status": "working" }] }));
        let id = w.add("Resource collapse before launch"); delegate(&id);
        let t = settle(&id);
        assert_eq!(t.s("stage"), "queued");
        assert!(t.at("resourceStart").b("prompt"), "hold retains the exact launch text");
        assert!(!calls(&w).iter().any(|c| c.starts_with("agent start ")), "the allocated slot alone must not authorize a program start");
    });
    assert!(count.get() >= 2);
}

#[test]
fn resource_admission_holds_wake_prompts_without_losing_the_inbox_text() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Wake later"); delegate(&id); let t = settle(&id);
    resource_gate();
    let before = calls(&w).len();
    sw::admission::with_probe(std::rc::Rc::new(|| { let mut r = resource_reading(); r.available_bytes = 1; Ok(r) }), || {
        assert_eq!(sw::flow::send::deliver(&t, "Keep this exact wake", None, false).unwrap(), "queued");
    });
    let pending = sw::store::inbox::pending_text(&id);
    assert_eq!(pending.last().unwrap().s("text"), "Keep this exact wake");
    assert!(!calls(&w)[before..].iter().any(|c| c.starts_with("agent prompt ")));
}

#[test]
fn resource_admission_checks_load_growth_staleness_and_unknown_readings() {
    let w = world(); resource_gate(); let t = task(&w.add("Safe refusal"));
    for change in 0..6 {
        sw::admission::with_probe(std::rc::Rc::new(move || {
            if change == 5 { return Err("timeout".into()); }
            let mut r = resource_reading();
            match change { 0 => r.load5 = 30.0, 1 => r.pressure = 4, 2 => r.swapout_bytes_per_second = 2_000_000.0,
                3 => r.sampled_at -= 3000, _ => r.load1 = f64::NAN }
            Ok(r)
        }), || assert!(sw::admission::run(&t, "worker", || -> Result<(), sw::herdr::cli::HerdrError> { panic!("unsafe start") }).is_err()));
    }
}

#[test]
fn resource_admission_releases_only_after_work_is_observed_and_snapshot_is_fresh() {
    let w = world(); resource_gate();
    sw::clock::set(Some(100000));
    let a = task(&w.add("Pending")); let b = task(&w.add("Next"));
    sw::admission::with_probe(std::rc::Rc::new(|| Ok(resource_reading())), || {
        sw::admission::run(&a, "worker", || Ok(())).unwrap();
        sw::clock::advance(60000);
        assert!(sw::admission::run(&b, "worker", || Ok(())).is_err(), "time alone is not proof of a started agent");
        sw::store::tasks::update(a.s("id"), |t| { t.put("workedAt", sw::clock::now()); Ok(sw::store::tasks::Change::Write) }).unwrap();
        sw::admission::run(&b, "worker", || Ok(())).unwrap();
    });
}

#[test]
fn resource_admission_corrupt_reservation_state_holds_instead_of_forgetting_claims() {
    let w = world(); resource_gate(); let t = task(&w.add("Unknown claims"));
    std::fs::write(sw::store::files::data_dir().join("admission.json"), "not-json").unwrap();
    sw::admission::with_probe(std::rc::Rc::new(|| Ok(resource_reading())), || {
        assert!(sw::admission::run(&t, "worker", || -> Result<(), sw::herdr::cli::HerdrError> { panic!("unknown reservations") }).is_err());
    });
}

#[test]
fn real_programs_resource_probe_is_bounded_and_rejects_failures_without_launching_agents() {
    use std::os::unix::fs::PermissionsExt;
    let w = world(); let t = task(&w.add("Native probe"));
    let path = w.base.join("host-reading");
    sw::store::settings::change::set_value("board.admission", &json!(path.display().to_string()), &sw::store::settings::lookup::Scope::you()).unwrap();
    let write = |body: &str| {
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    };
    write(&format!("printf '%s' '{}'", serde_json::to_string(&resource_reading()).unwrap()));
    sw::admission::run(&t, "worker", || Ok(())).unwrap();
    for body in ["exit 1", "printf not-json", "exec /usr/bin/yes", "exec /bin/sleep 30"] {
        write(body);
        let error = sw::admission::run(&t, "worker", || -> Result<(), sw::herdr::cli::HerdrError> { panic!("probe failure launched work") }).unwrap_err();
        assert_eq!(error.code, "admission_hold");
    }
    assert!(!std::fs::read_dir(sw::store::files::data_dir()).unwrap().filter_map(Result::ok).any(|e| e.file_name().to_string_lossy().starts_with("admission-probe-")));
}

#[test]
fn resource_admission_unknown_or_ambiguous_panes_hold_and_coordinator_exemption_is_scoped() {
    let w = world(); resource_gate();
    let denied = || -> Result<(), sw::herdr::cli::HerdrError> { panic!("unknown binding started work") };
    assert!(sw::admission::pane("unknown", denied).is_err());
    sw::admission::coordinator("coordinator", || {
        assert!(sw::admission::pane("coordinator", || Ok(())).is_ok());
        assert!(sw::admission::pane("other", denied).is_err());
    });
    assert!(sw::admission::pane("coordinator", denied).is_err());
    for name in ["First binding", "Duplicate binding"] {
        let id = w.add(name);
        sw::store::tasks::update(&id, |t| { t.obj_mut("link").put("pane", "shared"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    }
    assert!(sw::admission::pane("shared", denied).unwrap_err().message.contains("Ambiguous"));
}

#[test]
fn resource_admission_failed_claims_release_but_unknown_launches_keep_their_budget() {
    let w = world(); resource_gate(); sw::clock::set(Some(100000));
    let a = task(&w.add("Setup fails")); let b = task(&w.add("Next claim"));
    sw::admission::with_probe(std::rc::Rc::new(|| Ok(resource_reading())), || {
        sw::admission::claim(&a, "worker", || Ok(())).unwrap();
        sw::store::tasks::update(a.s("id"), |t| { t.put("stage", "failed"); Ok(sw::store::tasks::Change::Write) }).unwrap();
        sw::admission::claim(&b, "worker", || Ok(())).unwrap();
        sw::store::tasks::update(b.s("id"), |t| { t.put("stage", "done"); t.put("turnOpen", false); Ok(sw::store::tasks::Change::Write) }).unwrap();
        sw::admission::run(&a, "worker", || Ok(())).unwrap();
        sw::clock::advance(60000);
        assert!(sw::admission::claim(&b, "worker", || Ok(())).is_err(), "a failed task with missing turn data is not proof its launch stopped");
        sw::store::tasks::update(a.s("id"), |t| {
            t.put("turnOpen", false); t.put("last", json!({ "status": "done", "at": sw::clock::now() }));
            Ok(sw::store::tasks::Change::Write)
        }).unwrap();
        sw::admission::claim(&b, "worker", || Ok(())).unwrap();
        sw::clock::advance(1);
        sw::store::tasks::update(b.s("id"), |t| {
            t.put("stage", "starting"); t.put("last", json!({ "status": "done", "at": sw::clock::now() }));
            Ok(sw::store::tasks::Change::Write)
        }).unwrap();
        sw::clock::advance(1000);
        sw::admission::run(&task(b.s("id")), "worker", || Ok(())).unwrap();
        sw::store::tasks::update(b.s("id"), |t| { t.put("stage", "failed"); Ok(sw::store::tasks::Change::Write) }).unwrap();
        assert!(sw::admission::claim(&a, "worker", || Ok(())).is_err(), "an idle observation before the launch attempt cannot release the new launch");
    });
}
