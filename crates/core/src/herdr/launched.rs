//! What a task's agent launched in its pane, to end when the task is stopped: the pane's whole process tree below its
//! shell, split into the pane's own (its shell, the agent, the agent's helpers wired to it and what they run, busy.rs's
//! rule) and what the agent launched (a command in its group not wired to it, one in a group of its own, a job of the
//! shell, and everything those run).
use super::cli::kids;
use super::procs::{wiring, Wiring};
use super::Herdr;
use crate::json::*;

/// A pane's tree: `own` is kept (the agent stays, for a later reply), `launched` is what the agent started.
pub struct Tree { pub shell: Option<i64>, pub own: Vec<i64>, pub launched: Vec<i64> }

/// The pane's tree, with the program `agent` at its front. None: herdr does not say what runs in it.
pub fn tree(api: &Herdr, id: &str, agent: &str) -> Option<Tree> {
    let i = api.process_info(id)?;
    let procs: Vec<&Value> = i.arr("foreground_processes").iter().filter(|p| crate::gate::is_int(p.at("pid"))).collect();
    let front: Vec<i64> = procs.iter().map(|p| p.i("pid")).collect();
    let shell = crate::gate::is_int(i.at("shell_pid")).then(|| i.i("shell_pid"));
    let lead = i.i("foreground_process_group_id");
    // roots: the shell and the agent itself, whose other processes are each looked at; helpers keep what they run
    let mut roots: Vec<i64> = shell.into_iter().collect();
    let (mut helpers, mut launched) = (vec![], vec![]);
    for p in &procs {
        let pid = p.i("pid");
        if roots.contains(&pid) { continue; }
        if (Some(lead) != shell && pid == lead) || (!agent.is_empty() && p.s("name") == agent) { roots.push(pid); }
        else if wiring(pid, &front) == Wiring::Agent { helpers.push(pid); } else { launched.push(pid); }
    }
    let mut queue: Vec<i64> = shell.into_iter().chain(front.iter().copied()).collect();
    let mut seen = queue.clone();
    while let Some(p) = queue.pop() {
        let Ok(k) = kids(p) else { continue };
        for c in k {
            if seen.contains(&c) { continue; }
            seen.push(c);
            queue.push(c);
            if launched.contains(&p) { launched.push(c); }
            else if helpers.contains(&p) || wiring(c, &front) == Wiring::Agent { helpers.push(c); } // a helper in a group of its own
            else { launched.push(c); }
        }
    }
    let own = roots.into_iter().chain(helpers).collect();
    Some(Tree { shell, own, launched })
}
