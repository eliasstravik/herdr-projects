# herdr notes

What the builder verified about herdr, per stage, against the plan's "Assumptions to verify" table.

## Stage 1 (2026-09-17)

First pass ran on herdr 0.9.0; the client then upgraded the CLI to 0.9.1 and the rest was checked in a fresh `hp-dev` server on 0.9.1. The client's default session server was still 0.9.0 (`server_binary_stale: yes`) and was not touched.

| Assumption | Result |
| --- | --- |
| herdr is 0.9.1 or later on this Mac | Did not hold at first (`herdr 0.9.0`): stopped and asked; the client upgraded. Now `herdr 0.9.1`, and `doctor --session hp-dev` reports it. |
| A named session's server can start without an attached terminal | **Holds.** `herdr --session hp-dev server`, started detached with `HERDR_PROJECTS_ROOT` exported and pane variables scrubbed (`scripts/dev-herdr`), shows as `running` in `herdr session list`. |
| The builder can answer TUI dialogs with `herdr pane send-keys` | Mechanism holds: `pane send-text`, `pane send-keys <pane> Enter` and `pane read` work against a shell in `hp-dev`. Answering a real agent dialog is exercised in stage 2. |
| The herdr CLI targets a session when `HERDR_SOCKET_PATH` is set | **Holds.** With only `HERDR_SOCKET_PATH=<hp-dev socket>`, `herdr workspace list` returned hp-dev's (empty) list, not the default session's. |
| herdr commands have JSON output with ids | **Holds.** Output is JSON by default: `workspace create` returns `result.workspace.workspace_id` (`w1`), `result.tab.tab_id` (`w1:t1`), `result.root_pane.pane_id` (`w1:p1`), plus `cwd`, `foreground_cwd` and `agent_status`. `session list --json` returns `name`, `default`, `running`, `socket_path`. |
| An agent pane lacks the plugin environment variables | **Half true.** An ordinary agent pane (the builder's own) has `HERDR_BIN_PATH`, `HERDR_SOCKET_PATH`, `HERDR_WORKSPACE_ID`, `HERDR_TAB_ID`, `HERDR_PANE_ID` and `HERDR_ENV`. The plugin-only ones (`HERDR_PLUGIN_*`) were not present. Fallback: no change needed; the explicit `--root` prefix still works. |
| A detached child of a `[[startup]]` command survives | Not checkable until `ticker run` exists; checked at the start of stage 2, when the hp-dev server is restarted with a project present. `open` calls `ticker start` regardless, which is the fallback. |

Consequences noted:

- Because every pane has `HERDR_SOCKET_PATH`, a bare `herdr ...` or a flagless `doctor` run from a pane in the client's default session targets that session. All development commands therefore pass `--session hp-dev` and unset `HERDR_SOCKET_PATH`.
- Session sockets live at `~/.config/herdr/herdr.sock` (default) and `~/.config/herdr/sessions/<name>/herdr.sock`, but the binary asks `herdr session list --json` rather than assuming this layout.
- Panes in a named session also carry `HERDR_SESSION`, and inherit whatever the server was started with (`HERDR_PROJECTS_ROOT` was visible in an hp-dev pane). That is why the root is exported before the server starts.
- `herdr plugin link` is global (`~/.config/herdr/plugins.json`), not per session. After linking: `plugin list` shows `herdr-projects`, no `herdr-projects` process is running, and `~/.herdr-projects` does not exist.
- macOS ships `openrsync` (protocol 29) as `rsync`. Relevant to the stage 3 rsync assumption.

## Stage 2 (2026-09-17, herdr 0.9.1)

| Assumption | Result |
| --- | --- |
| A detached child of a `[[startup]]` command survives (carried over from stage 1) | **Holds.** With one project in the scratch root, restarting the `hp-dev` server ran `[[startup]]`; `ticker run` (new session via `setsid`, null stdio) was alive afterwards with the server's `HERDR_PROJECTS_ROOT`. |
| The builder can answer an agent's TUI dialogs with `pane send-keys` (completed from stage 1) | **Holds.** `pane send-keys <pane> Down` then `Enter` accepted claude's trust-this-folder dialog. |
| `workspace create`, `tab create`, `pane focus`/`agent focus`, `notification show` and `api snapshot` exist with the flags needed | **Holds.** `workspace create --cwd --label --focus/--no-focus`; `tab create --workspace --cwd --label`; `tab rename <TAB_ID> <LABEL>`; `agent focus <target>`; `notification show <TITLE> --body`; `api snapshot`. |
| A prompt sent while the user has half-typed text does not merge with or submit that text | **Does not hold.** With `HALFTYPED user draft` typed and not submitted, `herdr agent prompt` produced one submitted message `HALFTYPED user draftReply with exactly…`. **Fallback taken:** `nudge` defaults to `false`; with it off the ticker shows `herdr notification show` instead (built in stage 5); the risk goes in the README. |

Also learned:

- **herdr's CLI parser wants positionals first, options after.** `pane report-metadata --source x <PANE>` fails with `unknown option: x`; `pane report-metadata <PANE> --source x` works. There is no `--` separator except in `agent start`. Text in a positional slot may start with a dash (`agent prompt <target> "-x hello"` is accepted).
- **Errors** are one JSON object with `error.code` and `error.message`, exit status 1. `agent prompt` and `agent focus` accept a pane id as the target.
- **A trust dialog**: `agent start` fails fast (about 1 s, not a timeout) with `agent_not_ready` ("blocked during startup"), and `agent list` shows the agent as `blocked` with its name already set. After the dialog is accepted the state becomes `idle` and the ticker's pending-prompt path delivers. This settles the first stage 3 assumption early.
- **claude's folder trust is inherited from parent folders**, and `~/dev` is trusted on this Mac, so nothing under the scratch root shows a trust dialog. For the `demo2` check the project folder was moved to the builder's scratch directory under `/private/tmp` and symlinked back into `.dev-root`. That exposed that herdr reports a pane's *physical* working directory, so the recorded `cwd` is now the canonical project path.
- **Pane, tab and workspace ids restart from `w1` after a server restart**, so ids are reused for different panes across restarts. The identity check (ids plus working directory plus agent name) is what keeps a stale record from matching a new pane.
- `notification show` in a headless session returns `{"shown": false, "reason": "disabled"}` with exit 0.
- `api snapshot` exposes pane tokens at `result.snapshot.panes[].tokens` and `result.snapshot.agents[].tokens` (the coordinator pane showed `project`, `thread`, `rank`). Stage 4 sidebar checks can be read by the builder.
- An `hp-dev` server started from inside an agent's shell inherits that agent's environment (claude then reports "inherited CLAUDE_CODE_CHILD_SESSION"). `scripts/dev-server` therefore starts it with `env -i` and a minimal environment.
- The client's `claude` runs in auto mode, so the "ordinary first-command prompt" did not appear; the coordinator ran `skill` and `context` unprompted.

## Stage 3 (2026-09-17, herdr 0.9.1)

| Assumption | Result |
| --- | --- |
| `agent start` returns only once the agent is ready, and a trust dialog shows as `blocked` or a timeout | **Holds.** A ready agent returns in about 4 s with `agent_status: idle`. A trust dialog makes `agent start` fail after about 1 s with `agent_not_ready`, and `agent list` shows the agent `blocked`. A missing agent binary gives `timeout` after the full 20 s and no agent is listed. No extra `agent wait` is needed. |
| Pane, tab and workspace ids are not reused for a different pane, and either survive a server restart or all change together | **Partly.** Within one server run ids were never reused (`w1` … `wB`, hexadecimal). After a server restart they start again at `w1`, so an old record can name a new pane. The identity check (workspace id, tab id, working directory, agent name) is what prevents acting on the wrong pane, as the plan's fallback says. |
| `agent start` accepts a name that was used before | **Holds.** `thread restart` reused `hp-demo-t-0001` in a new pane after the old pane was closed. |
| `worktree remove` refuses a dirty worktree without a force option | **Holds.** Untracked or modified files give `dirty_worktree_requires_force`; the binary never passes `--force` and prints herdr's message unchanged. The branch is kept after a removal. |
| `rsync -rt` behaves the same with macOS's rsync and GNU rsync (symlinks skipped without `-l`) | **Holds on macOS.** `openrsync` (protocol 29) prints `skipping non-regular file` for file and directory symlinks, copies the rest and exits 0. The binary also walks the library with `lstat` first, so a partial copy is reported without parsing rsync's output. GNU rsync is checked on the remote machine in stage 6. |
| The working directory herdr reports for a pane does not change when the agent changes directory | **Holds for claude.** A shell's `cd` does change `cwd`, but claude running `cd /usr/local && pwd` left the agent's reported `cwd` unchanged. Other agent kinds are untested; if one changes it, that thread shows as `pane closed` rather than being acted on wrongly. |

Also learned:

- **`worktree open` needs `--cwd <repo>`**; with `--path` alone it answers `worktree_not_found`.
- **herdr puts worktrees under `~/.herdr/worktrees/<repo>/<branch-with-hyphens>`**, and `worktree create` also opens a workspace for the main repository when none is open. The binary records the path herdr returns. Only for `thread start --workspace`, which runs `git worktree add` itself, does it build the same path.
- **Excluded files do not protect a worktree from removal.** With only `.herdr-project/` (listed in `info/exclude`) present, `worktree remove` succeeds and deletes it. This is why `--remove-worktree` requires a complete final copy.
- **Every new worktree shows claude's trust dialog** on this Mac, because `~/.herdr/worktrees` is not under a trusted parent. So under the defaults every worktree thread begins as `prompt_pending`, shows under Waiting on you after 60 s, and needs one Enter in its pane. **Tab threads do not** (their folder is under the project folder, which the user trusted when opening the coordinator). After the dialog, claude's ordinary first-edit and first-command prompts show the thread under Waiting on you after 30 s each, as the plan expects. The README says so.
- **A failed `git worktree add` can leave the branch behind** (git creates the branch before the directory). `thread restart` then takes case (b) and asks for a human look, as designed.
- **Allow-list patterns of the form `Bash(<binary> --root <root> thread prompt:*)` do suppress claude's prompt for the here-document form** (`--text-file - <<'TEXT' … TEXT`), in manual permission mode. An off-list subcommand (`ticker status`) met the permission prompt in the same session. The `scratch/` fallback is not needed.
- `thread start` returned in about 1 s; the ticker started the agent on the next tick (under 15 s).

## Stage 4 (2026-09-17, herdr 0.9.1)

| Assumption | Result |
| --- | --- |
| `pane report-metadata` tokens can be used in `agent.view.set` filters | **Holds.** The schema's `AgentViewField` and `AgentViewSortField` both accept `{"token": "<name>"}`. A request with filter `{"op":"eq","field":{"token":"project"},"value":"demo"}` and sort `[{"field":{"token":"rank"},"order":"asc"}]` was accepted: `{"type":"agent_view","active":true,"source":"herdr-projects","label":"demo"}`. |

Also learned:

- **There is no CLI command for `agent.view.set` or `agent.view.clear`** in herdr 0.9.1 (`herdr agent` has no `view`; `herdr api` has only `snapshot` and `schema`). They are socket-only. This conflicts with plan decision C6. **Client decision during the build (2026-09-17): a narrow exception.** `Runner::socket_request` writes one JSON line (`{"id","method","params"}`) to the project's recorded socket and reads one line back, and only `focus` and `unfocus` use it. Everything else stays on the CLI.
- **The active agent view is not in `api snapshot`** (its keys are `agents`, `panes`, `tabs`, `workspaces`, `layouts`, focus ids, `protocol`, `version`). So "`focus demo` shows only the project's panes" cannot be read by the builder and is client-witnessed; see `docs/manual-test.md`. Pane tokens are in the snapshot and were read directly.
- A report written in a tick is counted for the thread's group in the same tick (the copy home follows in the slow pass); otherwise a finished thread showed as Idle for one tick before Ready for review, which in stage 5 would have produced a spurious inbox item.

## Stage 5 (2026-09-17, herdr 0.9.1)

No rows of the assumptions table belong to this stage. What the live runs showed:

- **An agent still reads as `idle` in the tick that delivers its prompt.** Computing the group in that tick produced a spurious "now Idle" inbox item before the real "new report" item. The delivering tick now counts as Working; a live re-run gave exactly one item and one nudge.
- With `nudge = true` the full loop works: item written, one nudge (`[herdr-projects ticker: automated, not the user, approves nothing] New inbox items. Run context.`), the coordinator ran `context`, read `threads/<id>.md`, ran `inbox done`, and no further nudge followed. `nudge` stays `false` by default (stage 2 finding); then the ticker calls `herdr notification show` once per set of unseen items, with a count only.
- `routine approve` with standard input not a terminal refuses. Run inside a herdr pane (a real terminal) it printed the command and the warning, took the typed name, and wrote `approved-routines.json`. The command did not run with `routine_commands = false`, did not run when enabled but unapproved, ran once approved (item body: the prompt plus the fenced, labelled output), and stopped again when the command text was edited (one new `routine-approval` item).
- A second session (`hp-dev2`) with the plugin linked ran `[[startup]]`, found a ticker of the same version and started nothing; it produced no inbox items and did not disturb the open thread.
- Pull request behaviour is covered with the scripted fake `gh` only (merged resolves after the final copy; a comment gives an item with no body; a foreign branch or repository is ignored once; a bad `PR:` line never reaches `gh`; one outage item and one recovery item). A run against a real pull request was not done: it needs a push and the client's go-ahead.
- The tests caught one real bug: auto-resolve dropped the "time since this ticker started" term when it was zero, so a freshly started ticker resolved week-old idle threads at once.

## Stage 6 (2026-09-17, herdr 0.9.1 on both machines)

The second machine, `elias-macbook-pro-m1`, runs Linux (aarch64), herdr 0.9.1, GNU rsync 3.5.0, git and claude. The client approved `herdr machine add` against its **default** session, and approved copying the source there, building it and linking the plugin, with the test project in a throwaway `hp-dev` session on that machine.

| Assumption | Result |
| --- | --- |
| `herdr --machine M worktree create` and `agent start` work and return the remote worktree path | **Holds.** `worktree create` returns `worktree.path` and `root_pane.cwd` as they are on the remote (`/home/…/.herdr/worktrees/<repo>/<branch>`); `agent start`, `agent prompt`, `agent list`, `pane list`, `pane send-keys`, `pane report-metadata` and `api snapshot` all work through `--machine`. |
| `herdr machine list --json` exposes an SSH target | **Holds.** Each entry has `id`, `label`, `target`, `session`, `enabled`, `selected`. `[machines.<label>] ssh` in `config.toml` remains the fallback. |
| `herdr --machine M worktree open` works for `thread restart` on a remote thread | **Holds** (with `--cwd <repo>`, as locally). `worktree remove --workspace` also works remotely and keeps the branch. |
| `rsync` is present on this Mac and on the remote machine | **Holds.** openrsync here, GNU rsync 3.5.0 there; a library file came home over `rsync -rt -e ssh`. The full test suite, including the symlink-skipping copy test, also passes on the Linux machine, which completes the stage 3 rsync row for GNU rsync. |
| Setting `HERDR_SOCKET_PATH` to the recorded local socket does not break `herdr --machine M ...`, and `--machine` targets the remote's default session | **Holds.** With `HERDR_SOCKET_PATH` set to the `hp-dev` socket, `herdr --machine M workspace list` listed the remote default session's workspaces. **But `--machine` cannot be combined with `--session`** ("--machine cannot be combined with other launch options; it uses the saved machine's session"), so the binary's env-variable form is the right one; the remote session is whatever `machine add --remote-session` saved (default here). |
| Remote thread panes appear in the local sidebar, carry pane tokens, and are included by `focus` | **Not as written; fallback taken.** Tokens reported through `--machine` are set on the *remote* server's panes (seen in `herdr --machine M api snapshot`), and the local server's snapshot contains no remote panes. `focus` installs its view on the project's local server only. So remote threads are in the text `overview`, in `thread list` and in inbox items, and their tokens show when the user looks at that machine; `focus` does not cover them. The README says so. Whether herdr's connected-machines sidebar shows the tokens is client-witnessed (`docs/manual-test.md`). |
| `herdr --remote <target>` attaches to a herdr server on the other machine, and a user can reach a blocked remote pane through it | **Holds.** From a pane on this Mac, `herdr --remote elias-macbook-pro-m1 --session hp-dev` showed the remote project's workspace and its blocked coordinator; `Down`, `Enter` typed into that client answered claude's trust dialog on the remote, and the remote ticker then delivered the priming prompt. Without `--session` it attaches to the remote default session, where a remote thread's pane and agent (`hp-demo-t-0009`) were visible. |

Also learned:

- **A remote thread ran end to end**: `thread start --machine` returned in 4 s (one ssh call for origin and base, one `herdr --machine worktree create`, one ssh call for directory, `info/exclude` and brief); the ticker launched the agent at the next remote poll; after the agent finished, the report and a library file were in the home project and the thread showed Ready for review about 70 s later.
- **A title of `Remote "hello" $(touch /tmp/hp-pwned2) it's` reached the remote `--label` unchanged and ran nothing**; the branch became `hp/demo/t-0009-remote-hello-touch-tmp-hp-pwned2-it-s`.
- **Outages** were simulated with an `ssh` shim first on the ticker's `PATH` (it fails like an unreachable host while a flag file exists); `herdr --machine` uses `ssh` from `PATH`, so this covers both herdr and the binary's own calls without touching the other machine or the client's ssh config. About 100 s of failure with the default threshold: no inbox item, no group change, one logged failed poll, and a local thread started during it had its agent launched 15 s later. With `HERDR_PROJECTS_OUTAGE_SECS=60`: exactly one `outage` item (after the second failed poll), still one after 2.5 more minutes down, then one "reachable again" item at the first successful poll.
- **The always-on recipe works**: the plugin built on the Linux machine (`cargo build --release --locked`, 12 s) and all tests pass there; a project opened in a session on that machine has its own ticker there and needs nothing from this Mac. Two things to document: (1) `herdr plugin link` fails with `plugin_requires_newer_herdr` while that machine's *running server* is still 0.9.0, even though the CLI is 0.9.1 — restart the server after upgrading; (2) attaching with `herdr --remote` to a server that was started by hand over ssh asks whether to restart it ("may not survive SSH connection loss"); answer `n` to keep its panes.
- The source was copied to `~/dev/herdr-projects` on that machine with rsync (there is no GitHub repository yet) and the plugin is still linked there.

## Stage 7 (2026-09-17, herdr 0.9.1)

| Assumption | Result |
| --- | --- |
| A plugin action can open a plugin pane with `herdr plugin pane open`, and the action (not the popup) receives the originating pane in `HERDR_PANE_ID` or `HERDR_PLUGIN_CONTEXT_JSON` | **Holds.** An action's environment has `HERDR_PANE_ID`, `HERDR_TAB_ID`, `HERDR_WORKSPACE_ID`, `HERDR_SOCKET_PATH`, `HERDR_SESSION`, `HERDR_PLUGIN_ID`, `HERDR_PLUGIN_ACTION_ID`, `HERDR_PLUGIN_ROOT`, `HERDR_PLUGIN_STATE_DIR`, `HERDR_PLUGIN_CONFIG_DIR`, `HERDR_BIN_PATH` and `HERDR_PLUGIN_CONTEXT_JSON` (`workspace_id`, `workspace_label`, `workspace_cwd`, `tab_id`, `tab_label`, `focused_pane_id`, `focused_pane_cwd`, `focused_pane_agent`, `focused_pane_status`, `invocation_source`, `correlation_id`). `herdr plugin pane open --plugin herdr-projects --entrypoint <id>` from an action returns `{"type":"ok"}` and the popup's command runs. **The popup does not get `HERDR_PANE_ID`, `HERDR_TAB_ID` or `HERDR_WORKSPACE_ID`** (it does get the context JSON), which is why the action captures the pane and hands it over in `handoff.json` in the plugin state directory. |

Also learned:

- **Popups are not panes in `pane list`**, and the throwaway session has no client attached, so the builder cannot type into a popup. The three interactive popups (`new`, `pick`, `adopt`) are client-witnessed (`docs/manual-test.md`). What was checked instead: the actions write the right handoff and open the right entrypoint (unit tests and one live `plugin action invoke adopt-workspace`, which captured pane `wG:p1`, the workspace label and its directory), and the popup's core ran live through `adopt-workspace --name … --pane … --workspace-cwd …`: project created, opened in the same session, pane adopted as `t-0001`.
- **claude's trust follows the git root, not only the parent folder**: a fresh `git init` inside the trusted `~/dev` tree still showed the trust dialog. An agent adopted while blocked on it got `prompt_pending = true`, and the ticker delivered the brief line once the dialog was answered.
- `plugin action invoke` takes its context from the session's focused pane (`invocation_source: "cli"`), so actions can be exercised from the CLI.
- After `delete --force` the ticker did not recreate the project folder (checked 35 s later): writers re-check `PROJECT.md` after taking the lock.

## Stage 8 (2026-09-17)

- The development `[safety]` table and routine approval were removed; `~/.config/herdr-projects/` held nothing else and was removed. The `hp-dev` session, the scratch root `.dev-root`, the scratch repositories and their worktrees were removed. `~/.herdr-projects` was never created.
- `delete demo --force` moved the project to `.trash/` and left the scratch repository's thread branch in place.
- Left in place for the client to decide: the plugin is linked on this Mac and on the second machine (source at `~/dev/herdr-projects` there), and `elias-macbook-pro-m1` is a saved herdr machine.
- The README follows the structure of the client's `herdr-call` and `herdr-agent-progress` READMEs at the client's request; the detail the plan asked the README to carry (symlink, allow-list, soft-guard and routine warnings, the `unfocus` note) is in `docs/getting-started.md` and `docs/operations.md`, with the warnings summarised in the README's questions. The `/landingpage-readme` skill can only be run by the client, so it was not used.

## After the build (2026-09-18, herdr servers restarted on 0.9.1 on both machines)

- With both default servers on 0.9.1 the plugin loads in them: all nine actions are listed on this Mac and on the second machine, `plugin link` works there without a named session, and no ticker runs on either (no projects yet).
- **A herdr server not started from a login shell gives plugins a minimal `PATH`.** The `doctor` action in this Mac's default session reported `gh` as not installed although it is at `/opt/homebrew/bin/gh`. A ticker started by `[[startup]]` would have silently skipped pull request follow-up. The binary now appends `/opt/homebrew/bin`, `/usr/local/bin`, `~/.local/bin` and `~/.cargo/bin` to its own `PATH` at startup; the same action then reported `gh` and `gh auth` as ok.
- `eliasstravik/herdr-projects` was created as a private repository with the `herdr-plugin` topic. The name had been a redirect to `herdr-tracker` (that repository's earlier name); creating the new repository replaced the redirect. No local clone used the old URL.
