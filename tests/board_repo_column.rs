//! The repo column: setting `board.repoColumn` puts each task's repo name on its row in the board of every repo.
use crate::common;
use sw::board::view::repo_col::ink_of;
use sw::json::*;
use sw::shown::ink::{Ink, Inked};

fn repos(on: bool) -> (common::World, common::board::Board) {
    let w = common::world_with(common::Opts { repo: true, settings: json!({"board.repoColumn": on}), ..Default::default() });
    for name in ["alpha", "beta"] {
        let dir = w.base.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        common::git(&dir, &["init", "-q"]);
        sw::actions::filing::add(&dir, &format!("{name} job"), Default::default()).unwrap();
    }
    let b = common::board::open(&w.root, 40);
    (w, b)
}
fn row_of<'a>(lines: &'a [String], title: &str) -> &'a str { lines.iter().map(|l| l.split('│').next().unwrap_or("")).find(|l| l.contains(title)).unwrap_or_else(|| panic!("no row {title}")) }

#[test]
fn board_repo_column_on_shows_each_rows_repo_before_the_trailing_columns() {
    let (_w, mut b) = repos(true);
    let lines = b.screen(100);
    for name in ["alpha", "beta"] {
        let row = row_of(&lines, &format!("{name} job"));
        assert!(row.contains(&format!("  {name} ")), "{row}");
        assert!(row.find("job").unwrap() < row.rfind(name).unwrap() && row.rfind(name).unwrap() < row.find("ago").unwrap(), "{row}");
    }
    assert!(!row_of(&lines, "alpha job").contains("beta"));
}
#[test]
fn board_repo_column_off_shows_none() {
    let (_w, mut b) = repos(false);
    let lines = b.screen(100);
    assert!(!row_of(&lines, "alpha job").replace("alpha job", "").contains("alpha"));
    assert!(!row_of(&lines, "beta job").replace("beta job", "").contains("beta"));
}
#[test]
fn board_repo_column_hidden_in_a_single_repo_view() {
    let (_w, mut b) = repos(true);
    sw::board::run::prefs::apply(&mut b.s, &obj(json!({"repos": ["alpha"]})));
    sw::board::update::list::relist(&mut b.s);
    let lines = b.screen(100);
    assert!(!row_of(&lines, "alpha job").replace("alpha job", "").contains("alpha"));
}
/// The narrowest screen that holds the whole title, and whether the repo's name stood on the row at the widest one.
fn narrowest(on: bool) -> (usize, bool) {
    let (_w, mut b) = repos(on);
    let whole = |b: &mut common::board::Board, cols: usize| b.screen(cols).iter().any(|l| l.split('│').next().unwrap_or("").contains("alpha job"));
    let least = (14..120).find(|c| whole(&mut b, *c)).unwrap();
    let tight = row_of(&b.screen(least), "alpha job").replace("alpha job", "").contains("alpha");
    (least, tight)
}
#[test]
fn board_repo_column_goes_before_the_title_is_cut() {
    // The title needs the same width with the column on as off, and at that width the column has given way.
    let (off, _) = narrowest(false);
    let (on, shown) = narrowest(true);
    assert_eq!(on, off);
    assert!(!shown);
}
#[test]
fn board_repo_column_ink_is_stable_per_repo() {
    assert_eq!(ink_of("alpha"), ink_of("alpha"));
    assert_ne!(ink_of("alpha"), ink_of("beta"));
    let (_w, mut b) = repos(true);
    let inked: Vec<Inked> = b.inked(100);
    let find = |name: &str| -> Ink { inked.iter().flat_map(|l| l.0.iter()).find(|p| p.t.trim() == name).map(|p| p.c).unwrap_or_else(|| panic!("no {name}")) };
    assert_eq!(find("alpha"), ink_of("alpha"));
    assert_eq!(find("beta"), ink_of("beta"));
}
