//! A call started from the terminal board, which runs on one machine while you sit at another: the board rings
//! (ring.rs), and the call helper on the computer you sit at picks it up and opens the call's page there, in a small
//! browser window of its own, with that computer's microphone, speaker and echo cancelling (helper.rs, browser.rs). The
//! helper reaches the board's machine the way the board reaches other machines, over SSH to herdr's saved machines, and
//! listens there (listen.rs); of several computers listening, the one you are at takes it. No helper: the board shows
//! a link to take the call in this computer's browser. `herdr-projects voice` says how it stands (command.rs).
pub mod browser;
pub mod command;
pub mod helper;
pub mod listen;
pub mod ring;
