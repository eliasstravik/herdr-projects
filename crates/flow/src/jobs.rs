//! The detached runner's entry: `herdr-projects run <job> <task id>`. Every runner runs one housekeeping pass as it
//! exits, so a slot freed by a merge is reused at once.
use crate::herdr::cli::current_socket;
use crate::housekeeping::{housekeeping, poll_prs, Force};
use crate::json::*;
use crate::runner::inline;

pub fn run_job(job: &str, id: &str, extra: &[String]) -> i32 {
    // A stop from the board ends this runner's own subprocesses with it: the runner leads its own process group.
    if !inline() {
        extern "C" fn on_term(_: i32) { unsafe { libc::kill(0, libc::SIGKILL); libc::_exit(0); } }
        unsafe { libc::signal(libc::SIGTERM, on_term as *const () as usize); }
    }
    let has = |f: &str| extra.iter().any(|x| x == f);
    let mut id = id.to_string();
    match job {
        "delegate" => crate::delegate::jobs::delegate_job(&id),
        "restart" => crate::delegate::jobs::restart_job(&id),
        "finish" => crate::finish::turn_ended::finish_job(&id, has("--force")),
        "pipeline" => crate::finish::pipeline::pipeline_job(&id),
        "chatreview" => crate::finish::chat::chat_review_job(&id),
        "sync" => crate::finish::sync_job::sync_job(&id, has("--force")),
        "compare" => crate::attempts::compare::compare_job(&id),
        "recover" => crate::finish::recover_job::recover_job(&id),
        "land" => crate::land::merge::land_job(&id),
        "cleanup" => crate::land::cleanup::cleanup_job(&id, if has("--merged") { "merged" } else { "always" }),
        "prpoll" => poll_prs(),
        // What ended tasks left behind (leftovers/sweep.rs).
        "sweep" => { crate::leftovers::sweep::sweep(); }
        // What the tasks waited for on other machines are now (after/far_ask.rs).
        "afterfar" => crate::after::far_ask::far_pass(),
        // Its id is a herdr session, not a task.
        "coordinator" => { crate::coordinator::pump::pump(&if id.is_empty() || id == "-" { current_socket() } else { id.clone() }); id = "-".into(); }
        _ => { eprintln!("Unknown job {job}"); return 1; }
    }
    let socket = if id.is_empty() || id == "-" { current_socket() } else { crate::store::tasks::get(&id).map(|t| t.at("link").t("socket")).filter(|s| !s.is_empty()).unwrap_or_else(current_socket) };
    housekeeping(&socket, Force::Yes);
    0
}
