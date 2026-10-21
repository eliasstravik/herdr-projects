//! Finding herdr when this process's PATH lacks it (a plain `ssh host cmd` gets no login shell): the usual install
//! spots under the home folder, then a login shell's PATH; a PATH that has it keeps the bare name.
use crate::common;
use common::*;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use sw::herdr::find::{elsewhere, spots};

fn program(dir: &Path, name: &str, mode: u32) -> std::path::PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, "#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(mode)).unwrap();
    p
}

#[test]
fn herdr_in_local_bin_is_found_when_path_lacks_it_and_the_login_shell_is_not_asked() {
    let w = world();
    let home = w.base.join("home");
    let found = program(&home.join(".local/bin"), "herdr", 0o755);
    let asked = std::cell::Cell::new(false);
    assert_eq!(elsewhere("herdr", "/usr/bin:/bin", &spots(&home), || { asked.set(true); String::new() }), Some(found));
    assert!(!asked.get(), "a usual spot that has it needs no login shell");
}

#[test]
fn a_path_that_has_herdr_keeps_the_bare_name() {
    let w = world();
    let home = w.base.join("home");
    let on_path = w.base.join("tools");
    program(&on_path, "herdr", 0o755);
    program(&home.join(".local/bin"), "herdr", 0o755);
    assert_eq!(elsewhere("herdr", &format!("/usr/bin:{}", on_path.display()), &spots(&home), || panic!("not asked")), None);
}

#[test]
fn the_login_shells_path_is_asked_last_and_a_file_that_does_not_run_is_no_program() {
    let w = world();
    let home = w.base.join("home");
    program(&home.join(".cargo/bin"), "herdr", 0o644);
    let odd = w.base.join("odd/place");
    let found = program(&odd, "herdr", 0o755);
    let mine = vec![home.join(".local/bin"), home.join(".cargo/bin")]; // not this machine's own /usr/local/bin
    assert_eq!(elsewhere("herdr", "", &mine, || format!("/nowhere:{}", odd.display())), Some(found));
    assert_eq!(elsewhere("herdr", "", &mine, || "/nowhere".into()), None, "found nowhere: the bare name, and herdr's own error");
}
