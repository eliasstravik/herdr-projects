# Release checklist

The repository is public and carries the `herdr-plugin` topic, so the Herdr marketplace can list it.

## Every release

- [ ] `version` agrees in `Cargo.toml`, `herdr-plugin.toml` and `Cargo.lock`.
- [ ] `cargo build --release --locked` and `cargo test` pass on macOS and on Linux.
- [ ] Walk through `docs/manual-test.md`, including the client-witnessed checks.
- [ ] Tag `v<version>` on `main`, push the tag, and create the GitHub release with notes in user terms.
- [ ] On every machine that runs the plugin from a checkout: `git pull`, `cargo build --release --locked`, then `herdr-projects doctor` and `doctor --fix`.

## Done

- [x] Public repository with the `herdr-plugin` topic, description and homepage set.
- [x] No "private repository" wording in `README.md` or `docs/getting-started.md`.
- [x] `v0.2.0`: the Herdr-native redesign.

## Open

- [ ] Verify `herdr plugin install eliasstravik/herdr-projects` from a clean machine: it clones, builds with `cargo build --release --locked`, and registers the actions and popups.
- [ ] Optional: prebuilt release binaries so users need no Rust toolchain.
- [ ] `docs/herdr-notes.md` and `docs/manual-test.md` name the author's machines and home paths. Generalise them.
