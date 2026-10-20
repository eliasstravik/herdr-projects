//! The one program: `herdr-projects` and its subcommands. Argument routing only: everything lives in the library.
fn main() {
    sw::temp::leave_task_env(); // a command a task's agent ran is the board's, not that task's (temp.rs)
    std::process::exit(sw::cli::main(std::env::args().skip(1).collect()));
}
