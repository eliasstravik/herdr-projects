mod actions;
mod adopt;
mod cli;
mod coordinator;
mod doctor;
mod herdr;
mod inbox;
mod lifecycle;
mod overview;
mod paths;
mod pr;
mod project;
mod remote;
mod routine;
mod runner;
#[cfg(test)]
mod scenarios;
mod steps;
mod thread;
mod threads;
mod ticker;

/// Crate version plus a build identifier (short git hash and build time), so a
/// rebuilt binary always differs from the one a running ticker was started from.
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+", env!("HP_BUILD_ID"));

fn main() {
    if let Err(error) = cli::run() {
        eprintln!("herdr-projects: {error:#}");
        std::process::exit(1);
    }
}
