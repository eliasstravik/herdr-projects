# Your first board

This lesson takes you from no install to an open board. At the end, you have a key that opens the board, and you
know the first parts of the board that you will use.

The lesson gives only the steps that you need now. The full detail of each step is in
[Install and update](../how-to/install-and-update.md) and [Install details](../reference/install-details.md).

## Before you start

You need these items:

- Linux or macOS.
- [Herdr](https://herdr.dev) 0.9.3 or newer.
- An agent program that Herdr supports: Claude Code, Codex, pi or another one.
- `git`, and `curl` or `wget`.

You do not need Node or npm. The board is one program.

**Note:** If you used herdr-superwork or herdr-projects 0.2 before, you do not have to do more. The first run
takes over the old install by itself. If you used the older Node version, you must remove it from your PATH. See [Move from an old install](../how-to/move-from-an-old-install.md).

## Step 1: Install the plugin

1. In a shell, run this command:

   ```sh
   herdr plugin install eliasstravik/herdr-projects
   ```

Herdr gets the plugin and downloads the program. You do not have to build anything.

## Step 2: Open the board for the first time

1. In Herdr, run the plugin action **Open the work board**.

   The board opens a first-run screen. The screen offers two additions to your Herdr `config.toml`:

   - **1.** A key that opens the board. The screen offers `prefix+a`.
   - **2.** Sidebar rows. With them, the entry of an agent in the Herdr sidebar shows a small copy of its task row.

   **Note:** If your config already uses `prefix+a`, the screen offers a different free key and tells you why. In
   this lesson, use that key in place of `prefix+a`.

2. If an item is off, press its number (`1` or `2`) to switch it on.
3. Press `Enter` to save.

The board writes the two items into `config.toml`. It makes a backup of the file first. Each item is between two
marker lines, so that the board can find it again later.

**Note:** If you press `Esc`, the board saves nothing. It asks you again the next time.

## Step 3: Put the command on your PATH

This step is optional. Do it if you want to type `herdr-projects` in a shell.

1. Add the folder `~/.herdr-projects/data/bin` to your PATH, in `~/.profile` or your shell's start-up file.
2. Open a new shell.
3. Type `herdr-projects` and press `Enter`.

The board opens in that pane.

## Step 4: Open the board with its key

1. In Herdr, press `prefix+a`.

The board opens in a tab of its own. From now on, you can open the board in one of these two ways:

- Press `prefix+a` in Herdr.
- Type `herdr-projects` in any pane.

## Step 5: Look at the board

Look at the top of the board, the header. The header names the repo that new tasks go to. This is the repo that
you opened the board in.

One board holds the tasks of every repo on this machine. The tag of each task names its repo, for example `PW-12`
or `HSW-3`. Press `Alt+S` to group the list by repo.

1. Press `?`.

   The board shows all of its keys.

2. Read the list, then close it.

## What you learned

- You installed the plugin with one command.
- You added a key and sidebar rows on the first-run screen.
- You opened the board with `prefix+a` or with `herdr-projects`.
- You found the repo for new tasks in the header, and the key list under `?`.

To get a newer version later, run `herdr-projects update`.

## Next lesson

In the next lesson, you give the board its first task. You type what must be done and press `Enter`: the
Coordinator makes it a task. If you press `Tab` in place of `Enter`, the board also puts an agent on the task
immediately. Continue with [Your first task](first-task.md).

## Related pages

- [Your first task](first-task.md)
- [Install and update](../how-to/install-and-update.md)
- [Install details](../reference/install-details.md)
- [The board header and list](../reference/board-header-and-list.md)
- [Keys](../reference/keys.md)
- [Troubleshoot](../how-to/troubleshoot.md)
