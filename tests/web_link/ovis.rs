//! The web UI found under its new name, Ovis (`devswha.ovis`), and under its name before (`devswha.herdr-web-ui`):
//! which of the two herdr lists wins, the port a start of it settled on read from the state folder of the id it was
//! found under, and what `herdr-projects web-ui` says about it (its lines about links ask Tailscale: web_link.rs).
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::Path;
use std::rc::Rc;
use sw::json::*;
use sw::webui::{command as cmd, find::{address_from, web_ui, WebUi, OLD, OVIS}};

/// herdr with these plugins in its list, each with a config folder of its own (`<base>/cfg/<id>`).
struct Listing { inner: Rc<common::herdr::FakeHerdr>, plugins: std::cell::RefCell<Vec<Value>>, base: String }
impl sw::herdr::cli::Fake for Listing {
    fn run(&self, args: &[String], input: &str) -> sw::herdr::cli::Ran {
        if args.first().map(|a| a.as_str()) != Some("plugin") { return self.inner.run(args, input); }
        let out = match args.get(1).map(|a| a.as_str()) {
            Some("list") => json!({ "result": { "plugins": self.plugins.borrow().clone() } }).to_string(),
            _ => format!("{}/cfg/{}\n", self.base, args.get(2).cloned().unwrap_or_default()),
        };
        sw::herdr::cli::Ran { status: 0, stdout: out, stderr: String::new() }
    }
}
fn plugin(id: &str, version: &str, enabled: bool) -> Value { json!({ "plugin_id": id, "enabled": enabled, "version": version, "plugin_root": "/p" }) }
fn env(k: &str, v: Option<&str>) { match v { Some(v) => unsafe { std::env::set_var(k, v) }, None => unsafe { std::env::remove_var(k) } } }
fn port_in(dir: &Path, port: u16) { std::fs::create_dir_all(dir).unwrap(); std::fs::write(dir.join("plugin-port"), format!("{port}\n")).unwrap(); }

#[test]
fn ovis_is_found_under_its_new_id_the_old_id_still_is_and_where_herdr_lists_both_ovis_wins() {
    let w = world();
    let base = w.base.display().to_string();
    let herdr = Rc::new(Listing { inner: w.fake.clone(), plugins: std::cell::RefCell::new(vec![json!({ "plugin_id": "other" }), plugin(OVIS, "0.4.14", true)]), base: base.clone() });
    sw::herdr::cli::set_fake(Some(herdr.clone()));
    for id in [OVIS, OLD] { std::fs::create_dir_all(w.base.join("cfg").join(id)).unwrap(); }
    std::fs::write(w.base.join("cfg").join(OVIS).join("env"), "PORT=17417\n").unwrap();
    std::fs::write(w.base.join("cfg").join(OLD).join("env"), "PORT=17317\n").unwrap();
    // Ovis alone: found under its own id, with its own config folder, its version, and the command says which
    assert_eq!(web_ui(true), Some(WebUi { host: "127.0.0.1".into(), port: 17417, socket: w.socket.clone(), plugin: OVIS.into(), version: "0.4.14".into(), named: false }));
    assert_eq!(cmd::found_line(web_ui(true).as_ref()), "Web UI (Ovis): found at http://127.0.0.1:17417 (devswha.ovis 0.4.14)");
    // only the old id: still found, under that id
    *herdr.plugins.borrow_mut() = vec![plugin(OLD, "0.4.12", true)];
    assert_eq!(web_ui(true).map(|x| (x.plugin, x.version, x.port)), Some((OLD.to_string(), "0.4.12".to_string(), 17317)));
    assert_eq!(cmd::found_line(web_ui(true).as_ref()), "Web UI (Ovis): found at http://127.0.0.1:17317 (devswha.herdr-web-ui 0.4.12)");
    // one named by hand: no id, no version
    assert_eq!(cmd::found_line(Some(&WebUi { host: "127.0.0.1".into(), port: 7317, named: true, ..Default::default() })), "Web UI (Ovis): found at http://127.0.0.1:7317");
    // both (the installer moving it from one id to the other): Ovis, in whichever order herdr lists them
    *herdr.plugins.borrow_mut() = vec![plugin(OLD, "0.4.12", true), plugin(OVIS, "0.4.14", true)];
    assert_eq!(web_ui(true).map(|x| (x.plugin, x.port)), Some((OVIS.to_string(), 17417)));
    // both, Ovis turned off in herdr: the old one; both off: none, and the command names both ids to look for
    *herdr.plugins.borrow_mut() = vec![plugin(OVIS, "0.4.14", false), plugin(OLD, "0.4.12", true)];
    assert_eq!(web_ui(true).map(|x| x.plugin), Some(OLD.to_string()));
    *herdr.plugins.borrow_mut() = vec![plugin(OVIS, "0.4.14", false), plugin(OLD, "0.4.12", false)];
    assert_eq!(web_ui(true), None);
    assert_eq!(cmd::main(&[]).s("msg"), "Web UI (Ovis): not found: its plugin (devswha.ovis, or devswha.herdr-web-ui before the rename) is not installed in herdr\nFront door: not running");
    assert!(cmd::link_now().s("msg").contains("its plugin devswha.ovis, or devswha.herdr-web-ui before the rename"), "{}", cmd::link_now());
    sw::herdr::cli::set_fake(Some(w.fake.clone()));
}

#[test]
fn the_port_a_start_settled_on_is_read_from_the_state_folder_of_the_id_it_was_found_under() {
    let w = world();
    let (cfg, home) = (w.base.join("xdg"), w.base.join("home"));
    let d = w.base.join("webui-config");
    std::fs::create_dir_all(&d).unwrap();
    let d = d.display().to_string();
    for k in ["OVIS_STATE_DIR", "HERDR_WEB_STATE_DIR"] { env(k, None); }
    env("XDG_CONFIG_HOME", Some(&cfg.display().to_string()));
    // nothing saved: the default port, under either id
    assert_eq!((address_from(&d, OVIS).port, address_from(&d, OLD).port, address_from(&d, OVIS).plugin), (7317, 7317, OVIS.to_string()));
    // Ovis: `ovis` in the config folder (XDG_CONFIG_HOME, else ~/.config)
    port_in(&cfg.join("ovis"), 17401);
    assert_eq!((address_from(&d, OVIS).port, address_from(&d, OLD).port), (17401, 7317));
    env("XDG_CONFIG_HOME", None);
    port_in(&home.join(".config/ovis"), 17402);
    assert_eq!(address_from(&d, OVIS).port, 17402, "~/.config/ovis");
    env("XDG_CONFIG_HOME", Some(&cfg.display().to_string()));
    assert_eq!(address_from(&d, OVIS).port, 17401);
    // under the old id: its own `herdr-web-ui` folder, or HERDR_WEB_STATE_DIR, never Ovis's
    port_in(&cfg.join("herdr-web-ui"), 17403);
    assert_eq!(address_from(&d, OLD).port, 17403);
    let old_state = w.base.join("old-state"); port_in(&old_state, 17404);
    env("HERDR_WEB_STATE_DIR", Some(&old_state.display().to_string()));
    assert_eq!(address_from(&d, OLD).port, 17404);
    // Ovis with the old folder still holding state of its own (a move it could not make): it goes on using that one,
    // and the old variable is not its own
    assert_eq!(address_from(&d, OVIS).port, 17403);
    env("HERDR_WEB_STATE_DIR", None);
    // the old folder moved over, a link to the new one left at its name: the new one
    std::fs::remove_dir_all(cfg.join("herdr-web-ui")).unwrap();
    std::os::unix::fs::symlink(cfg.join("ovis"), cfg.join("herdr-web-ui")).unwrap();
    assert_eq!((address_from(&d, OVIS).port, address_from(&d, OLD).port), (17401, 17401));
    // an empty old folder is no state of its own
    std::fs::remove_file(cfg.join("herdr-web-ui")).unwrap(); std::fs::create_dir_all(cfg.join("herdr-web-ui")).unwrap();
    assert_eq!(address_from(&d, OVIS).port, 17401);
    // OVIS_STATE_DIR, from its environment or from its own settings (laid over its environment), wins
    let own = w.base.join("ovis-state"); port_in(&own, 17405);
    env("OVIS_STATE_DIR", Some(&own.display().to_string()));
    assert_eq!(address_from(&d, OVIS).port, 17405);
    let set = w.base.join("ovis-set"); port_in(&set, 17406);
    std::fs::write(Path::new(&d).join(".env"), format!("OVIS_STATE_DIR={}\n", set.display())).unwrap();
    assert_eq!(address_from(&d, OVIS).port, 17406);
    // a PORT of its own is never swapped for a saved one
    std::fs::write(Path::new(&d).join("env"), "PORT=8000\n").unwrap();
    assert_eq!((address_from(&d, OVIS).port, address_from(&d, OLD).port), (8000, 8000));
    env("OVIS_STATE_DIR", None);
}
