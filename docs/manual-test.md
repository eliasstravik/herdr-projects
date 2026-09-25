# Manual test list

## 0.2.0: the Herdr-native redesign

Checked on 2026-09-23 in a scratch `hp-dev` session (herdr 0.9.1, macOS, Claude Code 2.1.280). "Builder" means the builder ran it and read the result; "client-witnessed" needs a person looking at an attached Herdr client.

| Slice | Check | How it was checked |
| --- | --- | --- |
| 1 | `new` writes `AGENTS.md`, `CLAUDE.md` → `AGENTS.md`, `uploads/`; `open` starts an agent there and its first reply runs `skill` and `context` through the absolute path | Builder (Claude Code). Codex and OpenCode: client-witnessed |
| 1 | A `--kind tab` thread's first turn does not run them and does not call itself the coordinator | Builder |
| 1 | A brief starts with the project header and has no operational settings; `thread prompt` lands under `## Follow-ups`; `thread show --json` has the Next list | Builder, unit |
| 1 | `doctor --fix` adds the priming files and `routines/pr-followup.md` to an older project | Builder, unit |
| 2 | After `configure`, a thread shows `hp_activity` within one turn; a thread that asks a question is `needs you` within one tick | Builder (hooks from a scratch settings file via `--settings`) |
| 2 | `unconfigure` leaves the hook files byte-identical, keeping later user edits | Unit, builder |
| 2 | A thread survives a server restart with native resume and keeps its group and name; a cleared name is re-applied | Builder (client attached through `script`) |
| 3 | Tokens `hp_project`, `hp_rank`, `hp_state`, the display name and the Space token `hp` are set; old tokens cleared | Builder (`herdr api snapshot`) |
| 3 | Four-line rows, colours, the project count and `projects: N need you` render; `focus <slug>` narrows and `unfocus` restores the by-need order | Client-witnessed |
| 4 | `prefix+a` opens the popup scoped to the current project (also in a thread's workspace), and on all projects elsewhere; `P` opens the project picker on the current scope, `/` filters it, `esc` clears then closes, from any section; `↵` focuses the thread and the popup is gone | Client-witnessed (the same TUI was driven in a pane by the builder) |
| 4 | A Next number key reaches the thread and its task file; `s` stops a working thread; a settings edit reaches PROJECT.md; `X` asks first | Builder |
| 5 | Resolving a merged thread leaves no worktree, branch or workspace and keeps `threads/<id>.md` and `library/<id>/`; an unmerged one keeps its branch and says so | Builder |
| 5 | `sweep --dry-run` lists a planted orphan worktree and `sweep --yes` removes it; `archive` closes and hides, `unarchive` reopens | Builder |
| 6 | A real pull request is polled; a review comment fires `pr-followup` and the thread fixes it; forwarding "Merge the PR" merges it; the merge resolves and cleans the thread | Builder (private scratch repo `eliasstravik/hp-pr-probe`) |
| 6 | A needs-you event gives one notification titled `<Project> · t-0009` with the request sound; `mute` silences it; the nudge waits for a minute of idle | Unit. Seeing the notification: client-witnessed |

## Before 0.2.0

The acceptance checks from the original plan, by stage, with how each was checked on 2026-09-17 (herdr 0.9.1; macOS 26 on the home Mac, Linux aarch64 on the second machine). "Builder" means the builder ran it in the throwaway `hp-dev` session and read the result; "unit" means a test in `cargo test`; "client-witnessed" means it is visual and the client has to look. Details of each run are in [`herdr-notes.md`](herdr-notes.md).

## Set up a throwaway session

```bash
scripts/dev-server                                   # headless `hp-dev` session, root = <repo>/.dev-root
scripts/dev-hp new demo && scripts/dev-hp open demo --session hp-dev
scripts/dev-herdr pane read <pane>                   # read a pane; `pane send-keys <pane> Down Enter` answers a dialog
```

## Checks by stage

| Stage | Check | How it was checked |
| --- | --- | --- |
| 1 | `herdr plugin list` shows the plugin; after linking no ticker runs and `~/.herdr-projects` does not exist; `doctor --session hp-dev` reports the herdr version | Builder |
| 2 | `new demo` creates the skeleton; `open demo --session hp-dev` yields a workspace in `hp-dev` only, with a coordinator that ran `skill` and printed the digest | Builder |
| 2 | With an untrusted folder, `open` leaves `prime_pending = true`, and the ticker delivers the priming prompt after the dialog is accepted | Builder (folder moved under `/private/tmp` and symlinked, because `~/dev` is trusted) |
| 2 | The printed `context` command works from a scrubbed environment | Builder and unit (`tests/cli.rs`) |
| 2 | `ticker stop` ends the ticker within two ticks; `ticker start` after a rebuild replaces the old one | Builder (1 s; seen in `.ticker.log`) |
| 3 | `thread start` on a scratch repo: branch `hp/demo/t-0001-*`, record `open`, brief with instructions and memory, report written with no out-of-directory prompt; `git status` shows nothing from `.herdr-project/` | Builder |
| 3 | Kill the pane, `thread restart` brings it back; forced failure before the worktree exists then `thread restart` creates it; restart of a running thread refuses | Builder, unit (cases a to e) |
| 3 | Closing the pane of a thread with a report leaves it under Ready for review with `pane closed` | Builder, unit |
| 3 | `thread start` returns in under a minute; the ticker launches within two ticks; a missing agent binary gives `failed` after three attempts | Builder (1 s; 15 s; `--profile kimi`, not installed), unit |
| 3 | `thread prompt` reaches the thread; the README allow-list suppresses the prompt for the standard-input form | Builder (also a control: an off-list subcommand did prompt) |
| 3 | `--remove-worktree` refused for a tab thread and for a dirty worktree; with the ticker stopped, a late report survives `resolve --remove-worktree`; a tab thread starts in `threads/<id>/` | Builder, unit |
| 4 | Two tab threads in one workspace show different `thread` tokens | Builder (`api snapshot`) |
| 4 | `overview` run without a terminal returns at once; a finished thread stays under Ready for review until `thread ack` | Builder |
| 5 | A finishing thread gives one inbox item and one nudge, none after until a new item | Builder (with `nudge = true`), unit |
| 5 | A second session with the plugin linked produces no "pane gone" items | Builder (`hp-dev2`) |
| 5 | A command routine does not run until `routine_commands = true` and `routine approve` in a terminal; an edited command stops; approve without a terminal refuses | Builder (approved inside a herdr pane), unit |
| 5 | Fake `gh`: a merged pull request resolves its thread; a comment gives an item with no body; two events in one tick give two items | Unit. A run against a real pull request was not done (needs a push) |
| 6 | `thread start --machine` creates a worktree and agent on the second machine; the report and a library file come home within two minutes; Ready for review | Builder (about 70 s) |
| 6 | A short failed connection: no item, no group change, local ticks not slowed; a long one with a one-minute threshold: exactly one `outage` item and one recovery item | Builder (ssh shim on the ticker's `PATH`), unit |
| 6 | A title with quotes, spaces and `$(...)` reaches the remote `--label` unchanged and runs nothing | Builder |
| 6 | Always-on recipe: open a project on the second machine, reattach from this Mac with `herdr --remote <target> --session <name>`, answer a blocked prompt through it | Builder |
| 7 | `thread adopt` gives a thread with a brief; two adopted panes in one directory get separate thread directories; an adopted agent that ends in `done` still gets its pending prompt | Builder (first), unit (all three) |
| 7 | `adopt-workspace` creates a project from the current workspace | Builder: the action's handoff live, then the popup's core through the CLI. The popup itself: client-witnessed |
| 7 | A paused project refuses `thread start` and is skipped by the ticker; an archived one is hidden and its tokens are gone; `delete` refuses while the coordinator is alive, `--force` moves the folder to `.trash/` and leaves worktrees alone | Builder, unit |
| 7 | `new ../x`, `open ../x`, `thread list ../x` are refused | Unit (`tests/cli.rs`) |

## Client-witnessed checks

| Stage | Check | How to look |
| --- | --- | --- |
| 4 | `focus <slug>` shows only the project's panes in the sidebar, the coordinator first, then by attention; `unfocus` restores the full list | In a session with a project open and at least one thread plus one unrelated agent pane: run the `focus` action (or `herdr-projects focus <slug>`), look at the sidebar's agent list, then run `unfocus`. The builder verified that herdr accepts the request and that the tokens it filters on are present on the panes, but herdr does not expose the active view through `api snapshot`. |
| 6 | Remote thread panes in herdr's connected-machines sidebar: do they show the `project`, `thread` and `review` tokens? | With a remote thread running, select the machine in the sidebar (or run `herdr --remote <target>`) and look at the thread's agent row. The builder verified the tokens are set on the remote panes (`herdr --machine M api snapshot`) but cannot see a sidebar. `focus` does not cover remote threads either way. |
| 7 | The three interactive popups: `new` (asks name and goal, then creates and opens), `pick` (numbered project list for `open`, `pause`, `resume` when the current workspace is not a project) and `adopt` (asks the project name, pre-filled with the workspace label) | Run each action with `herdr plugin action invoke <id> --plugin herdr-projects`. The builder verified the actions' handoff and the code the popups run, but a headless session has no client to show or type into a popup. |
| 7 | The `overview` popup stays open until Enter, and the `doctor` action shows a notification | Run both actions with `herdr plugin action invoke`. Notifications are disabled in a headless session (`shown: false`). |
