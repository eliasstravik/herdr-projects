# Run the tests

This page tells you which commands to run to test a change to herdr-projects, and in which order. For how the check
works, and why, see [how the tests work](testing.md).

## The commands

One command runs the tests: `npm run check`. `npm test` is the same command.

| To run | Command |
| --- | --- |
| All the tests | `npm run check`, or `npm run check -- --all` |
| One area: its test files (`tests/board_panel.rs`, `tests/board_panel_*.rs`) | `npm run check board-panel`, `npm run check gate sync` |
| Only the tests whose names hold a word | `npm run check textbox` |
| The tests again, although they passed | Add `--fresh`. |
| Only what a change since a commit reaches (the check of the board on a fix round) | `npm run check -- --since <commit>` |

## How a name picks tests

1. The check first reads a name as an area. For example, `board-panel` (or `board_panel`) runs `tests/board_panel.rs`
   and each `tests/board_panel_*.rs`. It runs exactly these files, not `web_board_panel.rs`.
2. If a name is not an area, the check compares it with the name of each test, as `cargo test` does. A test in
   `tests/` has the name of its file and its function (`board_hints::the_hint_line_names_the_task_once_…`). A test
   beside the code has the path of its module as its name.

Thus `npm run check textbox` runs the tests that hold `textbox`, in all places.

**Note:** A word that matches no test runs no test, and the check passes.

## Test a change

1. While you work, run the tests of the area that you changed, for example `npm run check board-panel`.
2. If the check wrote a made table (`docs/keys.md`, `docs/settings.md`), commit the table that it wrote.
3. If your task asks for it, run `npm run check -- --all` one time, last of all.

**Note:** Do not run the whole check after each edit. The board runs the whole check when you finish. It then
answers at once, because the check remembers what passed for this exact code.

If you see `Blocking waiting for file lock`, do nothing. Another build in the same folder is not finished yet. Your
build continues when it is done.

If the check stops early, read its message on stderr. An example of this message:
`The check stopped while building the program (exit 101).`

## Add a test file

1. Make the new file in `tests/`.
2. In the new file, write `use crate::common;`. Do not write `mod common;`.
3. Add one `mod` line for the file in `tests/main.rs`, in its place in the list.

**CAUTION:** If you do not add the `mod` line, cargo never builds or runs the file. The guard of the check fails.

For how to write a light test, see [light tests](testing-light-tests.md).

## Write the kept frames again

Do this only for a change to what the board looks like. `UPDATE_FRAMES=1` writes all the frames again (see
[four kinds of tests](testing-light-tests.md#four-kinds-of-tests)). The command below is the one that
[AGENTS.md](../../AGENTS.md) gives.

1. Run `UPDATE_FRAMES=1 npm run check frames`.
2. Look at what changed in `tests/frames/`.
3. Commit the changed frames.

## Run the program by hand

**CAUTION:** A new version can convert what it finds in your real board folder. Set `HERDR_PROJECTS_HOME` before you
start the program by hand.

1. Make a scratch folder.
2. Set `HERDR_PROJECTS_HOME` to the scratch folder.
3. Run the program.

## Change how the check runs

The check works with no variables set. You can set these variables to change it:

| Variable | What it does |
| --- | --- |
| `HERDR_PROJECTS_TEST_PROCS` | The number of test processes. With 1, all the tests run in one process. |
| `CARGO_BUILD_JOBS`, `RUST_TEST_THREADS` | If you set one, the check uses your number in place of the share of the cores. |
| `HERDR_PROJECTS_NO_CACHE=1` | Turns the shared build cache off. A `RUSTC_WRAPPER` of your own has priority. |
| `HERDR_PROJECTS_NO_WARM=1` | Turns the warm start of a new worktree off. |
| `HERDR_PROJECTS_NO_MOLD=1` | Turns mold off. This rebuilds nothing. |

## Look at the real thing

The tests read the board as text and as frames. Use a real terminal or a browser screenshot only for a task that
changes what the board or the review page looks like. For all other tasks, the tests are the evidence.

## Related pages

- [How the tests work](testing.md)
- [Light tests](testing-light-tests.md)
- [The build cache and the warm start](testing-build-cache.md)
- [Linking with mold](linking.md)
- [Crates](crates.md)
- [AGENTS.md](../../AGENTS.md)
