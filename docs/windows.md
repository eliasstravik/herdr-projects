# Windows

**Herdr Projects runs on Windows too, in preview, as Herdr's own plugins do there.** Herdr runs natively on Windows
(ConPTY panes, named pipes instead of Unix sockets); the board is the same program, built for Windows.

## Installing

The same command as everywhere, in a Herdr pane (PowerShell or Windows Terminal):

```powershell
herdr plugin install eliasstravik/herdr-projects
```

What you need:

- Windows 10 or 11 on x86_64 (Windows on Arm runs the x86_64 program).
- Herdr 0.9.3 or newer, installed natively (`irm https://herdr.dev/install.ps1 | iex`).
- [Git for Windows](https://git-scm.com/download/win). Its `sh` also runs your repo's `setup`, `check` and `teardown`
  commands, so they work the same as on Linux and macOS; without it they run in `cmd.exe`.
- Optional: `gh` (pull requests) and `tailscale` (private review links).

## How it differs

| | Linux and macOS | Windows |
|---|---|---|
| What herdr runs | `bin/run`, which runs `bin/run.sh` | `bin\run.exe`, the program itself |
| Getting the program | `bin/run.sh --fetch` | `bin/run.ps1`, the build step |
| The release file | `herdr-projects-<os>-<arch>` | `herdr-projects-windows-x86_64.exe` |
| Talking to herdr | its Unix socket | its named pipe |
| The launcher agents call | `~/.herdr-projects/data/bin/herdr-projects` | `…\data\bin\herdr-projects.cmd` |
| Repo commands (`setup`, `check`) | `sh -lc` | Git for Windows' `sh -lc`, else `cmd /c` |
| Stopping a task's processes | its process group | its process tree (`taskkill /T`) |

On Windows the build step removes `bin/run` (the Linux and macOS launcher) from the plugin's folder: Windows would find
that file before `bin\run.exe`. A plugin installed with `herdr plugin install` is a fresh copy at every update, so this
only shows in a checkout of your own linked with `herdr plugin link`.

## What does not work there yet

- The machine's load is not known, so the Watcher never slows down for a busy machine.
- The board does not ask the terminal for its colours: it takes `COLORFGBG`, or the dark theme.
- A plan's worktree is locked by making its files read-only; Windows still lets new files be added to its folders.
- Another machine's board reaches this one by its name only, not its addresses.

Something not working: say what in an issue, with your Windows version, Herdr version and terminal.
