# Move from an old install

Use this page if you used an older program before this version: herdr-superwork, herdr-projects 0.2, the Node
version, or a version that kept its files in `~/.local/state`, `~/.config` and `~/.local/share`. It also tells you
how to undo the move.

## Find what you must do

| Older program | What you must do |
|---|---|
| **herdr-superwork** (the working title of this version) | Nothing. The first run moves it into `~/.herdr-projects` and carries it on. Tasks, keys and history stay. |
| **herdr-projects 0.2** (the old Rust program, now on the `legacy` branch) | Nothing. The first run sets it aside in `-legacy` folders and never mixes it in. |
| **The older Node version** | Remove the Node program from your PATH (see below). |
| **A version with files in `~/.local/state`, `~/.config` and `~/.local/share`** | Nothing. The first run moves the files. |

"The first run" is any run of this version: the board, a command, or the hook that Herdr calls. The first run
deletes nothing. The full list of what it does is in
[What the takeover of an old install does](../reference/old-installs-takeover.md).

## Move from herdr-superwork or herdr-projects 0.2

1. Open the board, or run any `herdr-projects` command.

   The first run takes over both older programs by itself. It says what it did in one line, on the terminal or on
   the board when the board next opens.

2. Run `herdr-projects doctor`.

`herdr-projects doctor` looks again at any time. It takes over what is left and says where things stand.
`herdr-projects install` ends with the same lines. If you run either command again, it changes nothing.

If Herdr rejected a change to its config, `doctor` says so and tries again.

## Move from the older Node version

1. Run this command:

   ```sh
   npm unlink -g herdr-projects
   ```

2. Open the board.

The first run of this version keeps a copy of the data folder as it was, in `~/.herdr-projects/data-0.2.0`. It
makes the copy before it changes anything. The form of the data is the same, so the board converts nothing.

## Move from a version with files in `~/.local/state`, `~/.config` and `~/.local/share`

Older versions kept their files in three places:

- the data in `~/.local/state/herdr-superwork`;
- the settings in `~/.config/herdr-superwork`;
- the reports of agents in `~/.local/share/herdr-superwork-reports`.

You do not have to do anything. The first run of a new version moves all three into `~/.herdr-projects/` by itself.
Then it says so one time, in one line, the first time that you can see it: on the board when it opens, or after a
command that you typed. The line is:

    Moved the board's files into ~/.herdr-projects: data (from ~/.local/state/herdr-superwork), settings (from
    ~/.config/herdr-superwork), reports (from ~/.local/share/herdr-superwork-reports). The old places are links to
    the new ones, so whatever is running carries on.

If a folder is on a different disk, or the system cannot swap two paths in one step, the folder stays where it is.
The line then says so. If you set `HERDR_PROJECTS_DATA_DIR`, `HERDR_PROJECTS_CONFIG_DIR` or
`HERDR_PROJECTS_REPORTS_DIR` to a place of your own, the board does not move that part.

How the move keeps your data safe is in [The home folder](../explanation/home-folder.md#how-the-one-time-move-keeps-your-data-safe).

## Remove the old links

After a move, the old paths are links to the new folder. The board does not remove them.

**CAUTION:** Do not remove an old link while a program that started before the move still runs. That program can
still use the old path.

1. Make sure that nothing that started before the move still runs.
2. If you moved from herdr-superwork, remove the link `~/.herdr-superwork`.
3. If you had the three older places, remove the three links in `~/.local/state`, `~/.config` and `~/.local/share`.

## Remove old folders from your repos

Older versions had agents write into a `.herdr-superwork/` folder inside the folder where they worked. The board
removes only the old note of a task, when that task is cleaned up. It leaves everything else there. See
[The home folder](../reference/home-folder.md#leftovers-from-older-versions).

**CAUTION:** Do not delete the folder while an agent from before the update still runs.

1. Make sure that no agent from before the update still runs.
2. Delete the `.herdr-superwork/` folder yourself.

## Use herdr-projects 0.2 again

The old version finds its projects in the set-aside folder with this variable:

```sh
HERDR_PROJECTS_ROOT=~/.herdr-projects-legacy
```

Only the old version reads `HERDR_PROJECTS_ROOT`. If it is set, `doctor` tells you this.

## Undo the takeover of herdr-superwork or herdr-projects 0.2

1. Stop the board.
2. For herdr-projects 0.2: rename the `-legacy` folders back to their old names.
3. For herdr-superwork: remove the link `~/.herdr-superwork`.
4. For herdr-superwork: rename `~/.herdr-projects` back to `~/.herdr-superwork`.
5. For herdr-superwork: make `~/.herdr-projects` a link to `~/.herdr-superwork`. You must do this because the paths
   written inside the files now name `~/.herdr-projects`.
6. If you need the backups of the Herdr config and of the agent settings, find them beside those files
   (`.bak-herdr-projects`).
7. For herdr-superwork: run `herdr plugin enable herdr-superwork` to switch its plugin on again.

## Undo the move from `~/.local/state`, `~/.config` and `~/.local/share`

1. Stop the board.
2. Remove the three links.
3. Move `data`, `settings` and `reports` back to the old places.
4. Run the old version.

The move changed nothing else.

## Related pages

- [What the takeover of an old install does](../reference/old-installs-takeover.md)
- [The home folder (reference)](../reference/home-folder.md)
- [The home folder (explanation)](../explanation/home-folder.md)
- [Install and update](install-and-update.md)
- [Troubleshoot](troubleshoot.md)
