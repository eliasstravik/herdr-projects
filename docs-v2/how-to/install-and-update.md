# Install and update herdr-projects

Use this page to install herdr-projects, to do the first-run setup, and to get a newer version. For a short lesson,
see [Your first board](../tutorials/first-board.md). If something does not work, see
[Troubleshoot](troubleshoot.md).

## What you need

- Linux or macOS, on x86_64 or aarch64.
- Herdr 0.9.3 or newer.
- A running Herdr server of 0.9.3 or newer. An older server refuses the plugin until you restart it.
- `git`, and `curl` or `wget` to download the program.
- An agent program that Herdr supports: Claude Code, Codex, pi or another one.
- Optional: `gh` for pull requests, and `tailscale` for private review links.

You do not need Node or npm. The board is one program.

## Install with Herdr

1. Run this command:

   ```sh
   herdr plugin install eliasstravik/herdr-projects
   ```

2. Open the board one time, to do the first-run setup (see [Do the first-run setup](#do-the-first-run-setup)).

Herdr clones the main branch of the repo into its own plugins folder. `herdr plugin list` shows where that folder
is. Herdr then runs the build step of the plugin there, which downloads the program.

To install a given release, add `--ref` and the release tag, for example `--ref v1.2.3`.

## Install from a checkout of your own

Do this if you want to keep your own checkout of the repo in place of the Herdr copy.

1. Run these commands:

   ```sh
   git clone https://github.com/eliasstravik/herdr-projects && cd herdr-projects
   herdr plugin link "$PWD"          # hooks, the board's tab and the actions
   sh bin/run.sh install             # gets the program, writes its launcher, says how to put it on your PATH
   ```

2. Open the board one time, to do the first-run setup.

## Let an agent install it

1. Start Claude Code, Codex or pi inside Herdr.
2. Paste this text into the agent:

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

3. When the board shows its first screen, accept the key that it offers.

## Do the first-run setup

The first time that the board opens inside Herdr, it shows one screen. The screen offers two items for your Herdr
`config.toml`: a key that opens the board, and sidebar rows. Each item is your choice.

1. Open the board. Run the plugin action **Open the work board**, or type `herdr-projects` in a pane.
2. If you want to switch an item on or off, press its number (`1` or `2`).
3. If you want a different key, press `k` and type the key.
4. Press `Enter` to save.

If you press `Esc`, the board saves nothing and asks you again the next time. The rules for the key and for the
sidebar rows are in [Install details](../reference/install-details.md#the-first-run-screen).

You need nothing more: no Herdr integration, and no hooks in the settings of your own agents.

## Add the config blocks by hand

Do this only if you do not want the first-run screen to write the blocks.

**CAUTION:** A `[[keys.command]]` block wins over the Herdr binding of the same key, and Herdr gives no warning. If
you write it by hand on a key that Herdr uses, the Herdr action of that key stops working. (An early version of this
page showed `prefix+w` here. That removed the Herdr workspace picker.) `herdr --default-config` lists most of the
keys that Herdr binds. It does not list copy mode (`prefix+[`) or the keys that swap panes (`prefix+shift+h` `j` `k`
`l`).

1. Add these blocks to your Herdr `config.toml`:

   ```toml
   [[keys.command]]
   key = "prefix+a"
   type = "plugin_action"
   command = "herdr-projects.open"

   [ui.sidebar.agents]
   rows = [["state_icon", "machine", "workspace", { token = "tab", rules = [{ equals = "Agent", hide = true }, { starts_with = "Reviewer #", hide = true }] }], ["agent"], [{ token = "$summary", dim = true }, "$progress"]]

   [ui.sidebar.spaces]
   rows = [["state_icon", "workspace"], ["branch", "git_status"], [{ token = "$summary", dim = true }, "$progress"]]
   ```

2. If you want a different key for the board, change the `key` line.
3. Run `herdr server reload-config`.

You can also bind the action `herdr-projects.approve-current`. It works for the task that owns the focused pane. It
opens the board on that task, with the merge question open (`y` is yes, `n` or `Esc` is no). In other panes, the
action does nothing.

## Put the command on your PATH

Agents and hooks get the launcher `~/.herdr-projects/data/bin/herdr-projects`. The launcher runs the program.

**Note:** The board writes the launcher when it opens. After `herdr plugin install`, you must open the board one
time before the launcher exists. Use the plugin action **Open the work board**.

1. Add the folder `~/.herdr-projects/data/bin` to your PATH, in `~/.profile` or the start-up file of your shell.
2. Open a new shell.
3. Type `herdr-projects` to make sure that the shell finds it.

`herdr-projects install` writes the launcher again. It also tells you if a different program with the name
`herdr-projects` comes first on your PATH.

If other machines will read the tasks of this machine, a login shell must find `herdr-projects`. See
[Set up another machine](set-up-another-machine.md#put-herdr-projects-on-the-path-of-a-login-shell).

## Update

1. Run one of these commands:

   ```sh
   herdr-projects update           # the newest release
   herdr-projects update 0.3.1     # a given version
   ```

The command puts the release in place of the downloaded program, in one step. An open board and the keeper see the
new file and load it by themselves. The keeper also starts a new keeper on the new file. See
[Install details](../reference/install-details.md#how-an-update-is-loaded).

If you pull a newer plugin (`git pull`), the program also updates. At the next command that Herdr runs,
`bin/run.sh` downloads that version.

## Build from source

Do this if you want to run a program that you built yourself. You need Rust (https://rustup.rs).

1. In the checkout, run `cargo build --release`.

`bin/run.sh` then runs that build. It also uses your later builds in the same way. A program that you built
yourself never updates by a download. To update it, run `git pull` and then `cargo build --release`.

## Related pages

- [Your first board](../tutorials/first-board.md)
- [Install details](../reference/install-details.md)
- [Move from an old install](move-from-an-old-install.md)
- [Remove herdr-projects](remove.md)
- [Set up another machine](set-up-another-machine.md)
- [The home folder](../reference/home-folder.md)
- [Troubleshoot](troubleshoot.md)
