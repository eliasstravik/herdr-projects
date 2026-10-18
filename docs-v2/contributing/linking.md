# Linking with mold

This page tells how the builds of this repo link the program on Linux, and why. You do not have to install anything
for a build to work.

## What the builds use

On Linux, the builds of this repo link with [mold](https://github.com/rui314/mold), the fastest linker there is. If
mold is not installed, the builds link the way Rust links by itself.

Each build after an edit links the program and the test program again, and both hold the whole library. mold links in
half the time. This saves about 0.4 s for each build.

`HERDR_PROJECTS_NO_MOLD=1` turns mold off. If you turn mold on or off, cargo rebuilds nothing.

## How it works

- **The flags.** `.cargo/config.toml` gives Linux builds (x86_64 and aarch64) the flag `-Clink-arg=-Bscripts/ld/`.
  Thus the C compiler runs `scripts/ld/ld` as its `ld`. `scripts/ld/ld` is a link to `scripts/linker.sh`. This is true
  for every build in the repo: the check, the workspace setup, and your own `cargo build` or `cargo test`.
- **x86_64 Linux** also gets the flag `-Clinker-features=-lld`. On that target, Rust asks the C compiler for the lld of
  Rust by default. This flag stops that.
- **ARM Linux (aarch64) gets only the `-B` flag.** `-Clinker-features` is unstable on aarch64, as on macOS. It is
  stable on x86_64 Linux. With this flag, every build on stable Rust stops at once. The error says that the flag is
  unstable on the `aarch64-unknown-linux-gnu` target. The flag is not necessary there: on that target, Rust does not
  ask for its lld. Thus the C compiler runs `scripts/ld/` all the same, and it links with mold too.
- **The script.** `scripts/linker.sh` runs mold. It uses one pinned version of mold, which it checks against the
  published sha256. It keeps mold in `~/.cache/herdr-projects`, beside the shared build cache. The workspace setup
  installs mold (`sh scripts/linker.sh --install`, in `scripts/setup.sh`).
- **mold uses all the cores**, as the lld of Rust does. A link takes a fraction of a second. On the share of the cores
  of this task (see [how the tests work](testing.md#its-share-of-the-cores)), mold kept little of its gain.
- **If mold is not installed**, the script runs the linker that Rust would use. Examples are GitHub CI, a fresh
  machine, no network, and a platform with no mold build. On x86_64, this linker is the lld of Rust (`rust-lld`, which
  Rust puts first on the PATH of the linker). On other platforms (aarch64), it is the `ld` of the system.
- **Release builds link as before.** If the files of a link go under a `release` folder, the link uses the same
  default linker. This is true for `cargo build --release`, also with `--target`. The release files are built for musl
  targets, which `.cargo/config.toml` does not name.
- **macOS** keeps the linker of Apple (ld-prime, the default since Xcode 15). On macOS, ld-prime is as fast as lld,
  and mold has no macOS build.

## A test guards every platform

`tests/linking.rs` runs `rustc --print cfg` with the flags that the config gives each platform. The platforms are
x86_64 Linux and aarch64 Linux (gnu and musl), macOS, and every target that the config names. The test fails if rustc
rejects the flags, or if one flag is a `-Z` flag.

rustc complains before it needs the library of that target. Thus the test finds a config that breaks ARM Linux on any
machine, also on CI.

## Why flags, and not a linker named in the config

Cargo remembers what it built by the settings that it built it with. A linker named in the config
(`linker = "scripts/linker.sh"`) would go to cargo as the whole path of that worktree. Then every library would look
out of date in two places:

- in a build copied from another worktree (the warm start, see
  [the build cache and the warm start](testing-build-cache.md));
- in the shared build cache, which keys a compile by its arguments.

The two flags are the same words in every worktree. Thus neither cargo nor the cache sees a difference.

The `-B` folder is relative. The C compiler looks for it from the folder where the compile runs. For the program and
its tests, this folder is the repo itself. The build scripts and macros of the libraries compile in their own folders,
where there is no `scripts/ld/`. They link with the `ld` of the system. They are small programs, built once.

## What it saves

These times were measured on vm-str (16 cores, Rust 1.98.1, with other tasks building at the same time). Each time is
for a one-line change to `src/lib.rs`, built as the check builds: the program, then the test programs. Each build ran
six times. Each cell is the range of the time of one link.

| Link | The lld of Rust (before) | mold on the 2 cores of the check | mold on all cores (as shipped) |
| --- | --- | --- | --- |
| The program (39 MB) | 0.22–0.27 s | 0.16–0.20 s | 0.09–0.12 s |
| The test program (`tests/main.rs`) | 0.37–0.45 s | 0.28–0.33 s | 0.14–0.19 s |
| The library's own tests | 0.12–0.16 s | 0.06–0.10 s | 0.05–0.09 s |
| The program with debug data (`dev`, 134 MB) | 0.39–0.51 s | — | 0.17–0.22 s |
| **All links after one edit** | **about 0.8 s** | about 0.6 s | **about 0.4 s** |

mold links in about half the time. But the time of the whole build almost does not change. The program takes about
2.7 s, and the tests take about 6 s more. The runs differ by as much as the gain. Compiling is almost all of the time,
because the lld of Rust was already fast.

## Related pages

- [How the tests work](testing.md)
- [The build cache and the warm start](testing-build-cache.md)
- [Crates](crates.md)
- [Run the tests](run-the-tests.md)
