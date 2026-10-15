# Linking

**On Linux, this repo's builds link with [mold](https://github.com/rui314/mold), the fastest linker there is.** Where
mold is not installed, they link the way Rust does by itself. You do not need to install anything for a build to work.

Why it matters: every build after an edit links the program and the test program again, and both hold the whole
library. mold halves the link time, about 0.4 s saved per build.

`HERDR_PROJECTS_NO_MOLD=1` turns mold off. Switching it on or off rebuilds nothing.

## How it works

- **The flags.** `.cargo/config.toml` gives Linux builds (x86_64 and aarch64) `-Clink-arg=-Bscripts/ld/`. So the C
  compiler runs `scripts/ld/ld` as its `ld`, which is a link to `scripts/linker.sh`. This holds for every build in the
  repo: the check, the workspace setup, and your own `cargo build` or `cargo test`.
- **x86_64 Linux** also gets `-Clinker-features=-lld`. There Rust asks the C compiler for its own lld by default, and
  this flag stops it.
- **ARM Linux (aarch64) gets only the `-B` flag.** `-Clinker-features` is unstable there (as on macOS; it is stable on
  x86_64 Linux). With it, every build on stable Rust stops at once, saying the flag is unstable on the
  `aarch64-unknown-linux-gnu` target. It is not needed there: Rust does not ask for its lld on that target, so the C
  compiler runs `scripts/ld/` all the same and links with mold too.
- **The script.** `scripts/linker.sh` runs mold: one pinned version, checked against its published sha256, kept in
  `~/.cache/herdr-projects` beside the shared build cache. The workspace setup installs it
  (`sh scripts/linker.sh --install`, in `scripts/setup.sh`).
- **mold uses all the cores**, like Rust's lld. A link takes a fraction of a second. On this task's share of the cores
  ([testing](testing.md)), mold kept little of its gain.
- **Where mold is not installed** (GitHub CI, a fresh machine, no network, a platform with no mold build), the script
  runs the linker Rust would use: its own lld (`rust-lld`, which Rust puts first on the linker's PATH) on x86_64, the
  system's `ld` elsewhere (aarch64).
- **Release builds link as before.** A link whose files go under a `release` folder (`cargo build --release`, also
  with `--target`) uses that same default linker. The release files are built for musl targets, which
  `.cargo/config.toml` does not name.
- **macOS** keeps Apple's own linker (ld-prime, the default since Xcode 15). It is as fast as lld there, and mold has no
  macOS build.

## A test guards every platform

`tests/linking.rs` runs `rustc --print cfg` with the flags the config gives each platform: x86_64 and aarch64 Linux
(gnu and musl), macOS, and every target the config names. It fails when rustc rejects them or one is a `-Z` flag.
rustc complains before it needs that target's library, so the test catches a config that breaks ARM Linux on any
machine, CI's included.

## Why flags, and not a linker named in the config

Cargo remembers what it built by the settings it built it with. A linker named in the config
(`linker = "scripts/linker.sh"`) would be given as that worktree's whole path. Then every library would look out of
date in a build copied from another worktree (the warm start, [testing](testing.md)) and in the shared build cache,
which keys a compile by its arguments. The two flags are the same words in every worktree, so neither notices them.

The `-B` folder is relative, and the C compiler looks for it from where the compile runs: the repo itself, for the
program and its tests. The libraries' build scripts and macros compile in their own folders, where there is no
`scripts/ld/`. They link with the system's `ld`: small programs, built once.

## What it saves

Measured on vm-str (16 cores, Rust 1.98.1, other tasks building at the same time), after a one-line change to
`src/lib.rs`, building as the check does (the program, then the test programs), six times each. Each cell is the range
of one link's time:

| Link | Rust's own lld (before) | mold on the check's 2 cores | mold on all cores (as shipped) |
|---|---|---|---|
| The program (39 MB) | 0.22–0.27 s | 0.16–0.20 s | 0.09–0.12 s |
| The test program (`tests/main.rs`) | 0.37–0.45 s | 0.28–0.33 s | 0.14–0.19 s |
| The library's own tests | 0.12–0.16 s | 0.06–0.10 s | 0.05–0.09 s |
| The program with debug data (`dev`, 134 MB) | 0.39–0.51 s | — | 0.17–0.22 s |
| **All links after one edit** | **about 0.8 s** | about 0.6 s | **about 0.4 s** |

mold links in about half the time. The whole build hardly changes, though (about 2.7 s for the program and 6 s more for
the tests, and runs vary as much as the gain): compiling is nearly all of it, because Rust's lld was already fast.
