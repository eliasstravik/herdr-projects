# Getting started: open your first project

Install the plugin, create a project, and let a coordinator agent start threads for you.

## 1. Check the prerequisites

- macOS or Linux, and [Herdr](https://herdr.dev) 0.9.1 or newer. Check with `herdr status`: both the client **and the running server** must be 0.9.1. After `herdr update`, a server that was already running stays on the old version until you restart it, and `herdr plugin link` or `install` then fails with `plugin_requires_newer_herdr`.
- Rust/Cargo 1.89 or newer and a C compiler. Herdr builds the executable during installation. On macOS, `xcode-select --install` installs Apple's command-line build tools if missing. Install Rust using [rustup](https://rustup.rs).
- Git and access to `eliasstravik/herdr-projects`. The repository is currently private. Authenticate Git for GitHub before installing; with GitHub CLI, use `gh auth login` and `gh auth setup-git`.
- An agent CLI Herdr can start, on `PATH`. Only Claude Code has been exercised so far; `PROJECT.md` lets you pick any kind Herdr supports (`herdr agent start --help` lists them).
- Optional: `gh`, logged in, for pull request follow-up; `ssh` and `rsync` for threads on other machines.

No hosted service or separate API key is required by the plugin. Your agent CLI has its own prerequisites and account.

## 2. Install the plugin

```bash
herdr plugin install eliasstravik/herdr-projects
```

Review the install preview. Herdr clones the repository, runs its locked Cargo release build, and registers `herdr-projects` with nine actions and four popups. Its startup command starts a background ticker only when you have at least one project; until then it creates nothing.

To run the binary from a terminal, create a symlink yourself. `herdr plugin list` prints the plugin's folder:

```bash
ln -s <plugin root>/target/release/herdr-projects ~/.local/bin/herdr-projects
herdr-projects doctor
```

`doctor` prints the binary's absolute path, its version, the projects root and the config directory, so you can see exactly what is running.

## 3. Create and open a project

From Herdr's action menu, run **Projects: new project**. It asks for a name and a goal, creates the project, and opens it. Or from a terminal inside Herdr:

```bash
herdr-projects new "Billing" --goal "Ship the new billing page" --repo ~/dev/app
herdr-projects open billing
```

`new` creates `~/.herdr-projects/billing/`. `open` creates a Herdr workspace in that folder with a `coordinator` tab, starts your agent there, and sends it one priming line that tells it to print and follow the coordinator skill. The first time, your agent asks whether you trust the folder: answer it in the coordinator's pane. The ticker sends the priming line as soon as the agent is ready.

Edit `PROJECT.md` in the project folder to write your standing instructions and to change the agent kind, the default model (`thread_model`), `max_parallel_threads`, or the listed repos.

## 4. Tell the coordinator what you want

Type in the coordinator's pane, for example: "Add a billing page: API endpoint, the page itself, and end-to-end tests."

You can also ask it to add, assign, delegate and show tasks. It keeps them in `TASKS.md`.

By default it lists the threads it suggests and waits. Reply with a go-ahead that names them ("start all three"). Name a model and it starts the thread on that model ("run the audit on claude-opus-5"); see [Operations](operations.md#choosing-a-model-per-thread). Your agent then asks permission to run `thread start` for each one, unless you've allow-listed it (see [Operations](operations.md#the-allow-list-for-your-coordinator)).

## 5. Confirm the threads appear

Each code thread opens as its own workspace on a branch named `hp/<project>/<id>-<title>`; a task with no repository opens as a tab in the project's workspace. Expand Herdr's agent sidebar to see `project`, `thread` and `review` beside each one, or run **Projects: overview**.

Expect one interruption per thread under the default settings: **a new worktree folder is a folder your agent hasn't trusted yet**, so each code thread starts with your agent's trust dialog and shows under Waiting on you until you press Enter in its pane. After that come your agent's ordinary first-edit and first-command prompts. Tab threads live inside the project folder you already trusted, so they skip the dialog. To reduce the prompts, set `thread_agent_args` for the project (see [Operations](operations.md#safety-settings)).

When a thread finishes it writes a report. The report is copied to `threads/<id>.md` in the project folder and the thread moves to Ready for review. Tell the coordinator you've looked (it runs `thread ack`), or resolve the thread:

```bash
herdr-projects thread resolve billing t-0001                     # keep the worktree
herdr-projects thread resolve billing t-0001 --remove-worktree   # remove it; the branch is kept
```

## Check your setup

```bash
herdr-projects doctor
herdr-projects ticker status
```

- **`open` says the session is not reachable**: run it inside Herdr, or pass `--session <name>`. A project belongs to the session it was first opened in; opening it from another one is refused.
- **A thread stays at "no agent"**: the ticker launches agents, one per project per tick (about 15 seconds). `ticker status` shows whether it runs and which `herdr`, `git`, `gh`, `ssh` and `rsync` it resolves from its own environment, which may differ from your shell. After three failed launches the thread is marked failed with the reason; `thread restart` tries again.
- **Herdr was restarted**: panes are gone but records, reports and branches are not. Run `open <project>` for a new coordinator and `thread restart <project> <id>` for each thread you want back. The coordinator starts with no chat history; it works from memory, thread records and the inbox. To keep your agent's own history, set `coordinator_agent_args = ["--continue"]` (for Claude Code).
- **The coordinator forgot how to behave** after a long conversation: `herdr-projects open <project> --reprime`.

## Optional configuration

User-level settings live in `~/.config/herdr-projects/config.toml`, which you edit by hand. No agent works in that folder.

```toml
# Where projects live (default ~/.herdr-projects). HERDR_PROJECTS_ROOT and --root win over this.
root = "~/projects"

# Only needed when `herdr machine list --json` shows no SSH target for a machine.
[machines.buildbox]
ssh = "me@buildbox.local"
```

Safety settings are per project, in the same file. `herdr-projects safety show <project>` prints the effective values and the exact table header to add.

## Upgrade

```bash
herdr plugin install eliasstravik/herdr-projects
herdr-projects ticker start
```

A rebuilt binary has a new build identifier. `ticker start` (also run by `open` and `thread start`) stops a ticker of another version and starts the new one; it never replaces a healthy ticker of the same version.

## Remove

```bash
herdr-projects ticker stop
herdr plugin uninstall herdr-projects      # or: herdr plugin unlink herdr-projects
```

Your projects stay in `~/.herdr-projects/` and your settings in `~/.config/herdr-projects/`; delete them yourself if you no longer want them. Worktrees and branches that threads created are yours: nothing removes them for you.

## Troubleshooting

- **`plugin_requires_newer_herdr` although `herdr --version` says 0.9.1**: the running server is older than the CLI. Restart it (`herdr status` shows `server_binary_stale`).
- **`thread restart` says a half-made worktree needs a human look**: a failed `git worktree add` can leave the branch behind. Run `thread resolve`, delete or reuse the branch yourself, and start a new thread.
- **`thread resolve --remove-worktree` refuses**: either the worktree has uncommitted changes (Herdr's refusal is shown unchanged; nothing is ever forced), or part of the thread's files could not be copied home first. The message lists what was not copied; `--discard-uncopied` accepts that loss.
- **No nudge reaches the coordinator**: that is the default. See [Operations](operations.md#nudges-and-notifications).
