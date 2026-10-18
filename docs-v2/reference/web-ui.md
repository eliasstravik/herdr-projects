# The web board

This page gives the facts about the web board: the board in the Herdr web UI. Use it to find a command, an address,
a part of the **Boards** section, or how the page works. For the web control of each part of the terminal board,
refer to [the parts of the web board](web-ui-parts.md).

The web board runs only if the [Herdr web UI](https://github.com/devswha/herdr-web-ui) plugin is installed beside the
herdr-projects plugin. Without the web UI plugin, no part of it runs.

## Commands

| Command | What it does |
|---|---|
| `herdr-projects web-ui` | Shows the status: the web UI, the front door, and which links go through the front door. If there are two links, it lists both and tells which link has the Board. It also prints the address of the front door. |
| `herdr-projects web-ui link` | Points the shared link of the web UI at the front door. |
| `herdr-projects web-ui unlink` | Points the shared link back at the web UI. |
| `herdr-projects remove-extras` | Points the link back and stops the front door. |
| `herdr-projects set board.webUi none` | Turns off all of the web board and keeps the rest of the board. The front door points the link back and closes. `board.webUi` is a setting ([settings](settings.md)). |
| `herdr-projects serve` | The one page server of the board. It holds the front door and also serves the evidence pages ([task panel](task-panel.md)). |

## Addresses, files and names

| Name | What it is |
|---|---|
| `http://127.0.0.1:7327/` | The address of the front door on the machine itself, if that port was free. `herdr-projects web-ui` prints the real address. |
| `127.0.0.1` | The only address where the front door listens. |
| `/herdr-projects/` | The path where the front door serves the page files and the calls of the board. The web UI does not use this path. |
| `data/front-door.json` | The bookkeeping of the front door in [the home folder](home-folder.md): its process, its port, and where it sends requests. |
| `data/extras.json` | The record of the `tailscale serve` entries that the link step changed. |
| `x-herdr-projects` | The header of the board that each call that changes something must have. |
| `GET /api/machines` | The call that the front door uses to ask the web UI for its PCs, with your own login. The front door keeps nothing from it. |
| `GET /api/access` | The answer of the web UI to its Settings → Phone. The front door corrects one part of it ([explanation](../explanation/web-ui.md)). |
| `HERDR_PROJECTS_WEBUI` | Names a web UI that Herdr does not list as a plugin ([contributing](../contributing/web-ui.md)). |

## The Boards section

The **Boards** section is in the sidebar of the web UI, under **Agents** and above the foot of the sidebar
(**Install app**, **Settings**). It is the one place of the board in the sidebar. It replaces the single Board entry
that was at the foot of the sidebar.

- The heading has the same type, spacing and caret as the **Agents** heading, and it folds in the same way.
- On a phone, the section starts folded, as **Agents** does, because the sidebar is a drawer there.
- On a wide screen, the section starts open. A fold stays while the page is open.
- When the section is folded, the heading shows the dot of all the boards together.
- If the web UI draws its sidebar again, the script puts the section back in its place. On a phone, this occurs each
  time that the drawer opens.

### The entries

The section has one entry for each machine that has a board. Each entry looks like the entry of an agent.

- **This machine** and **each other machine that the board knows**. These are the machines that **Other machines**
  goes round: the saved machines of Herdr, or `board.machines`.
- The entries are in the order of the Herdr sidebar. This machine is where that sidebar puts it
  ([machine order](../explanation/machine-order.md)).
- **All machines** is the last entry. It shows also when this machine is the only machine. It then shows the tasks of
  this machine, and stays selected.

| Part of an entry | What it shows |
|---|---|
| The name | The name that the web UI gives to that machine, for example `Mac`, `elias-macbook-pro-m1` or `vm-str` ([other machines](#other-machines)). If the web UI has no PC for a machine, the entry uses the name of the board. If the web UI gives this machine no name of its own, the entry says `This machine`. |
| The dot | The status colours of the board, in the place of the status of an agent. Red: a task needs you. Green: a task is ready to approve. The working colour: work continues. Where the web UI has a breathing working dot, this dot breathes too. Dim grey: all is quiet. |
| The second line | For example `1 needs you · 2 working · 3 to do`, with `to approve` for a task that is ready to approve, or `all quiet`. Archived tasks and done tasks are not counted. |
| A machine that the board cannot reach | `not reachable`, dim as an agent on a PC that is away, with a ring for a dot. Point at it to see the reason, in the words of the board. |
| **All machines** | The sum of the machines that answered, and the number that did not answer, for example `· 1 not reachable`. |
| A machine that the board asks for the first time | `asking…` |

### Picking an entry

When you pick an entry, the board of that machine opens over the main area, with its tasks and its Coordinator. The
entry is highlighted as a selected agent is.

- An entry does the same as a choice of that machine in **Other machines** on the board. The board then tells you, for
  example `Only vm-str. What you type goes to vm-str.`
- The two stay in step. **Other machines** on the board, and the board that follows the web UI to another PC, also move
  the highlight.
- On a phone, the drawer closes, as it does when you pick a pane.
- The section asks for new data every four seconds while the page is in view.
- The board asks the other machines for their tasks every twenty seconds at most, as the terminal board does.

## The web board and the terminal board

The web board does all that the terminal board does, with buttons and touch in place of keys. For the part of the
web board that does the job of each part of the terminal board, refer to [the parts of the web board](web-ui-parts.md).

## The page

- The page names no terminal key. The settings of a task point to **All settings** below them, not to `Ctrl+E`. A
  setting that is the task's own says so in words, not with a star.
- There is one back arrow at a time. On a phone, the arrow of the open task replaces the arrow of the Board.
- No text that you read is smaller than 13 px.
- Each thing that you tap is a minimum of 40 px high. The two send controls of the chat box have the sizes of the web
  UI: 32 px and 34 px with a mouse, 40 px on touch.
- The header names the repo, with no folder path.
- Every [key](keys.md) of the board is a button or a menu entry, except the keys that belong to a terminal. These are:
  close the terminal of the board, the `?` menu, the side panel, and full screen. The page scrolls, and shows the list
  and the task side by side when the window is wide enough.
- The chat keys are the controls of the chat: the switch, **Hide** and **Show**, **Earlier** and **Later**, and `Tab`
  and `Enter` in the box ([the web board's chat box](web-chat-box.md)).

### Refresh and browsers

- While the page is open, it asks for new data every two seconds. It draws again only what changed.
- Each browser has its own selection, question and message. Thus a phone and a desktop do not interrupt each other.
- What you type stays in that browser tab until you send it.

### Who can do what

- A user who is not logged in to the web UI cannot read or change any part of the board.
- A device that the web UI paired to *watch* can look at the board. It can select, find, and show or hide. It cannot
  send, approve, stop or archive.
- The script and style files of the page hold no task. They are public, as the files of the web UI are.

For how the front door checks this, refer to [the explanation](../explanation/web-ui.md#who-gets-in).

## Other machines

The web UI has machines of its own, its **PCs**: this machine, and the PCs added in its Settings → Remote PCs. The board
has the saved machines of Herdr ([other machines](../explanation/other-machines.md)). The board matches the two:

1. By the Herdr id of the saved machine, if the web UI names it.
2. Else by how they connect: the same host, port and Herdr session, and the same user if both name one.

| What | How it works |
|---|---|
| Every machine, or one | An entry of the **Boards** section shows the board of that machine, or of all machines. `More` → **Other machines** goes round as `Alt+M` does: this machine, every machine, each machine alone. A chip tells which. A tap on the chip picks another. Its × goes back to this machine. |
| Buttons and words | Every button works on the task of another machine as on the terminal board. What you type while one machine shows goes to the Coordinator of that machine. |
| The first view | A page opens on the choice of the terminal board (this machine, every machine, or one), so both boards show the same tasks. A board picked on the web stays for the next page. Pick the choice of the terminal board again to make the web follow the terminal board again. |
| **Open this task's agent** on the task of another machine | That machine opens the space of the task again and starts an agent that exited, as it does for the terminal board. The page goes to the view of that pane in the web UI, on that PC (`?machine=<its PC>&pane=…`). The work never moves: only the page does. |
| The web UI on a pane of another PC | The Board opens as the board of that machine. Back on a pane of this machine, it shows what it showed before. Your own choice stays while the web UI stays where it is. |
| A task on another machine | It shows and acts as on the terminal board. Its agent's pane is on that machine: the board tells you so in place of opening it. |

The board tells you what is missing. It does not hide it:

- The web UI has no PC for that machine. Add it in Settings → Remote PCs.
- The PC is not connected in the web UI.
- The web UI did not tell which PCs it has (an older web UI).
- The other machine did not answer.
- The other machine has a plugin that is too old to open a space again. The page opens the pane that it named last,
  and the board tells you to update.
- The web UI is on a PC for which the board knows no machine. Add it with `herdr machine add`.

Each machine with both plugins also has a front door, and a Board of its own at the link of its own web UI.

## Related pages

- [Set up the web board](../how-to/set-up-the-web-ui.md)
- [The parts of the web board](web-ui-parts.md)
- [The web board's chat box](web-chat-box.md)
- [The terminal board in the web UI's terminal view](web-ui-terminal-view.md)
- [How the web board works](../explanation/web-ui.md)
- [Your first look at the web board](../tutorials/web-board.md)
- [Settings](settings.md)
- [Other machines](../explanation/other-machines.md)
