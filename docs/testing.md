# Tests

**For people working on this code. One command runs the tests: `npm run check`.** It builds the program, checks the
repo's rules and runs the tests, against a fake Herdr and scripted fake agents: no model calls, no real board, no real
Herdr. What to run, and when, is in [AGENTS.md](../AGENTS.md). This page says how the command works, and why.

| To run | Command |
|---|---|
| Everything | `npm run check`, or `npm run check -- --all` |
| One area: its test files (`tests/board_panel.rs`, `tests/board_panel_*.rs`) | `npm run check board-panel`, `npm run check gate sync` |
| Only the tests whose names hold a word | `npm run check textbox` |
| Again, although it passed | add `--fresh` |
| Only what a change since a commit reaches (the board's check on a fix round) | `npm run check -- --since <commit>` |

The tests are in `tests/`, one file per area, with what they share in `tests/common/`, all of them one test program
(`tests/main.rs`, [below](#one-test-program)). Small tests of one function sit beside it in the code (`#[cfg(test)]`, in
`src/` and `crates/`).

## One command

Everything that runs tests uses the same command: you, an agent at work, the reviewer, and the board's own check (`check`
in [`.herdr-projects.json`](../.herdr-projects.json)).

- `npm test` is the same command. Both run `scripts/check.sh`; nothing of Node is used but the name.
- `.herdr-superwork.json` says the same, for the board that works on this repo until its machines run herdr-projects (this
  program's name before): it reads only that name.

**Picking tests by name.** A name is first an area: `board-panel` (or `board_panel`) runs `tests/board_panel.rs` and every
`tests/board_panel_*.rs`, exactly those (not `web_board_panel.rs`). A name that is no area is matched against each test's
own name, as `cargo test` matches it: a test in `tests/` is named by its file and its function
(`board_hints::the_hint_line_names_the_task_once_…`), and one beside the code by its module path. So
`npm run check textbox` runs the tests that say `textbox`, wherever they are. A word that matches no test runs none, and
passes.

**What it does**, in this order, stopping at the first that fails:

1. **Builds the program** with the `quick` profile (`Cargo.toml`): the program's own code with no optimisations and no
   debug data, quick to build again after a small change, and the libraries it uses a little optimised (they are built
   once, [below](#one-build-cache-for-every-worktree)). The release build, for the files people install, is another
   profile ([installing](install.md)).
2. **Writes the made tables** when they are out of date (`herdr-projects docs`): the keys in `docs/keys.md`, the settings
   in `docs/settings.md`, and the map in your own folder. It says which it wrote: commit the tables.
3. **Checks the repo's own rules** (`herdr-projects docs --guard`, which `tests/guard.rs` runs too): no source file past
   300 lines, every source file starts with a comment that says what it holds, a key's chord written only in
   `crates/flow/src/board/keys.rs`, and no made table out of date.
4. **Runs the tests**, with the names you gave, in several processes at once (`scripts/run-tests.sh`,
   [below](#several-processes-at-once)).

It compiles through the build cache every worktree shares, and takes this task's share of the cores, not all of them
(both below).

The old program's check had to find names used and never defined, and which test files a change reached through its
imports. The compiler does the first now (such a file does not build), and a build is one whole: there are no separate
test files to pick.

### It remembers what passed

A run that passed is remembered for the exact code it passed on: the git tree of the folder as it stands (uncommitted
changes and new files included), the compiler's full version (`rustc -V`), and the names it was given. Asked again for
the same, it answers at once (`The check already passed for this exact code.`) and runs nothing.

- The results are kept outside the worktrees (`~/.cache/herdr-projects-check`), so every worktree shares them: the
  reviewer and the board's check, on the worker's commit, reuse the worker's passing run.
- Committing changes no file, so it changes nothing here. Any edit does.
- A failed run is never remembered.
- A run with names is remembered as that run only: `npm run check textbox` passing says nothing about the whole.
- `--fresh` runs regardless.

### The shorter check of a fix round

On a later round of a task (a review or a failed check sent it back), the board runs
`npm run check -- --since <commit>` instead of the whole check, where GitHub's checks still guard the merge:
`check.again` in [`.herdr-projects.json`](../.herdr-projects.json), and when the board does so in
[shorter checks on fix rounds](shorter-checks.md). The commit is the one the whole check last passed on.

It builds, writes the made tables and checks the rules as the whole check does. Then it runs every test beside the code
(and the doc tests), and the test files the change since that commit reaches: committed since, not committed yet, and new
files. `scripts/changed-tests.sh <commit>` picks them, for each file changed:

| Changed | The test files it reaches |
|---|---|
| a test file (`tests/sync.rs`) | that file |
| a module the files share (`tests/far/`) | the files that use it (`crate::far`) |
| a kept frame (`tests/frames/`) | `board_frames` |
| a file of the program (`crates/flow/src/sync/conflict.rs`) | those whose names hold its name or its folder's as a word (`sync_conflict`, `sync`, `sync_behind`), and those that write its module's path (`sync::conflict`); a `mod.rs` goes by its folder |
| a page (`docs/keys.md`, `README.md`) | those that write its name, and `guard` |

**Every test runs** (it says why: `Every test runs: …`) when it cannot tell: for `tests/main.rs`, `tests/common/` and
`tests/fixtures/` (every test uses them); `src/lib.rs`, `src/main.rs` and the crates' root files (`src/sw_*.rs`); a file
of the program that no test file is named by or writes the path of; any other file (`Cargo.toml`, `crates/`, `scripts/`,
`.github/`); a commit that is not on the branch; and a change that reaches every test file anyway.

This picks by names, not by what the tests really run: a change can break a test it did not pick. That is why the board
runs it only where GitHub runs the whole check on the pull request before anything merges.

- A shorter check that passed is remembered as itself (with its commit).
- One that ran every test counts as the whole check too.
- The whole check having passed on this exact code answers it at once.

### One test program

Every file in `tests/` is a module of one test program, `tests/main.rs` (`autotests = false` in `Cargo.toml`), the usual
way in Rust. Cargo compiles and links the tests once. (A program per file meant 191 compiles, each linking the whole
library again, and about 2.3 GB of programs in every worktree.)

- A test file is still the file of one area.
- What the files share (`common`, `far`, `review_kit`, `move_home`) is declared once in `tests/main.rs`. A file reaches it
  as `use crate::common;`.
- **A new test file is one `mod` line in `tests/main.rs`**, in its place in the list. The guard fails a file left out,
  which would never be built or run.

All the tests run side by side in that one process, so whatever a test changes of the process is everyone's: the
environment (a world takes its turn, below; a test that sets it without a world takes the same turn,
`common::one_at_a_time()`) and the working folder (a world leaves its folder before it is removed).

### Several processes at once

Most tests make a world, and in one process worlds take turns: one test program run as cargo runs it took about 90
seconds, on 4 threads or on 16 alike. So the check runs it as several processes at once (`scripts/run-tests.sh`):

1. it builds the test programs (`cargo test --no-run`);
2. it lists the tests each holds that the names pick;
3. it deals them out in turn to the processes, runs them all, and says how many passed.

A process that failed is printed whole, as cargo test would have printed it. Each process has worlds of its own (a
world's folder is named by the process), and the turns are only within it.

**How many:** twice this task's share of the cores ([below](#its-share-of-the-cores)), since a process whose tests take
turns uses about half a core. On 16 cores and five agents that is 8 processes, and the whole check about 15 seconds; five
checks at once each take about 20. `HERDR_PROJECTS_TEST_PROCS` sets another number (1: everything in one process, as
before).

### Builds take turns

Cargo lets one build at a time use a folder's `target/`: a second one says `Blocking waiting for file lock` and goes on
when the first is done. Each worktree has a `target/` of its own, so this happens only within one folder. There are no
other turns and no slots: one run is one build and one run of the tests.

### One build cache for every worktree

The libraries the program uses are the same in every worktree, so they are compiled once per machine and kept in one
cache: [sccache](https://github.com/mozilla/sccache), one pinned version checked against its published sha256.

- The workspace setup (`scripts/setup.sh`, `workspace.setup` in `.herdr-projects.json`) installs it with
  `scripts/build-cache.sh` and builds through it. The check compiles through it when it is there.
- It lives in `~/.cache/herdr-projects` with its cache (2 GB at most, oldest dropped first) and a server of its own (a
  socket there, so another sccache of yours is left alone). That is outside every worktree, so removing a worktree
  (`workspace.cleanup`) frees that worktree's `target/` and nothing shared.
- Where it is not installed (CI, which keeps its own cache; another platform; no network), builds go on as before, each
  worktree alone.
- `HERDR_PROJECTS_NO_CACHE=1` turns it off. A `RUSTC_WRAPPER` of your own wins.
- The builds link with mold where it is installed ([linking](linking.md)).

What it keeps is what a task does not change: the libraries. A fresh worktree finds most of them there. A few it builds
again, because they write code into that worktree's own `target/` (their build scripts) or are compiler macros, which
sccache never keeps: a few seconds. The program's own code is not shared: it differs in every task, and the compiler
keeps its own notes on it in each `target/` (incremental compilation), which makes the next build after a small edit
quick; a cache cannot take those.

A shared `target/` instead of the cache was left out: cargo lets one build at a time into a `target/`, so tasks would
wait on each other, and the program it builds for the tests is one file at one place there, which two tasks would
overwrite under each other's tests.

### A new worktree starts warm

The cache keeps the libraries, not the program's own code, and a build of that alone took about 35 seconds. So the setup
does not start a fresh worktree's build from nothing: `scripts/warm-start.sh` copies in the last kept build of the main
line (`target/quick`), and only what changed since is built again.

- **Which build is kept:** after its build, a worktree whose commit is on the main line and newer than the kept build,
  with nothing changed in it before or during the build, keeps its own as the next one's start.
- **Measured** on a 16-core machine with ten agents at once (two cores a build): a fresh worktree's setup took 37
  seconds before, a little over one now. The first worktree after the main line moved on builds the program again from
  the compiler's notes, about 15 seconds, and keeps that build for the ones after it.
- **Where:** `~/.cache/herdr-projects/warm/<repo>/`, one folder each, with `current` naming the newest. A new one is made
  beside the old and then named, so worktrees set up at the same moment each copy a whole build. An old one goes 30
  minutes after it stopped being current.
- **The copy** is a clone where the file system can (APFS, Btrfs, XFS: no extra space), a plain copy elsewhere (ext4:
  about 500 MB a kept build); never hard links, since cargo writes into some of its files in place.
- **Timestamps:** cargo judges what is out of date by when files changed, and every file of a fresh worktree is new. The
  files whose content is the kept build's commit's get an old time, so cargo takes them as built; the rest (changed on
  the main line since, or in the folder) keep theirs and are built again.
- **Folder paths:** cargo also takes as built what has the folder it was built in written into it
  (`env!("CARGO_MANIFEST_DIR")`), which here would be another worktree. The copy builds such a part again, and the
  program finds its own repo when it runs (`docs::root`), not when it is built.
- **Crates:** the program is several crates ([crates](crates.md)), each with its code in its own folder, so cargo names
  that code from the crate's folder: a crate none of whose files changed since the kept build is taken as built whole.
- **Not copied:** a build by another compiler, or one grown past 2 GB (cargo never removes the libraries of older
  versions). That worktree builds from nothing, and its build is kept instead.
- `HERDR_PROJECTS_NO_WARM=1` turns it off.

### Its share of the cores

A task's build and tests take the machine's cores divided by `board.agents` (how many agents work at once; 4 when it is
not set), rounded up. `scripts/share-cores.sh` sets `CARGO_BUILD_JOBS` and `RUST_TEST_THREADS`, for the check and the
setup. Tasks working at once then share the machine instead of each asking for all of it. Set either yourself to use
another number. A machine with no settings file (CI, a fresh clone) takes the 4.

A check that stops early says why on stderr (`The check stopped while building the program (exit 101).`), also when the
step that stopped it said nothing. `tests/check_script.rs` starts the check in an empty home, with no settings and no
build cache, so a fresh machine always gets past its start.

### It tells the board where it is

When the board runs the check, it names a file in `HERDR_PROJECTS_PROGRESS`, and the command writes its progress there:
which of its three steps it has done (`1/3 steps`), or that the answer was known at once. The board shows that on the
task: [the check's progress](check-progress.md).

### No time budget

No test and no run has a time budget, and nothing is measured against the clock. The old check had budgets per file and
lowered its own priority, because every test file was a program of its own that started more programs. A test here is a
function in a test program, and the light ones start nothing. So: never wait for a quiet machine, and never cut a test to
fit a time.

## Light tests

What makes a test slow is starting programs and waiting. A new test does neither:

- **It calls the code**, in the test's own process. `world()` in `tests/common/mod.rs` gives it a board folder of its own,
  a git repo and a fake Herdr. The board's code still builds the same Herdr command and reads the same answer and exit
  status, but no program is started: `herdr::cli::set_fake` hands the call to the fake, which answers in the test's own
  thread. `gh` is faked the same way (`flow::git::set_fake_gh`, `tests/common/gh.rs`).
- **It does not sleep, and does not depend on the machine's speed.** `world()` sets the code's own waits to zero. The
  clock is `sw::clock`: `set` stands it at a moment and `advance` moves it on, on the test's thread only. A moment in the
  past or the future is written into the data, not waited for. Nothing compares against a duration it measured.
- **The board's background jobs run when the test says.** What would be a detached runner (delegating, the finish line, a
  merge) is put in a list instead (`HERDR_PROJECTS_INLINE`). `settle(id)` runs that list, then lets the hook look at the
  agent's pane, round after round until nothing moves. `settle_all(id)` does the same for a task done several times at
  once.
- **It shares set-up inside one test**, rather than building a new world for every small assertion: a world is a git repo
  and a first commit. `to_approval(&w, title)` is the scene most finish-line tests start from.
- **git is the one program still started**, because the board's work is git work. Ask for each fact once.

A world sets the process's environment, so the tests that make a world take turns, in all the files (a lock in
`tests/common/mod.rs`); the others run beside them. The check runs several processes at once to get past that (above).

**A wait the board has to make is a number a test can make short.** The board's own waits are zero in a world, and a test
of a real program gives it the short one it needs (`HERDR_PROJECTS_STAMP_MS`: how often an open board looks for a new
version of itself; `HERDR_PROJECTS_TICK_MS`: how often a running check is looked at). Never wait a fixed time for
something that says when it is done: a real terminal is read until its frame is drawn.

### The stand-ins

- **The fake Herdr** (`tests/common/herdr.rs`, with `panes.rs`, `agents.rs` and `worktrees.rs`): its whole world is one
  JSON record (spaces, tabs, panes, their agents, every call it was asked), which a test reads (`w.state()`, `calls(&w)`)
  or changes (`w.patch`, `set_status`). Its worktrees are made with git itself.
- **Scripted agents** (`tests/common/agents.rs`): a test gives each role its turns,
  `w.script(json!({"worker": [step, …], "reviewer": […], "picker": […], "coordinator": […]}))`. Each prompt plays the
  role's next step: it writes files, commits, and writes the closing note, the plan, the verdict or the comparison where
  the board tells an agent to. A step may end its turn with a recorded ending of a real agent's conversation
  (`tests/fixtures/endings`, `tests/common/endings.rs`) and run the agent's own turn-end report.
- **Notes written between the board's jobs.** A real agent writes its closing note after the board armed the turn it
  ends. The fake writes the notes of the turns that ended so far between the board's jobs (`flush()`, which `settle`
  calls), stamped after that moment, however slow the machine running the test.
- **The rows every screen test draws** (`tests/common/fixtures.rs`): one task in each state the board shows, with the
  long, wide and odd text that breaks a layout. A new state is one new row there, and every screen test draws it.

## Four kinds of tests

- **The board's state, updated.** `common::board::open(&w.root, rows)` opens the real board on a world, without a
  terminal. `press(&["alt+t", "esc", "ctrl+l"])` and `typed("…")` go through the board's one `update`, what they start
  runs at once, and `text()`, `screen(cols)`, `chat(cols)` and `draft(cols)` read the screen as plain text. Most tests of
  the board are of this kind.
- **Whole frames** (`tests/board_frames.rs`): the board drawn by the terminal drawer into cells, at ten sizes from 10×8 to
  200×60, each with the list, the task beside it, the whole task, the `?` menu and a draft. Each is compared with the
  frame kept in `tests/frames/`, once as text and once with each cell's colour as a letter (with a legend). A missing
  frame is written; `UPDATE_FRAMES=1` writes them all again, for a change to what the board looks like. Look at what
  changed before you commit it. Every frame also holds to the rules that need no picture: no line over the width, the
  hint line last.
- **The board as data for the web page** (`tests/web_board.rs`): `common::board::web(&w, call, args)` asks the web board
  what the page would ask, on the same world: the same list, groups, marks and words as the terminal board, and its
  buttons pressing the board's own keys.
- **A real terminal**, a few: `tests/real_terminal.rs` starts the built program in a pseudo-terminal and drives it as a
  terminal would: it starts and draws, takes a burst of keys, resizes cleanly, and on closing gives the terminal back
  exactly as it was.

**A test never reads or writes your real board.** `world()` gives it a home folder of its own (`HOME`,
`HERDR_PROJECTS_HOME`), takes Herdr's own variables (a pane's id, its socket), `TMUX` and `NO_COLOR` out of the
environment, and points Herdr at a socket of its own. When you run the program by hand while working on it, set
`HERDR_PROJECTS_HOME` to a scratch folder first: a new version may convert what it finds in the real one.

**A real program is started only where the program itself is what is tested.** Those tests say so in their name
(`real_programs_…`): the command line (`tests/cli.rs`), another machine's door (`tests/remote.rs`) and the real terminal
(`tests/real_terminal.rs`). They run the program cargo built for the tests, inside the test's world, with no Herdr to
reach. A repo's own `setup`, `check` and `teardown` commands are shell commands by nature, and run as such in the tests
about them.

## Looking at the real thing

Tests read the board as text and as frames. A real terminal or a browser screenshot is only for a task that changes what
the board or the review page looks like. For any other task, the tests are the evidence.
