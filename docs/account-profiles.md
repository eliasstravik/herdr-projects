# Account profiles

Choose an account per repo or task, independently of program/model/effort. An account profile is a named environment setup in your own `~/.herdr-projects/settings/profiles.json` (or `HERDR_PROJECTS_CONFIG_DIR/profiles.json`):

```json
{
  "main": {"env": {"CLAUDE_CONFIG_DIR": null}},
  "channels": {"env": {"CLAUDE_CONFIG_DIR": "~/.claude-channels"}}
}
```

`null` means unset, not an empty string. Before any profile starts, variables managed by other profiles are unset too: a reused pane must not keep the previous account. `none`, the default profile setting, unsets all variables managed by profiles. With no profiles file, it changes nothing. The profile file is user configuration, never a fact in the repo's committed `.herdr-projects.json`. Do not store credentials in profiles: their resolved environments are kept with the task and shell launch lines.

Program, model, effort and access use the existing typed settings, for example `worker.agent claude/sonnet`. Profiles contain only `env`; there are no untyped launch flags or agent aliases.

```sh
herdr-projects set worker.profile main
herdr-projects set worker.profile channels --repo my-repo
herdr-projects set reviewer.profile channels --repo my-repo
herdr-projects set plan.profile channels --repo my-repo
herdr-projects set coordinator.profile main
herdr-projects set watcher.profile main

herdr-projects add "Use the other account" --repo /path/to/repo --worker.profile channels
herdr-projects delegate MYR-12 --worker.profile channels
herdr-projects add "Create and delegate" --repo /path/to/repo --worker.profile channels --delegate
```

These flags work for a Coordinator too. A task's selection wins over its repo's selection and your global setting. `reviewer.profile` and `plan.profile` are separate choices: a worker's override does not silently change the reviewer's account. The Coordinator and Watcher are machine settings. The Watcher batches screens from several repos into one call, so it uses its own account, not each worker's.

The environment is resolved on the executing machine and pinned in `agentEnvs` when a conversation starts. Existing conversations keep that map when settings change, including when the program is restarted. Select another account before first delegation; to move work already in progress to a different account, create a new task. Each new review chooses and pins its reviewer profile. A separate planner's map is discarded with its conversation when the worker begins. Token debt stores the original map, so later reads of past conversations use the original account too.

Herdr 0.9.3 has no `agent start --env`. Before the canonical `agent start`, the board types quoted exports/unsets into an idle pane shell, waits for an exact-line acknowledgement and confirms the shell is idle again. The kind, integration, agent name, hook arguments and resume flags stay Herdr's. Planning mounts use the same preparation. Direct Watcher children receive the environment with `Command::env`/`env_remove`. Environment remains in that task pane after the agent exits; the next launch clears managed values. A user-defined shell function named after the agent can override a profile: make that function honor the selected account or use the actual executable.

Readers have a nested thread-local environment scope, restored on drop (also on panic), without changing the board process's environment. This covers Claude/Codex/pi homes, conversation search, cached chat text, turn-end/error handling, submission checks, hooks, trust, token accounting and Coordinator reads. Claude trust follows the selected config directory (`$CLAUDE_CONFIG_DIR/.claude.json`), or `~/.claude.json` when unset. Skills and cleanup of old global hooks still operate on the process's installation home; run migration cleanup explicitly for each account during a planned cutover.

Variables use `[A-Z_][A-Z0-9_]*` names (up to 64 bytes). Runtime/shell variables such as `PATH`, `HOME`, `SHELL`, `IFS`, `BASH_ENV`, `LD_*`, `DYLD_*` and `HERDR_*` are refused. Values are one printable line without backslashes, up to 1024 bytes. Only `~`/`~/`, `${HOME}` and `$HOME/` (or trailing `$HOME`) expand, locally; no shell expression is evaluated. Agent home variables must expand to absolute paths. Bash and zsh are supported by the pane preparation; other shells require verification.

Profiles must be defined separately on each machine. Do not send an expanded Linux account path to a Mac. Remote account mapping, live account switching, provider login and authentic paid requests are outside the sandbox checks.
