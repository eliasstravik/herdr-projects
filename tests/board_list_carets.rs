//! The caret on a heading that folds: `▾` open, `▸` folded, in the marks' column and the muted ink; the automations'
//! heading folds too and says how many it holds; a narrow list keeps the caret.
use crate::common;
use common::board::open;
use common::*;
use sw::automations::list::{self as l, Add};
use sw::automations::triggers::parse_schedule;
use sw::json::*;
use sw::shown::ink::Ink;

#[test]
fn a_heading_that_folds_has_a_caret_that_says_which_way() {
    let w = world();
    sw::clock::set(Some(sw::clock::now()));
    for n in 1..=2 { w.add(&format!("Task {n}")); }
    l::add(Add { name: "Nightly audit", prompt: "Audit.", trigger: Some(parse_schedule("daily 03:00").unwrap()), repo: "", folder: &w.root, settings: Obj::new(), research: None }).unwrap();
    let mut s = open(&w.root, 30); s.s.side = false; s.load();
    let lines = s.screen(80);
    let at = |lines: &[String], start: &str| lines.iter().position(|l| l.starts_with(start)).unwrap_or_else(|| panic!("{start}\n{}", lines.join("\n")));
    // Open: `▾` in the marks' column, the name where a task's tag starts (both a step in under the machine's row).
    let todo = at(&lines, "   ▾ to do");
    let task = at(&lines, "   ○ PRJ-");
    assert_eq!(lines[todo].find("to do"), lines[task].find("PRJ-"), "{}", lines.join("\n"));
    // The automations fold as a group does: a caret, the name in the headings' column.
    let autos = at(&lines, "   ▾ automations");
    assert_eq!(lines[autos].find("automations"), lines[todo].find("to do"));
    // In the muted ink, the name in its own.
    let inked = s.inked(80);
    let caret = inked[todo].0.iter().find(|p| p.t.contains('▾')).map(|p| p.c);
    assert_eq!(caret, Some(Ink::Muted));
    // Folded (←): `▸`, and how many tasks it holds.
    s.s.sel = "head:status\t\tto do".into(); s.load();
    s.press(&["left"]);
    let lines = s.screen(80);
    assert!(lines.iter().any(|l| l == "▌  ▸ to do  2 tasks"), "{}", lines.join("\n"));
    assert!(!lines.iter().any(|l| l.contains('▾') && l.contains("to do")), "{}", lines.join("\n"));
    // Narrow: the caret stays, the line never runs past the edge.
    for width in [12, 9] {
        let narrow = s.screen(width);
        let line = narrow.iter().find(|l| l.contains('▸')).unwrap_or_else(|| panic!("{}", narrow.join("\n")));
        assert!(line.starts_with("▌ ▸ to") && line.chars().count() <= width, "{line:?}");
    }
    s.press(&["right"]);
    // The automations' heading folded: `1 automation`, its row gone.
    for _ in 0..5 { if s.s.sel == "head:status\t\tautomations" { break; } s.press(&["down"]); }
    s.press(&["left"]);
    let lines = s.screen(80);
    assert!(lines.iter().any(|l| l == "▌  ▸ automations  1 automation") && !lines.iter().any(|l| l.contains("Nightly")), "{}", lines.join("\n"));
    s.press(&["right"]); for _ in 0..5 { if s.s.sel == "head:status\t\tto do" { break; } s.press(&["up"]); }
    let narrow = s.screen(12);
    assert!(narrow.iter().any(|l| l.starts_with("▌ ▾ to do")), "{}", narrow.join("\n"));
}
