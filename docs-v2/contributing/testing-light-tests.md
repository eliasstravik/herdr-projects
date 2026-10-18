# Light tests

This page tells how a test of herdr-projects runs without real programs, and what stands in for them. Use it when you
write a new test.

## What makes a test light

A test is slow if it starts programs and waits. A new test does neither.

- **It calls the code** in the test's own process. `world()` in `tests/common/mod.rs` gives the test a board folder of
  its own, a git repo and a fake Herdr. The code of the board still builds the same Herdr command, and reads the same
  answer and exit status. But no program starts: `herdr::cli::set_fake` gives the call to the fake, which answers in
  the test's own thread. `gh` is faked in the same way (`flow::git::set_fake_gh`, `tests/common/gh.rs`).
- **It does not sleep, and it does not depend on the speed of the machine.** `world()` sets the waits of the code to
  zero. The clock is `sw::clock`: `set` stops it at a moment, and `advance` moves it on. Both work only on the thread of
  the test. A moment in the past or the future is written into the data. The test does not wait for it. Nothing
  compares against a duration that it measured.
- **The background jobs of the board run when the test tells them to.** Some jobs would be a detached runner:
  delegation, the finish line, a merge. Such a job goes into a list (`HERDR_PROJECTS_INLINE`). `settle(id)` runs that
  list. Then it lets the hook look at the pane of the agent. It does this round after round, until nothing moves.
  `settle_all(id)` does the same for a task done several times at once.
- **It shares set-up inside one test.** Do not build a new world for each small assertion: a world is a git repo and a
  first commit. `to_approval(&w, title)` is the scene that most tests of the finish line start from.
- **git is the one program that still starts**, because the work of the board is git work. Ask for each fact one time.

A world sets the environment of the process. Thus the tests that make a world take turns, in all the files (a lock in
`tests/common/mod.rs`). The other tests run beside them. The check runs several processes at once to get past this
(see [how the tests work](testing.md#several-processes-at-once)).

**A wait that the board must make is a number that a test can make short.** In a world, the waits of the board are
zero. A test of a real program gives the wait the short value that it needs:

- `HERDR_PROJECTS_STAMP_MS`: how often an open board looks for a new version of itself.
- `HERDR_PROJECTS_TICK_MS`: how often the board looks at a running check.

Never wait a fixed time for something that tells when it is done. For example, a test reads a real terminal until its
frame is drawn.

## The stand-ins

- **The fake Herdr** (`tests/common/herdr.rs`, with `panes.rs`, `agents.rs` and `worktrees.rs`). Its whole world is
  one JSON record: spaces, tabs, panes, their agents, and each call that it got. A test reads the record (`w.state()`,
  `calls(&w)`) or changes it (`w.patch`, `set_status`). The fake makes its worktrees with git itself.
- **Scripted agents** (`tests/common/agents.rs`). A test gives each role its turns:
  `w.script(json!({"worker": [step, …], "reviewer": […], "picker": […], "coordinator": […]}))`. Each prompt plays the
  next step of the role. The step writes files and commits. It writes the closing note, the plan, the verdict or the
  comparison where the board tells an agent to. A step can end its turn with a recorded ending of the conversation of
  a real agent (`tests/fixtures/endings`, `tests/common/endings.rs`). It can then run the agent's own report at the end
  of the turn.
- **Notes written between the jobs of the board.** A real agent writes its closing note after the board armed the turn
  that the note ends. The fake writes the notes of the turns that ended so far between the jobs of the board
  (`flush()`, which `settle` calls). It stamps them after that moment, however slow the machine that runs the test is.
- **The rows that every screen test draws** (`tests/common/fixtures.rs`). There is one task in each state that the
  board shows, with long, wide and odd text that breaks a layout. A new state is one new row there, and every screen
  test draws it.

## Four kinds of tests

- **The state of the board, updated.** `common::board::open(&w.root, rows)` opens the real board on a world, without a
  terminal. `press(&["alt+t", "esc", "ctrl+l"])` and `typed("…")` go through the one `update` of the board. What they
  start runs at once. `text()`, `screen(cols)`, `chat(cols)` and `draft(cols)` read the screen as plain text. Most
  tests of the board are of this kind.
- **Whole frames** (`tests/board_frames.rs`). The terminal drawer draws the board into cells, at ten sizes from 10×8
  to 200×60. Each size has the list, the task beside it, the whole task, the `?` menu and a draft. The test compares
  each frame with the frame kept in `tests/frames/`. It compares one time as text, and one time with the colour of each
  cell as a letter (with a legend). If a frame is missing, the test writes it. `UPDATE_FRAMES=1` writes all the frames
  again, for a change to what the board looks like. Look at what changed before you commit it. Each frame must also
  obey the rules that need no picture: no line is wider than the width, and the hint line is last.
- **The board as data for the web page** (`tests/web_board.rs`). `common::board::web(&w, call, args)` asks the web
  board what the page would ask, on the same world. The answer has the same list, groups, marks and words as the
  terminal board. Its buttons press the board's own keys.
- **A real terminal**, in a few tests. `tests/real_terminal.rs` starts the built program in a pseudo-terminal, and
  drives it as a terminal would. The program starts and draws, takes a burst of keys and resizes cleanly. When it
  closes, it gives the terminal back exactly as it was.

## A test never touches your real board

A test never reads or writes your real board. `world()` does these things:

- It gives the test a home folder of its own (`HOME`, `HERDR_PROJECTS_HOME`).
- It removes the variables of Herdr (the id of a pane, its socket), `TMUX` and `NO_COLOR` from the environment.
- It points Herdr at a socket of its own.

## Real programs only where they are what is tested

A test starts a real program only where the program itself is what is tested. The names of these tests tell this
(`real_programs_…`). They are the command line (`tests/cli.rs`), the door of another machine (`tests/remote.rs`) and
the real terminal (`tests/real_terminal.rs`). They run the program that cargo built for the tests, inside the world of
the test, with no Herdr to reach.

The `setup`, `check` and `teardown` commands of a repo are shell commands by nature. The tests about them run them as
shell commands.

## Related pages

- [Run the tests](run-the-tests.md)
- [How the tests work](testing.md)
- [The build cache and the warm start](testing-build-cache.md)
- [AGENTS.md](../../AGENTS.md)
