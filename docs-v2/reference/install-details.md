# Install details

This page gives the facts about the install: the program files, how `bin/run.sh` picks the program, the first-run
screen, the launcher and how an update is loaded. For the steps, see
[Install and update](../how-to/install-and-update.md).

## Commands

| Command | What it does |
|---|---|
| `herdr plugin install eliasstravik/herdr-projects` | Installs the plugin from the main branch. Add `--ref v1.2.3` for a given release. |
| `herdr plugin link "$PWD"` | Links a checkout of your own as the plugin: hooks, the board's tab and the actions. |
| `sh bin/run.sh install` | Gets the program, writes its launcher, and says how to put it on your PATH. |
| `sh bin/run.sh --fetch` | The build step of the plugin. It only gets the program. If it cannot download the program, it builds it from source. |
| `herdr-projects install` | Writes the launcher again. Says if a different `herdr-projects` comes first on your PATH. Ends with the same lines as `doctor` about older installs. |
| `herdr-projects doctor` | Looks at the install and says where things stand. See [Move from an old install](../how-to/move-from-an-old-install.md). |
| `herdr-projects update` | Installs the newest release. |
| `herdr-projects update 0.3.1` | Installs a given version. |
| `herdr-projects remove-extras` | Removes the marked blocks from `config.toml` and the how-to files. See [Remove herdr-projects](../how-to/remove.md). |
| `herdr-projects machines` | Checks this machine and each other machine. See [Set up another machine](../how-to/set-up-another-machine.md). |

## Program files

The program is one file for each platform:

- `herdr-projects-linux-x86_64`
- `herdr-projects-linux-aarch64`
- `herdr-projects-macos-x86_64`
- `herdr-projects-macos-aarch64`

The Linux files run on any Linux. `.github/workflows/release.yml` builds the release files for each version tag.

## How `bin/run.sh` picks the program

Each command that Herdr runs for the plugin goes through `bin/run.sh`. It runs the first of these that it finds:

1. A program built in this checkout (`target/release/herdr-projects`), if no source file is newer than it.
2. The downloaded program in `bin/`, if it is the version of this checkout (`Cargo.toml`) or newer.
3. The same file, downloaded now from the GitHub release of the version of this checkout. `bin/run.sh` checks its
   checksum and puts it in place in one step. It does only one download at a time.
4. If none of these is available, an older build or an older download. A line tells you that it is older.

If a download fails, `bin/run.sh` tries again ten minutes later, not on each command. If it tried on each command,
each Herdr hook would wait for the download.

If there is no program at all, `bin/run.sh` says what to do and stops immediately. Thus it never holds up Herdr.

### Environment variable

| Variable | What it does |
|---|---|
| `HERDR_PROJECTS_RELEASES` | Names a different place for the releases: a fork, a mirror, or a folder as `file:///…`. |

## The launcher

The launcher is `~/.herdr-projects/data/bin/herdr-projects`. It runs the program. Agents and hooks get it by its full
path.

- The board writes the launcher when it opens.
- `herdr-projects install` writes it again.
- To type `herdr-projects` in a shell, put `~/.herdr-projects/data/bin` on your PATH.

## The first-run screen

The first time that the board opens inside Herdr, it shows one screen. The screen offers two items for your Herdr
`config.toml`. Each item is your choice.

| Key | What it does |
|---|---|
| `1`, `2` | Switches each item on or off. |
| `k` | Picks a different key. |
| `Enter` | Saves. |
| `Esc` | Saves nothing, and asks again the next time. |

The board makes a backup of the file first. Each block is between two marker lines. If Herdr rejects a change, the
board puts the file back as it was.

### Item 1: a key that opens the board

- The screen offers `prefix+a`. Herdr does not use this key itself.
- If your config already binds `prefix+a`, the screen offers the next free key and says why. The order is
  `prefix+y`, then `prefix+u`, `prefix+i` and `prefix+m`.
- The screen never takes a key that Herdr binds by default, or that your config already binds. It refuses the key
  and says what the key does now. For example, `prefix+w` is the Herdr workspace picker.
- This rule applies to the offered key and to a key that you type after `k`. It applies to each form of a binding in
  your config: one key, a list of keys, or the older `[keys.indexed]`.
- If your config already has a key for the board, the board does not change it. This applies to each form:
  `[[keys.command]]`, or `command = [{…}]` under `[keys]`.
- The key opens the board in a tab of its own. To open the board as a popup over the Herdr window, run
  `herdr-projects set board.open popup`. The same key then also closes the popup. The key itself stays the same.
  See [Settings](settings.md).

### Item 2: sidebar rows

With these rows, the entry of an agent in the Herdr sidebar is a small copy of the row of its task on the board
(see [The sidebar](sidebar.md)):

- the number and the title of the task;
- who is on it and where it stands, with the review round and the pull request (`agent · working · R#1 PR#113`);
- the percent and the time to go (`8% 16m`).

Without these rows, Herdr shows the first two lines, with the name of the tab (`Agent`) after the title. It does not
show the third line, because the default Herdr sidebar shows no custom value.

- If an older version of the plugin wrote rows, the board updates them by itself, between the same marker lines.
- The board never changes rows of your own. If your config already sets `ui.sidebar`, the board does not offer its
  rows. This applies to each form: `[ui.sidebar.agents]`, `sidebar = {…}` under `[ui]`, or a dotted key.
- In that case, the first run says in one line that it keeps your rows. It shows the rows of the plugin, so that you
  can merge them into yours by hand.
- In a small window, `↑` `↓` and `pgup` `pgdn` scroll these rows. Any other key continues.

The text of the blocks is in [Install and update](../how-to/install-and-update.md#add-the-config-blocks-by-hand).

## How an update is loaded

- `herdr-projects update` puts the release in place of the downloaded program, in one step.
- An open board and the keeper see that the file changed, and load it by themselves. See
  [Board size](../explanation/board-size.md).
- If you pull a newer plugin (`git pull`), the same thing occurs at the next command that Herdr runs: `bin/run.sh`
  downloads that version.
- A program that you built yourself updates only with `git pull` and `cargo build --release`, never by a download.

### How the keeper updates itself

1. The keeper waits until the new file stops changing. It also waits for a build that is still being linked.
2. It waits until no board is open and no agent runs.
3. It starts its successor on the new file.
4. It stops only when the successor runs.

If the new file does not start, the old keeper continues to run. It tries again a little later. `keeper.log` in the
data folder records this.

## Related pages

- [Install and update](../how-to/install-and-update.md)
- [Your first board](../tutorials/first-board.md)
- [The home folder](home-folder.md)
- [Commands](commands.md)
- [Settings](settings.md)
- [The sidebar](sidebar.md)
- [Remove herdr-projects](../how-to/remove.md)
