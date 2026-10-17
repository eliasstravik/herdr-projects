//! Saved board views keep exact repo scope and independent preferences in one task store.
use crate::common;
use sw::json::*;

#[test]
fn board_views_scope_is_exact_and_counts_agree() {
    let w = common::world();
    for repo in ["alpha", "alpha-web", "beta"] {
        let dir = w.base.join(repo);
        std::fs::create_dir_all(&dir).unwrap();
        common::git(&dir, &["init", "-q"]);
        sw::actions::filing::add(&dir, &format!("{repo} task"), Default::default()).unwrap();
    }
    let mut b = common::board::open(&w.root, 40);
    assert_eq!(b.s.items.len(), 3);
    sw::board::run::prefs::apply(&mut b.s, &obj(json!({"repos": ["alpha"]})));
    sw::board::update::list::relist(&mut b.s);
    assert_eq!(b.s.items.len(), 1, "repo scope must exclude alpha-web and beta");
    assert_eq!(b.s.items[0].s("id").split('/').next(), Some("alpha"));
    assert_eq!(sw::board::update::list::counts(&b.s).values().sum::<usize>(), 1);
    assert!(b.text().contains("only repo alpha"));
}

fn options(view: &str, repos: Option<&str>) -> sw::board::saved_views::View {
    let mut flags = obj(json!({"view": view}));
    if let Some(repos) = repos { flags.put("repo", repos); }
    sw::board::saved_views::View::parse(&flags).unwrap()
}
fn named(w: &common::World, view: &str, repos: Option<&str>) -> common::board::Board {
    let mut b = common::board::open(&w.root, 40);
    b.s = sw::board::run::start::view_with(b.s.ctx.clone(), &w.root, &options(view, repos));
    b.load();
    b
}
fn repo(w: &common::World, name: &str) -> std::path::PathBuf {
    let dir = w.base.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    common::git(&dir, &["init", "-q"]);
    for title in ["first", "second"] {
        sw::actions::filing::add(&dir, &format!("{name} {title}"), Default::default()).unwrap();
    }
    dir
}
#[test]
fn board_views_independent_filter_selection_and_reopen() {
    let w = common::world();
    repo(&w, "alpha"); repo(&w, "beta");
    let mut global = common::board::open(&w.root, 40);
    let mut a = named(&w, "alpha", Some("alpha"));
    let mut b = named(&w, "beta", Some("beta"));
    let before = (global.s.sel.clone(), b.s.sel.clone());
    a.press(&["ctrl+f"]); a.typed("second"); a.press(&["enter"]);
    b.press(&["down"]);
    assert_eq!(global.s.sel, before.0);
    assert_eq!(global.s.filter, ""); assert_eq!(b.s.filter, "");
    assert_ne!(b.s.sel, before.1);
    assert_eq!(a.s.filter, "second");
    for s in [&global.s, &a.s, &b.s] { sw::board::run::prefs::save(s); }
    let selected = a.s.sel.clone();
    let again = named(&w, "alpha", None);
    assert_eq!(again.s.repos, vec!["alpha"]);
    assert_eq!(again.s.filter, "second"); assert_eq!(again.s.sel, selected);
    assert_eq!(again.s.items.len(), 2);
    use sw::board::run::prefs::{file, view_file};
    assert_ne!(std::fs::read(file(false)).unwrap(), std::fs::read(view_file(false, "alpha")).unwrap());
    assert_ne!(std::fs::read(view_file(false, "alpha")).unwrap(), std::fs::read(view_file(false, "beta")).unwrap());
    assert_eq!(global.s.items.len(), 4);
    assert!(!global.text().contains("only repo"));
}
#[test]
fn board_views_default_files_stay_compatible_and_popup_names_do_not_collide() {
    let w = common::world(); let id = w.add("legacy");
    use sw::board::run::prefs::*;
    sw::store::files::write_json(&file(false), &json!({"sel": id, "filter": "legacy", "byRepo": true})).unwrap();
    let b = common::board::open(&w.root, 40);
    assert_eq!(b.s.sel, id); assert_eq!(b.s.filter, "legacy"); assert!(b.s.by_repo);
    assert!(b.s.repos.is_empty()); save(&b.s);
    let p = sw::store::files::read_obj(&file(false)).unwrap(); assert!(!p.has("repos"));
    assert!(!view_file(false, "repo-proj").exists());
    assert_eq!(view_file(false, ""), file(false));
    assert_eq!(view_file(true, ""), file(true));
    assert_ne!(view_file(true, "alpha"), view_file(false, "alpha-popup"));
    assert_ne!(view_file(false, "popup"), file(true));
    let a = named(&w, "alpha", Some("proj")); save(&a.s);
    let pop = read_view(true, "alpha"); assert_eq!(pop.arr("repos"), &vec![json!("proj")]);
    assert!(!pop.has("draft"));
}
#[test]
fn board_views_multi_repo_unknown_and_new_task_destination() {
    let w = common::world(); repo(&w, "alpha"); repo(&w, "alpha-web"); repo(&w, "beta");
    let mut b = named(&w, "both", Some("alpha,beta,alpha"));
    assert_eq!(b.s.repos, vec!["alpha", "beta"]); assert_eq!(b.s.items.len(), 4);
    b.typed("new scoped task"); b.press(&["enter"]);
    let tasks = sw::store::tasks::list(Default::default());
    let new = tasks.iter().find(|t| t.s("title") == "new scoped task").unwrap();
    assert!(new.s("id").starts_with("alpha/"));
    let mut unknown = named(&w, "missing", Some("unknown"));
    assert!(unknown.s.items.is_empty()); assert!(unknown.text().contains("Unknown repo unknown"));
    unknown.typed("must not be added"); unknown.press(&["enter"]);
    assert_eq!(sw::store::tasks::list(Default::default()).len(), tasks.len());
    let global = common::board::open(&w.root, 40); assert_eq!(global.s.items.len(), tasks.len());
}
#[test]
fn board_views_archive_and_automations_respect_scope() {
    let w = common::world(); let alpha = repo(&w, "alpha"); repo(&w, "beta");
    sw::actions::archive::archive("alpha/1").unwrap(); sw::actions::archive::archive("beta/1").unwrap();
    use sw::automations::list::{add, Add};
    for name in ["alpha", "beta"] {
        add(Add { name, prompt: "synthetic audit", trigger: Some(sw::automations::triggers::parse_schedule("every 2h").unwrap()), repo: name, folder: &alpha, settings: Obj::new(), research: None }).unwrap();
    }
    let mut b = named(&w, "alpha", Some("alpha"));
    assert_eq!(b.s.autos.len(), 1); assert_eq!(b.s.autos[0].s("repo"), "alpha");
    b.s.archive_view = true; sw::board::update::list::relist(&mut b.s);
    assert!(b.s.autos.is_empty());
    assert_eq!(b.s.grouped.iter().flat_map(|(_, rows)| rows).count(), 1);
    assert_eq!(b.s.grouped[0].1[0].s("id"), "alpha/1");
    b.s.archive_view = false; b.s.by_repo = true; sw::board::update::list::relist(&mut b.s);
    assert_eq!(b.s.grouped.len(), 1); assert_eq!(b.s.grouped[0].0, "alpha");
}
#[test]
fn board_views_cli_flags_and_name_validation() {
    use sw::cli::flags::{parse, refuse_unknown};
    for args in [vec!["board", "--repo", "alpha"], vec!["--view", "work", "--repo", "alpha,beta"], vec!["board", "--view", "work"]] {
        let a = parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert!(refuse_unknown("board", &a).is_ok());
        assert!(sw::board::saved_views::View::parse(&a.flags).is_ok());
    }
    for name in ["../x", "", "Upper", "a/b"] {
        assert!(sw::board::saved_views::View::parse(&obj(json!({"view": name}))).is_err());
    }
    let a = parse(&["board".into(), "--unknown".into()]); assert!(refuse_unknown("board", &a).is_err());
    assert_eq!(sw::board::saved_views::View::parse(&obj(json!({"repo": "alpha"}))).unwrap().name, "repo-alpha");
    assert!(sw::board::saved_views::View::parse(&obj(json!({"repo": true}))).is_err());
}
#[test]
fn board_views_remote_rows_use_the_original_exact_repo_name() {
    let w = common::world(); repo(&w, "alpha");
    let mut b = named(&w, "alpha", Some("alpha"));
    use sw::board::{data::Remote, update::remotes};
    b.s.remotes.insert("remote".into(), Remote { machine: sw::remote::Machine { label: "remote".into(), ..Default::default() }, ..Default::default() });
    b.s.scope = "remote".into();
    remotes::answered(&mut b.s, "remote", Ok(json!({"tasks": [
        {"id": "alpha/1", "title": "remote alpha", "stage": "todo"},
        {"id": "alpha-web/1", "title": "remote alpha-web", "stage": "todo"}
    ]})));
    assert_eq!(b.s.items.len(), 1); assert_eq!(b.s.items[0].s("rid"), "alpha/1");
    b.s.scope = "*".into(); sw::board::update::list::relist(&mut b.s);
    assert_eq!(b.s.items.len(), 3);
}
#[test]
fn board_views_scoped_board_keeps_all_five_status_groups() {
    let w = common::world(); repo(&w, "alpha"); repo(&w, "beta");
    let states = [("1", json!({"state": "blocked", "kind": "question"})), ("2", json!({"state": "blocked", "kind": "permission", "stage": "approval"})),
        ("3", json!({"state": "working"})), ("4", json!({"state": "todo"})), ("5", json!({"state": "done"}))];
    for name in ["alpha", "beta"] {
        for (n, _) in &states { if *n != "1" && *n != "2" { sw::actions::filing::add(&w.base.join(name), &format!("{name} extra {n}"), Default::default()).unwrap(); } }
    }
    let mut b = named(&w, "alpha", Some("alpha"));
    for r in b.s.data.items.iter_mut() {
        let at = r.s("id").split('/').nth(1).and_then(|n| n.parse::<usize>().ok()).unwrap_or(0);
        if let Some((_, patch)) = states.get(at.saturating_sub(1)) { let o = std::sync::Arc::make_mut(r); for (k, v) in patch.as_object().unwrap() { o.put(k, v.clone()); } }
    }
    sw::board::update::list::relist(&mut b.s);
    for g in sw::shown::states::GROUPS {
        let rows = &b.s.grouped.iter().find(|(name, _)| name == g).unwrap_or_else(|| panic!("group {g} dropped")).1;
        assert!(!rows.is_empty(), "group {g} is empty");
        assert!(rows.iter().all(|r| r.s("id").starts_with("alpha/")), "{g} holds a task of another repo");
    }
    let head: String = sw::board::view::list::head_lines(&b.s, 200).iter().map(|l| l.text()).collect::<Vec<_>>().join(" ");
    for want in ["needs you", "ready", "working", "to do", "done"] { assert!(head.contains(want), "header lacks {want}: {head}"); }
}
