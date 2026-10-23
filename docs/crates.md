# Crates

**The program is five crates, stacked one on top of the other.** A change rebuilds only its own crate and the ones
above it.

| Crate | Folder | What it holds |
|---|---|---|
| sw-core | `crates/core/` | the data (`store/`), Herdr and the agents, and small tools every part uses |
| sw-flow | `crates/flow/` | the flow of a task: delegating, the finish line, landing, the keeper, other machines, automations |
| sw-board | `crates/board/` | the board's screen (`board/`): its state, what a key does, what it draws |
| sw-web | `crates/web/` | the board in the web UI, Ovis (`webui/`): the front door and the web board |
| herdr-projects | `Cargo.toml`, `src/` | the command line (`cli/`) and the made pages (`docs.rs`); the library is `sw` |

Why: the compiler reads a whole crate again after any edit (and once more for its tests). When it was one crate of
27,000 lines, every edit, even one line of the board's screen, paid for all of it.

Most changes are to the board's screen and the web board. Those rebuild sw-board, sw-web and the command line, and
keep sw-core and sw-flow as they were. A change to sw-core rebuilds all five.

## Where a new module goes

**In the lowest crate whose code it uses**: one `pub mod` line in that crate's `src/lib.rs`. A new file in a folder is
one line in that folder's `mod.rs`.

A module has the same path in every crate. Each crate's `src/lib.rs` names its modules (`pub mod store;`) and passes on
those of the crate below as its own (`pub use sw_core::*;`). So a module is `crate::store` in every crate and
`sw::store` in the tests. Moving a module to another crate changes no path that uses it.

## A crate uses only the crates below it

The compiler enforces it: code in sw-flow that names `crate::board::view` does not build. When lower code needs
something from above, either move it down, or let the caller above hand it in.

- **Moved down.**
  - The keys' table (`crates/flow/src/board/keys.rs`) and the theme (`crates/flow/src/board/theme/`) are built in
    sw-flow, in a `board` module of its own: a message of the flow names a key, and the review page uses the board's
    colours. sw-board's `board` module passes them on, so they are `crate::board::keys` everywhere.
  - The same for what the flow and the board's screen use of the web UI (`crates/flow/src/webui/`): where it is
    (`find.rs`), its PCs (`pcs.rs`), its link (`link.rs`, `command.rs`), passing a request on (`proxy.rs`) and the
    front door started from outside (`keep.rs`). The front door's own loop, which serves the web board, is
    `door_loop.rs` in sw-web.
- **Handed in.** The page server (`review_page::serve_main`, sw-flow) is also the front door of the web UI (Ovis). The
  command line hands it the front door's loop (`webui::door_loop::run_door`, sw-web).

## A new worktree

Each crate's code is in its own folder, so cargo tracks what a crate was built from by paths inside that folder, the
same in every worktree. A new worktree starts from the warm build of the main line ([testing](testing.md)). There, a
crate with no changed files since that build counts as built: only the crates a change reaches are built again.

The shared build cache keeps the libraries, not these crates. The compiler keeps its own notes on them in each
`target/` (incremental compilation), which a cache cannot share.

## What it saves

Measured on vm-str (16 cores, other tasks building at the same time), on this task's share of two cores, after a
one-line change to a file of each crate (the median of three runs; one run for a fresh worktree):

| Change in | Program built again, before → after | Fresh worktree from the warm build, before → after |
|---|---|---|
| the board's screen (sw-board) | 2.7 s → 1.5 s | 12.8 s → 4.8 s |
| the web board (sw-web) | 2.9 s → 1.1 s | 13.1 s → 2.6 s |
| the command line (`src/`) | 2.8 s → 0.7 s | 13.5 s → 1.5 s |
| the flow (sw-flow) | 2.8 s → 2.4 s | 12.5 s → 10.7 s |
| the data (sw-core) | 2.7 s → 3.2 s | 12.9 s → 13.1 s |

A build from nothing is about the same (33 s → 30 s). So is the test program (4 s after a change, 15 s in a fresh
worktree): it is one crate of its own, `tests/main.rs`, rebuilt after any change below it.
