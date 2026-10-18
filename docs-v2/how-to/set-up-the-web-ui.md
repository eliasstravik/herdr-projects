# Set up the web board

This page tells how to show the board in the Herdr web UI, how to link and unlink it by hand, and how to turn it off.
For what each part of the web board does, refer to [the web board](../reference/web-ui.md).

## Before you start

- You must have the herdr-projects plugin ([install and update](install-and-update.md)).
- You must have the [Herdr web UI](https://github.com/devswha/herdr-web-ui) plugin. Without it, no part of the web
  board runs.

## Show the board in the web UI

**Note:** When both plugins are installed, the front door of the board runs on this machine, in front of the web UI.
It starts with Herdr, and the keeper keeps it running.

1. Install both plugins.
2. Share the web UI as it tells you. Its Settings → Phone shows the `tailscale serve` command.
3. Open the board in the terminal.
4. When the board asks *Show this board in the herdr web UI?*, select **Yes**.

**Yes** points the shared link of the web UI at the front door. **No** leaves the link as it is, and the board does not
ask again. **Later** asks again the next time that the board opens.

This one change is all of the setup.

## Link or unlink by hand

You can do these at any time:

    herdr-projects web-ui            how things stand: the web UI, the front door, which links go through it
    herdr-projects web-ui link       point the web UI's shared link at the front door
    herdr-projects web-ui unlink     point it back

- To see the status, run `herdr-projects web-ui`. If there are two links, it lists both and tells which link has the
  Board.
- To point the link at the front door, run `herdr-projects web-ui link`.
- To point the link back at the web UI, run `herdr-projects web-ui unlink`.

## Open the board on the machine itself

1. Run `herdr-projects web-ui` to see the address of the front door.
2. Open that address in a browser on the same machine.

The address is `http://127.0.0.1:7327/`, unless another program had that port.

## Turn the web board off

To turn off all of the web board and keep the rest of the board, do this step:

1. Run `herdr-projects set board.webUi none` ([settings](../reference/settings.md)).

The front door points the link back without help, and closes.

To point the link back and stop the front door as a part of a removal, run `herdr-projects remove-extras`
([remove](remove.md)).

## Show the board of another machine

1. In the sidebar of the web UI, open the **Boards** section.
2. Pick the entry of the machine, or **All machines**.

Or use the board itself:

1. Open **More…**.
2. Pick **Other machines**. Each pick goes to the next choice: this machine, every machine, each machine alone.

To go back to the tasks of this machine, tap the × on the chip under the header.

## Show the tasks of a machine without the web UI

Do these steps if the web UI runs on one machine and the tasks are on another machine.

1. On the machine with the web UI, install this version of the herdr-projects plugin.
2. On that machine, save the machine with the tasks in Herdr with `herdr machine add`, or add it to `board.machines`.
3. Make sure that the machine with the tasks is reachable over SSH, with `herdr-projects` on its path.
4. On the machine with the tasks, update the plugin to this version. If you do not, its terminal board does not
   stay in place in the terminal view of the web UI.

The machine with the tasks needs no web UI and no front door. The Board shows the tasks of that machine when the web UI
is on one of its panes. It also shows them through `More` → **Other machines**.

## Fix a missing machine

If the board tells you that a machine is missing, do the step for that message:

- If the web UI has no PC for that machine, add the machine in Settings → Remote PCs of the web UI.
- If the web UI is on a PC for which the board knows no machine, run `herdr machine add`.
- If the board tells you to update, update the plugin on the other machine.

## Type a key in the terminal view

If a key does not work in the terminal board in the terminal view of the web UI, do this:

1. Type `?` to open the menu. On a phone, type `?` into the input line and send it.
2. Pick the line of the key in the `?` menu.

On a phone, you can also push `Ctrl` or `Alt` on the key bar, then type the letter with direct typing on. For all the
keys, refer to [the terminal view](../reference/web-ui-terminal-view.md).

## After the web UI updates

You do not have to do anything. The front door changes none of the files of the web UI.

## Related pages

- [Your first look at the web board](../tutorials/web-board.md)
- [The web board](../reference/web-ui.md)
- [The web board's chat box](../reference/web-chat-box.md)
- [How the web board works](../explanation/web-ui.md)
- [Install and update](install-and-update.md)
- [Remove](remove.md)
- [Set up another machine](set-up-another-machine.md)
