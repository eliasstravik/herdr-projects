# Working on herdr-projects

Several agents change this repo at once. Before you add or change anything, look at the map: it says which file
holds what, so that two tasks do not edit the same file.

- Build the program once: `cargo build --profile quick` (the workspace setup, `scripts/setup.sh`, does it for you,
  through the build cache every worktree shares). It is then `./target/quick/herdr-projects`.
- `./target/quick/herdr-projects docs --where` prints all of the map; `… docs --where crates/board/src/board/view` only
  that folder.
- It is also a page in your own folder, `docs/where-things-live.md`, written by the workspace setup and by
  `npm run check`. It is not in git: never commit it. The tests are one file per area (`ls tests`), all of them
  modules of one test program, `tests/main.rs`.

The program is Rust, one native program: `Cargo.toml` at the root, the command in `src/main.rs`. Its code is five
crates, one on top of the other, so that a change builds again only its own crate and those on top of it: sw-core
(`crates/core/`: the data, herdr, the agents), sw-flow (`crates/flow/`: the flow of a task), sw-board
(`crates/board/`: the board's screen), sw-web (`crates/web/`: the board in the herdr web UI), and the command line
with the made pages in `src/` (`sw`, which the tests use) ([docs/crates.md](docs/crates.md)). There is no Node and
nothing to `npm install`; `package.json` is only there so that every agent finds the one command that runs the tests.

## Where new things go

- **One area, one small file.** Put a change in the file of its area. A new area is a new file, not more lines at
  the end of an old one. A source file may not pass 300 lines (the check fails): split it by area into a folder of
  small files with the old file passing them on, as `crates/flow/src/finish.rs` and `crates/flow/src/finish/` do.
- **Every source file starts with a comment that says what it holds**: `//!` lines in a `.rs` file (`#` in a shell
  script, `//` in the web page's `.js`, `/* */` in its `.css`). Its first sentence is its line in the map; the check
  fails for a file without one.
- **A new module goes in the lowest crate whose code it uses**: one `pub mod` line in that crate's `src/lib.rs`. Every
  crate passes on the modules of those below it, so it is `crate::store` in every crate and `sw::store` in the tests. A
  crate cannot use the crates on top of it (it does not build): move what it needs down, or let the caller above hand
  it in ([docs/crates.md](docs/crates.md)).
- **A part of the board's screen is a file in `crates/board/src/board/view/`** (a part of the task panel:
  `crates/board/src/board/view/panel/`), named in that folder's `mod.rs` with one `pub mod` line beside its kin. What it
  draws is that file's; what a key does to the board is `crates/board/src/board/update/`, one file per area; the board's
  one record of what it shows is `crates/board/src/board/state.rs`.
- **A key is one entry in `crates/flow/src/board/keys.rs`**: its chord, its name, what it does, its hint, its line in
  the `?` menu, its button on the web board. The key handler, the menu, the hint line and `docs/keys.md` all read that
  list. What it does is its arm in `crates/board/src/board/update/` (a key with no arm there does not compile). Never
  write a key's chord anywhere else (the check fails): test for it with `keys::is(p, K::…)`, and name it in a message
  with `key_name("undo")`.
- **A setting is one entry in `crates/core/src/store/settings_list.rs`**, beside the ones of its area: its default and
  what it means. Its row in the task panel is `crates/board/src/board/view/panel/settings.rs`.
- **In a list many tasks add to, one entry per line (or per block), placed beside the entries it belongs with**, never
  at the end: two additions at the end always conflict.
- **Never edit a made table by hand**: what is between the `made:` lines in `docs/keys.md` and `docs/settings.md`.
  Change the code; `npm run check` writes the table when it is out of date and says so (`herdr-projects docs` does
  the same): commit what it wrote. A merge conflict in a made table is settled the same way: run the command, do not
  pick lines.
- **A task's words go in the `docs/` page of its own topic, and only there.** A new topic is a new page. A page that
  grows a second topic is split.
- **Do not touch `README.md` for feature work.** It is only what this is and how to install and start it. It has no
  list of pages: a new page is found in `docs/`.
- **A report, a review or a research result is never committed to the repo**, in no folder. It is kept in the task's
  own folder among the board's reports (`~/.herdr-projects/reports/<repo>/<task number>/`, outside every repo) and
  shared as a private link by the board ([docs/research.md](docs/research.md)).

## Tests

One command runs tests, for everyone: `npm run check` (`npm test` is the same; both run `scripts/check.sh`). It builds
the program with the `quick` profile, writes the made tables when they are out of date, checks the repo's own rules
(`herdr-projects docs --guard`: file sizes, the comment at the top, chords only in the keys' table, no made table out
of date) and runs the tests, in several processes at once (`scripts/run-tests.sh`). It remembers what passed for this
exact code (every file of the folder, committed or not, and the compiler), and shares that with the reviewer and the
board: asked again, it answers at once.

- **Run cargo through the check**: it builds through the one build cache every worktree shares and takes its share of
  the cores, all of them when it runs alone (`scripts/build-cache.sh`, `scripts/share-cores.sh`).
- **While you work, run the tests of what you touched**: `npm run check board-panel` runs one area's test files
  (`tests/board_panel.rs` and `tests/board_panel_*.rs`), `npm run check gate sync` several. A name that is no area runs
  the tests whose own names hold it (`npm run check textbox`): a second or two after the build. With no name it runs
  them all, about 15 seconds on a quiet machine.
- **The board runs the whole check at the end**, when you finish: `npm run check -- --all`. Run that yourself at most
  once, last of all, when your task asks for it; never after each edit. The board's run then answers at once.
  On a later round (a review or a failed check sent the work back) it runs the shorter check instead,
  `npm run check -- --since <commit>`: only the test files your change since the last whole run reaches.
- **Never wait in a loop for the machine to be quiet**, and never look at the load before a run. A busy machine fails
  nothing: the check has no time budget, and the light tests never wait for the real clock. `Blocking waiting for
  file lock` is cargo letting another build in the same folder finish first: leave it, it goes on by itself.
- A failed test is the only thing to fix.
- The check writes the made tables (`docs/keys.md`, `docs/settings.md`) when they are out of date and says so: commit
  them. `--fresh` runs again what already passed; you rarely need it.
- A new test file is a module of the one test program: one `mod` line in `tests/main.rs`, in its place in the list
  (the check fails a file left out), and `use crate::common;` in it, never `mod common;`.
- New tests are the light kind: they call the code in the test's own process. `world()` in `tests/common/mod.rs`
  gives a test a board folder of its own, a git repo, a fake herdr that answers in the test's own thread and agents
  that play a script (`w.script(json!({"worker": [step, …]}))`); `settle(id)` runs what would have been the board's
  background jobs until nothing moves; `common::board::open` gives the board itself, to press keys on and read as
  text. A test starts no program and never sleeps or depends on the machine's speed (move the clock:
  `sw::clock::set`, `sw::clock::advance`). A real program only when the program itself is what is tested, in a test
  whose name starts with `real_programs_`.
- Whole frames of the board are kept in `tests/frames/`. A change to what the board looks like changes them on
  purpose: `UPDATE_FRAMES=1 npm run check frames` writes them again; look at what changed before you commit it.
- A look at the real thing (a real terminal, a browser screenshot) only for a task that changes what the board looks
  like.

How it works, and why: [docs/testing.md](docs/testing.md). You do not need it to start.
