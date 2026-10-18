# Crates

This page tells how the code of herdr-projects is divided into five crates, and why. Use it to find the crate for a
new module.

## The five crates

The program has five crates. Each crate is on top of the crate below it. A change rebuilds only its own crate and the
crates above it.

| Crate | Folder | What it holds |
| --- | --- | --- |
| sw-core | `crates/core/` | The data (`store/`), Herdr, the agents, and small tools that every part uses. |
| sw-flow | `crates/flow/` | The flow of a task: delegation, the finish line, landing, the keeper, other machines and automations. |
| sw-board | `crates/board/` | The board's screen (`board/`): its state, what a key does and what the screen draws. |
| sw-web | `crates/web/` | The board in the Herdr web UI (`webui/`): the front door and the web board. |
| herdr-projects | `Cargo.toml`, `src/` | The command line (`cli/`) and the made pages (`docs.rs`). The name of the library is `sw`. |

## Why the code has several crates

After each edit, the compiler reads all of a crate again. It reads the crate one more time for its tests. At first,
the code was one crate of 27,000 lines. Then each edit caused a compile of all of it, also an edit of one line of the
board's screen.

Most changes are to the board's screen and the web board. These changes rebuild sw-board, sw-web and the command line.
sw-core and sw-flow stay as they were. A change to sw-core rebuilds all five crates.

## Where a new module goes

1. Find the lowest crate whose code the new module uses.
2. Add one `pub mod` line for the module in the `src/lib.rs` file of that crate.
3. If the new file is in a folder, add one line for it in the `mod.rs` file of that folder.

A module has the same path in every crate. Each crate's `src/lib.rs` names its own modules (`pub mod store;`). It also
passes on the modules of the crate below it as its own (`pub use sw_core::*;`). Thus a module is `crate::store` in
every crate, and `sw::store` in the tests. If you move a module to another crate, no path that uses it changes.

## A crate uses only the crates below it

The compiler enforces this rule. For example, code in sw-flow that names `crate::board::view` does not build.

If code in a lower crate needs something from a higher crate, do one of these:

- Move the thing down to the lower crate.
- Let the caller in the higher crate hand the thing in.

### Code that was moved down

- The keys' table (`crates/flow/src/board/keys.rs`) and the theme (`crates/flow/src/board/theme/`) are built in
  sw-flow, in a `board` module of its own. The reason: a message of the flow names a key, and the review page uses
  the board's colours. The `board` module of sw-board passes them on. Thus they are `crate::board::keys` in all crates.
- The parts of the web UI that the flow and the board's screen use are also in sw-flow (`crates/flow/src/webui/`):
  - where the web UI is (`find.rs`);
  - its PCs (`pcs.rs`);
  - its link (`link.rs`, `command.rs`);
  - the code that passes a request on (`proxy.rs`);
  - the front door when something outside starts it (`keep.rs`).
- The front door's own loop, which serves the web board, stays in sw-web: it is `door_loop.rs`.

### Code that is handed in

The page server (`review_page::serve_main`, in sw-flow) is also the front door of the Herdr web UI. The command line
hands it the loop of the front door (`webui::door_loop::run_door`, in sw-web).

## A new worktree

The code of each crate is in its own folder. Thus cargo tracks what it built a crate from by paths inside that folder.
These paths are the same in every worktree.

A new worktree starts from the warm build of the main line (see
[the build cache and the warm start](testing-build-cache.md)). In the new worktree, a crate counts as built if none of
its files changed since that build. Cargo builds again only the crates that a change reaches.

The shared build cache keeps the libraries, not these crates. The compiler keeps its own notes on the crates in each
`target/` folder (incremental compilation). A cache cannot share these notes.

## What it saves

These times were measured on vm-str (16 cores, with other tasks building at the same time), on this task's share of
two cores. Each time is for a one-line change to a file of each crate. Each time is the median of three runs, but a
fresh worktree had one run.

| Change in | Program built again, before → after | Fresh worktree from the warm build, before → after |
| --- | --- | --- |
| the board's screen (sw-board) | 2.7 s → 1.5 s | 12.8 s → 4.8 s |
| the web board (sw-web) | 2.9 s → 1.1 s | 13.1 s → 2.6 s |
| the command line (`src/`) | 2.8 s → 0.7 s | 13.5 s → 1.5 s |
| the flow (sw-flow) | 2.8 s → 2.4 s | 12.5 s → 10.7 s |
| the data (sw-core) | 2.7 s → 3.2 s | 12.9 s → 13.1 s |

A build from nothing takes about the same time (33 s → 30 s). The test program also takes about the same time (4 s
after a change, 15 s in a fresh worktree). The reason: the test program is one crate of its own, `tests/main.rs`, and
cargo builds it again after any change below it.

## Related pages

- [How the tests work](testing.md)
- [The build cache and the warm start](testing-build-cache.md)
- [Linking with mold](linking.md)
- [Run the tests](run-the-tests.md)
- [AGENTS.md](../../AGENTS.md)
