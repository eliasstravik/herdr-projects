# The build cache and the warm start

This page tells how the check and the workspace setup share compiled code between worktrees, and why. It is for people
who work on the code of herdr-projects.

## One build cache for every worktree

The libraries that the program uses are the same in every worktree. Thus they are compiled one time for each machine
and kept in one cache. The cache is [sccache](https://github.com/mozilla/sccache), one pinned version, checked against
its published sha256.

- The workspace setup (`scripts/setup.sh`, `workspace.setup` in `.herdr-projects.json`) installs sccache with
  `scripts/build-cache.sh`, and builds through it. If sccache is there, the check compiles through it.
- sccache is in `~/.cache/herdr-projects`, with its cache (2 GB at most, the oldest is removed first). It has a server
  of its own, a socket in that folder. Thus it does not touch another sccache of yours.
- This folder is outside every worktree. Thus, when you remove a worktree (`workspace.cleanup`), only the `target/` of
  that worktree is removed. Nothing that is shared is removed.
- If sccache is not installed, the builds continue as before, each worktree alone. Examples are CI (which keeps its
  own cache), another platform, and no network.
- `HERDR_PROJECTS_NO_CACHE=1` turns the cache off. A `RUSTC_WRAPPER` of your own has priority.
- The builds link with mold where mold is installed (see [linking with mold](linking.md)).

### What the cache keeps

The cache keeps what a task does not change: the libraries. A fresh worktree finds most of them in the cache. It builds
a few again, which takes a few seconds. These are the libraries that write code into the worktree's own `target/`
(their build scripts), and the compiler macros. sccache never keeps these.

The program's own code is not shared, because it is different in every task. The compiler also keeps its own notes on
that code in each `target/` (incremental compilation). These notes make the next build after a small edit quick. A
cache cannot take these notes.

### Why not one shared `target/`

A shared `target/` folder in place of the cache was not used, for two reasons:

- Cargo lets only one build at a time into a `target/`. Thus tasks would wait for each other.
- The program that cargo builds for the tests is one file at one place in `target/`. Two tasks would write over it
  while the other task tests with it.

## A new worktree starts warm

The cache keeps the libraries, not the program's own code. A build of the program's own code alone took about 35
seconds. Thus the setup does not start the build of a fresh worktree from nothing. `scripts/warm-start.sh` copies in
the last kept build of the main line (`target/quick`). Cargo builds again only what changed since that build.

- **Which build is kept.** After its build, a worktree keeps its own build as the start for the next worktree, if all
  these conditions are true:
  - its commit is on the main line;
  - its commit is newer than the kept build;
  - nothing changed in the worktree before or during the build.
- **Measured** on a 16-core machine with ten agents at once (two cores for each build). The setup of a fresh worktree
  took 37 seconds before. Now it takes a little more than one second. The first worktree after the main line moved on
  builds the program again from the notes of the compiler. This takes about 15 seconds. That worktree keeps its build
  for the worktrees after it.
- **Where.** The kept builds are in `~/.cache/herdr-projects/warm/<repo>/`, one folder for each build. `current` names
  the newest. The setup makes a new build beside the old one, and then names it. Thus worktrees that start at the
  same moment each copy a whole build. An old build is removed 30 minutes after it stopped being current.
- **The copy.** Where the file system can, the copy is a clone, which uses no extra space (APFS, Btrfs, XFS).
  Elsewhere it is a plain copy (ext4: about 500 MB for each kept build). The copy never uses hard links, because cargo
  writes into some of its files in place.
- **Timestamps.** Cargo finds what is out of date by the time when files changed. Every file of a fresh worktree is
  new. Thus the setup gives an old time to each file whose content is the same as at the commit of the kept build.
  Cargo then takes these files as built. The other files keep their time, and cargo builds them again. These are files
  changed on the main line since that commit, or changed in the folder.
- **Folder paths.** Cargo also takes as built a part that has the folder it was built in written into it
  (`env!("CARGO_MANIFEST_DIR")`). Here that folder would be another worktree. Thus the copy builds such a part again.
  The program finds its own repo when it runs (`docs::root`), not when it is built.
- **Crates.** The program is several crates (see [crates](crates.md)). The code of each crate is in its own folder.
  Thus cargo names that code from the folder of the crate. If no file of a crate changed since the kept build, cargo
  takes the whole crate as built.
- **Not copied.** The setup does not copy a build made by another compiler, or a build that grew past 2 GB (cargo
  never removes the libraries of older versions). That worktree builds from nothing, and the setup keeps its build in
  place of the old one.
- `HERDR_PROJECTS_NO_WARM=1` turns the warm start off.

## Related pages

- [How the tests work](testing.md)
- [Run the tests](run-the-tests.md)
- [Crates](crates.md)
- [Linking with mold](linking.md)
