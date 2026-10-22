//! pi's skills and prompts (the web UI's server/skills/pi.ts): skills as `/skill:name`, yours (its own folder, any
//! depth, a loose `.md` at its top too, and `~/.agents/skills`) and the project's (`.pi/skills`, `.agents/skills`);
//! prompt templates as `/name`, yours and the project's.
use super::read::{commands, skills, Walk};
use super::{entry, Place, Skill};
use crate::agents::homes::pi_home;
use std::path::PathBuf;

pub fn list(p: &Place) -> Vec<Skill> {
    let home = pi_home();
    let skill = |root: PathBuf| -> Vec<Skill> { skills(&root, Walk::DeepLoose, true).iter().map(|f| entry(format!("/skill:{}", f.name), f)).collect() };
    let mut out = skill(home.join("skills"));
    out.extend(skill(p.home.join(".agents").join("skills")));
    for d in p.projects() { out.extend(skill(d.join(".pi").join("skills"))); out.extend(skill(d.join(".agents").join("skills"))); }
    out.extend(commands(&home.join("prompts"), false).iter().map(|f| entry(format!("/{}", f.name), f)));
    out.extend(commands(&p.cwd.join(".pi").join("prompts"), false).iter().map(|f| entry(format!("/{}", f.name), f)));
    out
}
