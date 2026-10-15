# Get started

You need [Herdr](https://herdr.dev) 0.9.3+ on macOS or Linux, and Claude Code, Codex, pi or another agent herdr supports.

## Option A: do it yourself

1. Install:

   ```sh
   herdr plugin install eliasstravik/herdr-projects
   ```

2. In Herdr, run the action **Open the work board**. Say yes to the key it offers (`prefix+a`).
3. Optional: add `~/.herdr-projects/data/bin` to your PATH so you can type `herdr-projects`.

## Option B: let an agent do it

Paste this into Claude Code, Codex or pi, running inside Herdr:

```text
Install the herdr-projects plugin for Herdr and set it up for me.
1. Check `herdr --version` is 0.9.3 or newer, and that git and curl (or wget) exist. Stop and tell me if not.
2. Run `herdr plugin install eliasstravik/herdr-projects`.
3. Find the plugin's folder with `herdr plugin list` and run `sh <that folder>/bin/run.sh install`.
4. Add ~/.herdr-projects/data/bin to my PATH in my shell's start-up file, so `herdr-projects` works in new shells.
5. Run `herdr-projects doctor` and fix anything it reports.
6. Open the board: `herdr plugin action invoke open --plugin herdr-projects`.
Then tell me in two lines: it is ready, and to say yes to the key it offers (prefix+a) on its first screen.
Details if you need them: https://github.com/eliasstravik/herdr-projects/blob/main/docs/install.md
```

## Start it

- Press `prefix+a` in Herdr, or type `herdr-projects` in any pane.
- Type what needs doing and press `Enter`. The Coordinator makes it a task.
- Press `Tab` instead of `Enter` to put an agent on it right away.
- Press `?` to see everything else.

Update later with `herdr-projects update`.

More detail (building from source, older installs, uninstalling): [installing](install.md).
