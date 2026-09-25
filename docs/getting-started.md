# Getting started: open your first project

Install the plugin, run `configure` once, create a project, and talk to its coordinator.

## 1. Check the prerequisites

- macOS or Linux, and [Herdr](https://herdr.dev) 0.9.1 or newer. Check with `herdr status`: both the client and the running server must be 0.9.1 or newer. After `herdr update`, a server that was already running stays on the old version until you restart it, and `herdr plugin link` or `install` then fails with `plugin_requires_newer_herdr`.
- Only to build from source: Rust/Cargo 1.89 or newer and a C compiler. Releases carry prebuilt binaries for macOS and Linux on Apple Silicon/arm64 and Intel/x86_64, so most installs need neither. On macOS, `xcode-select --install` installs Apple's build tools. Install Rust with [rustup](https://rustup.rs).
- Git.
- An agent CLI Herdr can start, on `PATH`. Any of Herdr's 24 agent kinds works (`claude`, `codex`, `opencode`, `cursor`, `gemini` and more). Claude Code is the one exercised most. Progress self-reports come through hooks, which `configure` installs for Claude Code and Codex; other agents still work, with the state Herdr detects on its own.
- Optional: `gh`, logged in, for pull request follow-up; `ssh` and `rsync` for threads on other machines.

The plugin needs no hosted service and no API key. It depends on Herdr and nothing else, no other plugin included.

## 2. Install the plugin

```bash
herdr plugin install eliasstravik/herdr-projects
```

Review the install preview. Herdr clones the repository, runs `scripts/install.sh`, and registers the plugin. The script downloads the release's prebuilt binary for your machine and checks it against the release's `SHA256SUMS`. When there is no such binary, the download fails or the checksum does not match, it says so and runs the locked Cargo release build instead. Set `HERDR_PROJECTS_BUILD=source` to always build from source. A checkout with local changes, or on a commit after the release, also builds from source. Its startup command starts a background ticker only when you have at least one project.

To run the binary from a terminal, link it onto your `PATH`. `herdr plugin list` prints the plugin's folder:

```bash
ln -s <plugin root>/target/release/herdr-projects ~/.local/bin/herdr-projects
herdr-projects doctor
```

## 3. Run configure once

```bash
herdr-projects configure --dry-run   # shows what it would change
herdr-projects configure
```

Or run `herdr plugin action invoke configure --plugin herdr-projects`. It changes four things and records each change, so `herdr-projects unconfigure` removes exactly what it added:

- **Your Herdr config** (`~/.config/herdr/config.toml`). The sub-line row under agents (`$hp_sub`), a one-line card in place of Herdr's built-in rows that shows each project's head in bold (rows you wrote yourself are left alone), the popup key `prefix+a` and a tab-bar entry `projects: N need you`. Herdr checks the result with `herdr config check` before anything is written. Pick another key with `configure --key prefix+y`; a key Herdr or you already use is refused.
- **Claude Code hooks** in `~/.claude/settings.json` and **Codex hooks** in `~/.codex/hooks.json`. They tell an agent running in a Herdr pane how to report its progress, and remind it about once a minute. Outside Herdr they do nothing. Existing hooks and comments are kept.
- **The `autoproject` skill**, linked from the plugin's `skill/autoproject` into `~/.claude/skills` and Codex's `~/.agents/skills`. A coordinator loads it with `/autoproject` to run an independently reviewed improvement loop. A skill of that name that is not the plugin's link is left alone, and `doctor` names it. If you configured before the skill shipped, `update` links it for you.

Configure reloads the Herdr server's config. The sidebar rows are drawn by your client: if they don't show yet, run **reload config** in Herdr (`prefix+shift+r`).

If you used the standalone Agent Progress plugin, `doctor` prints the two commands that remove its hooks, so only one set runs.

## 4. Create and open a project

From a Herdr pane, run **Projects: new project** with `herdr plugin action invoke new --plugin herdr-projects`. It asks for a name and a goal, creates the project, and opens it. Or from a terminal inside Herdr:

```bash
herdr-projects new "Billing" --goal "Ship the new billing page" --repo ~/dev/app
herdr-projects open billing
```

`new` creates `~/.herdr-projects/billing/` with an `AGENTS.md` (and `CLAUDE.md` linked to it). `open` starts your agent in that folder, right in the pane you typed it in. Quit the agent and you are back at your shell. The agent reads `AGENTS.md`, which tells it that it is the coordinator and which two commands to run. Nothing is typed into it for you.

- `open billing --tab` starts it in a new tab of the project's own workspace instead. The plugin's actions and the popup always do that, and so does `open` run outside Herdr.
- When a coordinator is already running, `open` jumps to it. `open --new` starts another beside it, with a fresh conversation.
- `open billing --profile codex` starts another agent: every installed, signed-in harness is a profile, and your own profiles (a model, an effort, extra flags) are made in the popup's settings or with `profile add` ([operations](operations.md#agent-profiles)). Any agent you start by hand in that folder is a coordinator too, with no `open` needed, and several can run side by side.
- `open` resumes the agent's last session when Herdr recorded one for that profile.
- The first time, your agent may ask whether you trust the folder: answer it in the coordinator's pane.

## 5. Tell the coordinator what you want

Type in the coordinator's pane, for example: "Add a billing page: API endpoint, the page itself, and end-to-end tests."

On a new project it restates the goal, lists the repos, and asks for the first piece of work. It proposes threads and waits until you name the ones to start (or say "all"). Tell it how you like threads run ("workers use codex", "at most two at a time") and it remembers.

Everything about the project can be changed in chat: goal, instructions, repos, settings, tasks, routines, memory. You never need to edit a file.

## 6. Watch the threads

Each code thread runs in its own worktree workspace on a branch named `hp/<project>/<id>-<title>`; a task with no repository runs as a tab in the project's workspace.

- **The sidebar** shows each thread as `t-0003 · <title>` with a line under it that adds what Herdr's own state word does not say: `needs you · ~55%`, `review · PR #4`, `~40%`, `12m quiet`, `landing · PR #4`. The agent's own activity follows on the same line. The tab bar says `projects: 2 need you`. Agents and Spaces are grouped by project: each project starts with its own head row, the home Space and the coordinator, which show the project's name in bold, and its threads by need and its other Spaces follow as Herdr's own rows; agents or Spaces outside any project come last. Selecting a row lights only that row. The ticker keeps the Spaces in these blocks, so a Space you drag elsewhere moves back.
- **The popup** (`prefix+a`) lists threads, tasks, inbox, routines, settings and memory. Every thread report ends with a `## Next` list; press a number to send that line back to the thread, which then does it with its own tools. Other keys jump to a thread, stop it, restart it with another profile, resolve it, open its PR, edit settings, pause or archive the project.
- **Notifications** name the project and thread: `Billing · t-0003`, `needs you · blocked` with a sound; a new report or a merge with a softer one. `mute = true` (popup settings) silences a project.

New worktrees are folders your agent hasn't trusted yet, so a code thread usually starts with your agent's trust dialog and shows `needs you` until the coordinator answers it (with `thread keys`) or you do in its pane.

When a pull request fails its checks or gets review comments, the ready-made `pr-followup` routine prompts the thread to fix them. When it merges, the thread is resolved and its worktree, workspace and branch are removed. Its report stays in `threads/<id>.md` and its files in `library/<id>/`.

## Check your setup

```bash
herdr-projects doctor          # what is installed, configured, and left over
herdr-projects doctor --fix    # repairs the plugin's own files in each project
herdr-projects ticker status
```

- **A project made with an older version**: `doctor --fix` adds `AGENTS.md`, the `CLAUDE.md` link, `uploads/` and `routines/pr-followup.md`, rewrites binary paths that point at a moved binary, and links the `autoproject` skill for each harness you configured. It never touches another plugin's entries.
- **`open` says the session is not reachable**: run it inside Herdr, or pass `--session <name>`. A project belongs to the session it was first opened in.
- **A thread stays at "no agent"**: the ticker launches agents, one per project per tick (about 15 seconds). After three failed launches the thread is marked failed with the reason; `thread restart` tries again.
- **Herdr was restarted**: Herdr resumes Claude and Codex panes itself; the ticker gives resumed threads their names back. Threads of other agents need `thread restart`.

## Updating

**Once, if you're on 0.2.2 or older** (`herdr-projects --version`), which has no `update` yet:

```bash
herdr-projects ticker stop
herdr plugin install eliasstravik/herdr-projects
herdr-projects doctor --fix
herdr-projects ticker start
```

Herdr reinstalls the plugin in the same folder, so your `~/.local/bin/herdr-projects` link keeps working. If you linked a local checkout with `herdr plugin link` instead, run `git pull` and `sh scripts/install.sh` in it in place of the `herdr plugin install` line.

**From then on:**

```bash
herdr-projects update           # fetch, install the new binary, doctor --fix, restart the ticker
herdr-projects update --check   # only print the installed and the newest version
```

`update` works for both install types and changes nothing when you're already on the newest release. Its `doctor --fix` also links the `autoproject` skill for each harness you configured, so existing users don't need to run `configure` again. A linked checkout must be on `main` with no uncommitted changes, or `update` stops and says why. When the install fails, the old version stays installed and the ticker is restarted. `doctor` says when a newer version is out.

## Remove

```bash
herdr-projects unconfigure
herdr-projects ticker stop
herdr plugin uninstall herdr-projects      # or: herdr plugin unlink herdr-projects
```

Your projects stay in `~/.herdr-projects/`; delete them yourself if you no longer want them.
