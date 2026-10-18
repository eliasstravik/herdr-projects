# How the tests work

This page is for people who work on the code of herdr-projects. It tells how the one test command, `npm run check`,
works, and why it works that way.

For the commands to run, see [run the tests](run-the-tests.md). What to run, and when, is in
[AGENTS.md](../../AGENTS.md).

## What the check does

One command runs the tests: `npm run check`. It builds the program, checks the rules of the repo and runs the tests.
The tests run against a fake Herdr and scripted fake agents. There are no model calls, no real board and no real
Herdr.

The tests are in `tests/`, one file for each area. What the files share is in `tests/common/`. All the files are one
test program (`tests/main.rs`, see [one test program](#one-test-program)). Small tests of one function are beside that
function in the code (`#[cfg(test)]`, in `src/` and `crates/`).

## One command for everyone

All who run tests use the same command: you, an agent at work, the reviewer, and the check of the board (`check` in
[`.herdr-projects.json`](../../.herdr-projects.json)).

- `npm test` is the same command. Both run `scripts/check.sh`. Only the name comes from Node. Nothing else of Node is
  used.
- `.herdr-superwork.json` holds the same command for the board that works on this repo until its machines run
  herdr-projects. herdr-superwork is the earlier name of this program, and that board reads only the file with that
  name.

## The steps of the check

The check does these steps in this order. It stops at the first step that fails.

1. **It builds the program** with the `quick` profile (in `Cargo.toml`). This profile builds the program's own code
   with no optimisations and no debug data. Thus a build after a small change is quick. The libraries that the
   program uses are a little optimised. They are built one time (see
   [the build cache and the warm start](testing-build-cache.md)). The release build, for the files that people
   install, uses another profile (see [install details](../reference/install-details.md)).
2. **It writes the made tables** if they are out of date (`herdr-projects docs`). The made tables are the keys in
   `docs/keys.md`, the settings in `docs/settings.md`, and the map in your own folder. The check tells which tables it
   wrote. You must commit the tables.
3. **It checks the rules of the repo** (`herdr-projects docs --guard`, which `tests/guard.rs` also runs):
   - No source file has more than 300 lines.
   - Each source file starts with a comment that tells what the file holds.
   - The chord of a key is written only in `crates/flow/src/board/keys.rs`.
   - No made table is out of date.
4. **It runs the tests** that the names you gave pick, in several processes at once (`scripts/run-tests.sh`, see
   [several processes at once](#several-processes-at-once)).

The check compiles through the build cache that all worktrees share. It uses the share of the cores of this task, not
all the cores (see [the build cache](testing-build-cache.md) and [its share of the cores](#its-share-of-the-cores)).

The check of the old program had to find names that the code used but did not define. It also had to find the test
files that a change reached through their imports. Now the compiler does the first job: such a file does not build. A
build is also one whole, so there are no separate test files to pick.

## The check remembers what passed

The check remembers a run that passed, for the exact code that it passed on. The exact code is these three things:

- the git tree of the folder as it is now, with uncommitted changes and new files;
- the full version of the compiler (`rustc -V`);
- the names that the run got.

If you ask again for the same code, the check answers at once and runs nothing. It says
`The check already passed for this exact code.`

- The results are kept outside the worktrees (`~/.cache/herdr-projects-check`). Thus all worktrees share them. The
  reviewer and the check of the board, on the worker's commit, use the worker's run that passed.
- A commit changes no file, so it changes nothing here. Any edit changes the code.
- The check never remembers a run that failed.
- The check remembers a run with names as that run only. For example, if `npm run check textbox` passes, that tells
  nothing about all the tests.
- `--fresh` runs the check again in all conditions.

## The shorter check of a fix round

On a later round of a task, the board runs `npm run check -- --since <commit>`, not the whole check. A later round
starts when a review or a failed check sends the work back. The board does this only where the checks of GitHub guard
the merge. The setting is `check.again` in [`.herdr-projects.json`](../../.herdr-projects.json). When the board runs
it is in [shorter checks on fix rounds](../explanation/shorter-checks.md). The commit is the commit that the whole
check last passed on.

The shorter check builds, writes the made tables and checks the rules, as the whole check does. Then it runs all the
tests beside the code, and the doc tests. It also runs the test files that the change since that commit reaches. The
change includes files committed since, files not committed yet, and new files. `scripts/changed-tests.sh <commit>`
picks the test files, for each changed file:

| Changed | The test files it reaches |
| --- | --- |
| A test file (`tests/sync.rs`) | That file. |
| A module that the files share (`tests/far/`) | The files that use it (`crate::far`). |
| A kept frame (`tests/frames/`) | `board_frames`. |
| A file of the program (`crates/flow/src/sync/conflict.rs`) | The files whose names hold its name or its folder's name as a word (`sync_conflict`, `sync`, `sync_behind`), and the files that write the path of its module (`sync::conflict`). A `mod.rs` goes by its folder. |
| A page (`docs/keys.md`, `README.md`) | The files that write its name, and `guard`. |

**All the tests run** if the script cannot tell. It then says why: `Every test runs: …`. This occurs for:

- `tests/main.rs`, `tests/common/` and `tests/fixtures/` (all the tests use them);
- `src/lib.rs`, `src/main.rs` and the root files of the crates (`src/sw_*.rs`);
- a file of the program that no test file is named by, or writes the path of;
- any other file (`Cargo.toml`, `crates/`, `scripts/`, `.github/`);
- a commit that is not on the branch;
- a change that reaches all the test files.

The script picks by names, not by what the tests really run. Thus a change can break a test that the script did not
pick. For this reason, the board runs the shorter check only where GitHub runs the whole check on the pull request
before anything merges.

- The check remembers a shorter check that passed as itself, with its commit.
- A shorter check that ran all the tests also counts as the whole check.
- If the whole check passed on this exact code, the shorter check answers at once.

## One test program

Each file in `tests/` is a module of one test program, `tests/main.rs` (`autotests = false` in `Cargo.toml`). This is
the usual way in Rust. Cargo compiles and links the tests one time. Before, there was one program for each file. That
meant 191 compiles, each of which linked the whole library again, and about 2.3 GB of programs in each worktree.

- A test file is still the file of one area.
- What the files share (`common`, `far`, `review_kit`, `move_home`) is declared one time in `tests/main.rs`. A file
  gets it with `use crate::common;`.
- **A new test file is one `mod` line in `tests/main.rs`**, in its place in the list. The guard fails a file that is
  not in the list, because cargo would never build or run it.

All the tests run side by side in that one process. Thus a change that a test makes to the process affects all the
tests:

- The environment. A world takes its turn (see [light tests](testing-light-tests.md)). A test that sets the
  environment without a world takes the same turn, with `common::one_at_a_time()`.
- The working folder. A world leaves its folder before the folder is removed.

## Several processes at once

Most tests make a world, and in one process the worlds take turns. One test program, run as cargo runs it, took about
90 seconds, on 4 threads or on 16 threads. Thus the check runs it as several processes at once
(`scripts/run-tests.sh`):

1. The script builds the test programs (`cargo test --no-run`).
2. It lists the tests in each program that the names pick.
3. It gives the tests in turn to the processes, runs them all, and tells how many passed.

If a process fails, the script prints all of its output, as `cargo test` would print it. Each process has its own
worlds (the folder of a world has the name of the process). The worlds take turns only inside one process.

**How many processes:** two times the share of the cores of this task (see
[its share of the cores](#its-share-of-the-cores)). The reason: a process whose tests take turns uses about half a
core. On 16 cores and five agents, that is 8 processes, and the whole check takes about 15 seconds. Five checks at
once take about 20 seconds each. `HERDR_PROJECTS_TEST_PROCS` sets another number. With 1, all the tests run in one
process, as before.

## Builds take turns

Cargo lets only one build at a time use the `target/` folder of a folder. A second build says
`Blocking waiting for file lock` and continues when the first build is done. Each worktree has its own `target/`, so
this occurs only inside one folder. There are no other turns and no slots: one run is one build and one run of the
tests.

## Its share of the cores

The build and the tests of a task use the number of cores of the machine divided by `board.agents`, rounded up.
`board.agents` is how many agents work at the same time. If it is not set, it is 4. `scripts/share-cores.sh` sets
`CARGO_BUILD_JOBS` and `RUST_TEST_THREADS`, for the check and for the setup. Thus tasks that work at the same time
share the machine, and no task asks for all of it. You can set one of these variables yourself to use another number.
A machine with no settings file (CI, a fresh clone) uses 4.

If the check stops early, it tells why on stderr. It does this also when the step that stopped it said nothing. An
example is `The check stopped while building the program (exit 101).` `tests/check_script.rs` starts the check in an
empty home folder, with no settings and no build cache. Thus the check always starts correctly on a fresh machine.

## The check tells the board where it is

When the board runs the check, it gives the name of a file in `HERDR_PROJECTS_PROGRESS`. The command writes its
progress in that file: how many of its three steps it has done (`1/3 steps`), or that it knew the answer at once. The
board shows this on the task (see [the progress of the check](../explanation/check-progress.md)).

## No time budget

No test and no run has a time budget. Nothing is measured against the clock. The old check had time budgets for each
file, and it lowered its own priority. The reason was that each test file was a program of its own that started more
programs. Here, a test is a function in a test program, and the light tests start nothing. Thus:

- Never wait for a quiet machine.
- Never cut a test to fit a time.

## Related pages

- [Run the tests](run-the-tests.md)
- [Light tests](testing-light-tests.md)
- [The build cache and the warm start](testing-build-cache.md)
- [Crates](crates.md)
- [Linking with mold](linking.md)
- [Shorter checks on fix rounds](../explanation/shorter-checks.md)
- [The progress of the check](../explanation/check-progress.md)
- [AGENTS.md](../../AGENTS.md)
