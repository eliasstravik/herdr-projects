//! A call started from the terminal board, which runs on one machine while you sit at another (or at the same one): the
//! board rings (ring.rs), and the call helper on the computer you sit at picks it up and holds the call right there,
//! with that computer's microphone and speaker and no browser (helper.rs, native/). The helper reaches the board's
//! machine the way the board reaches other machines, over SSH to herdr's saved machines, and listens there (listen.rs);
//! of several computers listening, the one you are at takes it (here.rs): the board's own computer, when its herdr
//! window there is in use, and no other. `herdr-projects voice` says how it stands (command.rs).
pub mod command;
pub mod helper;
pub mod here;
pub mod listen;
pub mod native;
pub mod ring;
