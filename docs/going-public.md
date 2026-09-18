# Checklist for going public

The repository is private for now. When it is released as a public Herdr plugin:

- [ ] Make `eliasstravik/herdr-projects` public.
- [ ] Add the GitHub topic **`herdr-plugin`**: the Herdr marketplace indexes public repositories with that topic, so this is what makes it appear as a Herdr plugin. (`gh repo edit eliasstravik/herdr-projects --add-topic herdr-plugin`)
- [ ] Set the repository description and homepage to match `herdr-plugin.toml`.
- [ ] Remove "You'll need access to the private repository" from `README.md` (step 1 and "What does it cost?") and from `docs/getting-started.md`.
- [ ] Verify `herdr plugin install eliasstravik/herdr-projects` from a clean machine: it clones, builds with `cargo build --release --locked`, and registers nine actions and four popups. Whether `install` can clone a *private* repository is unverified; `herdr plugin link` is the path while private.
- [ ] Walk through `docs/manual-test.md`, including the client-witnessed checks.
- [ ] Tag a release (`v0.1.0`) once `Cargo.toml`, `herdr-plugin.toml` and the tag agree.
- [ ] Optional, later: prebuilt release binaries so users need no Rust toolchain.
- [ ] Optional: exercise another agent kind (Codex, Gemini) and record the result in `docs/herdr-notes.md`.
- [ ] `docs/herdr-notes.md` and `docs/manual-test.md` name the author's machines and home paths (`elias-macbook-pro-m1`, `/Users/…`, `/home/…`). Harmless while private; generalise them before the repository is public.
