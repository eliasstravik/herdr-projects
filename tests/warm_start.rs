//! [real programs] A new worktree's warm start (`scripts/warm-start.sh`): the build a worktree of the main line keeps,
//! a later worktree copies, with only what changed since built again; what was built with the other folder's path in
//! it built again too; nothing kept off the main line, from a changed folder or older than what is kept. The builds
//! are stand-ins (files laid out as cargo lays them out): the script is what is tested, not cargo.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

/// A repo with a main line, its worktrees beside it, and a cache folder of its own for the kept builds.
struct Bench { base: PathBuf }
impl Bench {
    fn new(name: &str) -> Bench {
        let base = std::env::temp_dir().join(format!("sw-warm-start-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("repo/src")).unwrap();
        let b = Bench { base };
        b.git("repo", &["init", "-q", "-b", "main"]);
        b.write("repo", "src/a.rs", "a");
        b.write("repo", "src/b.rs", "b");
        b.write("repo", ".gitignore", "/target/\n");
        b.commit("repo", "first");
        b
    }
    fn dir(&self, wt: &str) -> PathBuf { self.base.join(wt) }
    fn git(&self, wt: &str, a: &[&str]) -> String {
        let o = Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(a).current_dir(self.dir(wt))
            .env("GIT_CONFIG_GLOBAL", "/dev/null").env("GIT_CONFIG_NOSYSTEM", "1").output().unwrap();
        assert!(o.status.success(), "git {a:?}: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    }
    fn write(&self, wt: &str, f: &str, text: &str) {
        let p = self.dir(wt).join(f);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    fn commit(&self, wt: &str, msg: &str) { self.git(wt, &["add", "-A"]); self.git(wt, &["commit", "-q", "-m", msg]); }
    /// A new worktree of the repo, on a branch of its own from `from`.
    fn worktree(&self, wt: &str, from: &str) {
        let at = self.dir(wt).display().to_string();
        self.git("repo", &["worktree", "add", "-q", "-b", &format!("work/{wt}"), &at, from]);
    }
    /// A build as cargo leaves it: the program, built with this folder's path written into it, and a library whose
    /// generated code is in its own target/.
    fn build(&self, wt: &str) {
        let here = std::fs::canonicalize(self.dir(wt)).unwrap().display().to_string();
        self.write(wt, "target/quick/herdr-projects", &format!("built in {wt}"));
        self.write(wt, "target/quick/deps/sw-1a.d", &format!("# env-dep:CARGO_MANIFEST_DIR={here}\n"));
        self.write(wt, "target/quick/.fingerprint/herdr-projects-1a/lib-sw", "1");
        self.write(wt, "target/quick/deps/serde-2b.d", &format!("# env-dep:OUT_DIR={here}/target/quick/build/serde-3c/out\n"));
        self.write(wt, "target/quick/.fingerprint/serde-2b/lib-serde", "2");
    }
    /// The workspace setup's two steps around its build: `ws_seed`, the build (the stand-in, when `build`), `ws_keep`.
    fn setup(&self, wt: &str, build: bool) -> String {
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/warm-start.sh");
        let built = if build { format!("mkdir -p target/quick && echo 'built in {wt}' > target/quick/herdr-projects") } else { ":".into() };
        let o = Command::new("/bin/sh").arg("-c").arg(format!("set -e; . '{}'; ws_seed; {built}; ws_keep", script.display()))
            .current_dir(self.dir(wt)).env("XDG_CACHE_HOME", self.base.join("cache")).env_remove("HERDR_PROJECTS_NO_WARM")
            .env_remove("CARGO_TARGET_DIR").env("GIT_CONFIG_GLOBAL", "/dev/null").env("GIT_CONFIG_NOSYSTEM", "1").output().unwrap();
        assert!(o.status.success(), "the setup's steps failed: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).into_owned()
    }
    /// The commit the kept build was built from, if one is kept.
    fn kept(&self) -> Option<String> {
        let warm = std::fs::read_dir(self.base.join("cache/herdr-projects/warm")).ok()?.next()?.ok()?.path();
        let name = std::fs::read_to_string(warm.join("current")).ok()?;
        std::fs::read_to_string(warm.join(name.trim()).join("commit")).ok().map(|c| c.trim().to_string())
    }
    fn has(&self, wt: &str, f: &str) -> bool { self.dir(wt).join(f).exists() }
    fn changed(&self, wt: &str, f: &str) -> SystemTime { std::fs::metadata(self.dir(wt).join(f)).unwrap().modified().unwrap() }
}
impl Drop for Bench { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.base); } }

fn long_ago() -> SystemTime { SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000) }

#[test]
fn real_programs_a_new_worktree_starts_from_the_build_kept_and_builds_again_only_what_changed_since() {
    let b = Bench::new("warm");
    let first = b.git("repo", &["rev-parse", "HEAD"]);
    // the first worktree has nothing to start from: it builds, and keeps its build
    b.worktree("one", "main");
    assert_eq!(b.setup("one", false), "");
    b.build("one");
    assert_eq!(b.setup("one", false), "", "a folder with a build of its own is left alone");
    assert_eq!(b.kept(), None, "only a build made by the setup in a fresh folder is kept");
    b.worktree("two", "main");
    b.setup("two", true);
    assert_eq!(b.kept(), Some(first.clone()));
    // the main line moves on: b.rs changes
    b.write("repo", "src/b.rs", "b2");
    b.commit("repo", "b changes");
    b.worktree("three", "main");
    let said = b.setup("three", false);
    assert!(said.contains(&format!("Started from the warm build of {}", &first[..7])), "{said}");
    assert_eq!(std::fs::read_to_string(b.dir("three").join("target/quick/herdr-projects")).unwrap().trim(), "built in two");
    assert!(b.changed("three", "src/a.rs") < long_ago(), "a file as it was built is taken as built");
    assert!(b.changed("three", "src/b.rs") > long_ago(), "a file changed since is built again");
    assert_eq!(b.git("three", &["status", "--porcelain"]), "", "only the times changed");
    // and it kept its own, the newer
    assert_eq!(b.kept(), Some(b.git("repo", &["rev-parse", "HEAD"])));
}

#[test]
fn real_programs_what_was_built_with_the_other_folders_path_in_it_is_built_again() {
    let b = Bench::new("path");
    b.worktree("one", "main");
    b.setup("one", true);
    b.build("one");
    // kept again by hand as the setup would have, with the stand-in build's files in it
    let warm = std::fs::read_dir(b.base.join("cache/herdr-projects/warm")).unwrap().next().unwrap().unwrap().path();
    let name = std::fs::read_to_string(warm.join("current")).unwrap();
    let kept = warm.join(name.trim()).join("target");
    std::fs::remove_dir_all(&kept).unwrap();
    assert!(Command::new("cp").arg("-pR").arg(b.dir("one").join("target")).arg(warm.join(name.trim())).status().unwrap().success());
    b.worktree("two", "main");
    let said = b.setup("two", false);
    assert!(!b.has("two", "target/quick/.fingerprint/herdr-projects-1a"), "the program said where it was built: built again; setup said {said:?}; target files: {:?}", std::fs::read_dir(b.dir("two").join("target/quick")).unwrap().map(|e| e.unwrap().path()).collect::<Vec<_>>());
    assert!(b.has("two", "target/quick/.fingerprint/serde-2b"), "a path into its own target/ cargo moves itself");
}

#[test]
fn real_programs_no_build_is_kept_off_the_main_line_from_a_changed_folder_or_older_than_the_one_kept() {
    let b = Bench::new("kept");
    let first = b.git("repo", &["rev-parse", "HEAD"]);
    b.write("repo", "src/a.rs", "a2");
    b.commit("repo", "a changes");
    let second = b.git("repo", &["rev-parse", "HEAD"]);
    b.worktree("new", "main");
    b.setup("new", true);
    assert_eq!(b.kept(), Some(second.clone()));
    // an older commit of the line is no warmer
    b.worktree("old", &first);
    b.setup("old", true);
    assert_eq!(b.kept(), Some(second.clone()));
    // a commit off the main line
    b.worktree("side", "main");
    b.write("side", "src/c.rs", "c");
    b.commit("side", "side work");
    b.setup("side", true);
    assert_eq!(b.kept(), Some(second.clone()));
    // a folder changed before its build
    b.write("repo", "src/a.rs", "a3");
    b.commit("repo", "a changes again");
    b.worktree("dirty", "main");
    b.write("dirty", "src/a.rs", "mine");
    let said = b.setup("dirty", true);
    assert!(said.contains("Started from the warm build"), "{said}");
    assert!(b.changed("dirty", "src/a.rs") > long_ago(), "a file changed in the folder is built again");
    assert!(b.changed("dirty", "src/b.rs") < long_ago());
    assert_eq!(b.kept(), Some(second));
}
