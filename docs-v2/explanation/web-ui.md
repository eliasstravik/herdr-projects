# How the web board works

This page tells how the board gets into the Herdr web UI, and why it works this way. For the facts, refer to
[the web board](../reference/web-ui.md).

## The board inside the web UI

If the [Herdr web UI](https://github.com/devswha/herdr-web-ui) plugin is installed beside the herdr-projects plugin,
the board is a part of that web UI. It is a **Boards** section in the sidebar of the web UI, with one entry for each
machine. It has the same link and the same login as the web UI.

The web board does what the terminal board does, with buttons and touch in place of keys, on a phone and on a desktop.
Without the web UI plugin, no part of it runs.

## The same code decides

The web board calculates nothing again. On the server, it *is* the terminal board, opened with no terminal. It has the
same list, selection, questions and actions.

- A button presses the key of the board. What you type goes to the board as `Enter` sends it.
- The code of the board draws the mark, the words and the columns of a row, the lines of the task panel, the hint line
  and the `?` menu.
- It draws them through a theme that names each colour and does not paint it. The page gives each name a colour.
- Thus the two boards cannot disagree, and a change to the terminal board is a change to the web board.

For the files that do this, refer to [the contributor page](../contributing/web-ui.md#the-code).

## What the board looks like there

The web board uses the colours, type, sizes and spacing of the web UI: its CSS variables (`DESIGN.md` in the web UI).
Thus it follows light, dark and every palette of the web UI without help.

- The web UI does not read the theme of Herdr. Thus the Board matches Herdr as the web UI shows it.
- The frame of the box that you type in tells who gets your words, as on the terminal board
  ([colours](colours.md)).
- That box is the chat input of the web UI, built again and measured against it
  ([why](web-chat-box.md)).

## What the web chat does differently, and why

- `Esc` does not put a draft away. The page keeps the draft in its box, and over a reload. Thus there is nothing for
  `Ctrl+Z` to bring back on the web. The terminal way stays on the terminal.
- The switch flips also while there are words in the box. The words stay locked to their receiver, and the box tells
  you so. (On the terminal board, only `Enter`, `Tab` and `Esc` act on a draft.)
- A scroll down past the end of an opened conversation, with the wheel or a swipe, does not close it. A scroll that
  runs on would close it under your finger. Thus only **Later** closes it.

## Why a front door

The web UI has no place for an add-on (checked on herdr-web-ui 0.4.8).

- Its pages allow scripts and connections only from its own address, and they cannot go in a frame (`server/static.ts`
  in the web UI). Thus the board must come from the same address as the web UI.
- A change to the files of the web UI does not work either. Its updater builds each release from the official code and
  must have a clean checkout. Each update would remove a patch, and a patch could stop the update.

Thus the front door stands in front of the web UI. The front door is a small server of the board on this machine. It
starts with Herdr, and the keeper keeps it running.

## What the front door does

- **It sends each request on as it is.** This includes pages, `/api/*`, the `/ws` socket, streams, ranged and
  compressed files, and the service worker. The same method, address and headers go in, in their own order and
  spelling. The same status, headers and bytes come out as they arrive. Only the two connection headers that each HTTP
  hop writes for itself (`Connection`, `Keep-Alive`) are its own.
- **It adds one line to the page of the web UI.** The line is a script, from the same address, that puts the **Boards**
  section into the sidebar. Nothing else in the page changes, and its security policy stays as sent. A page that the
  front door cannot read goes on as it is.
- **It corrects one answer.** This is what the Settings → Phone of the web UI gets about its own link
  (`GET /api/access`). The web UI finds its link from a `tailscale serve` entry that points at its own port. Through
  the front door, the entry points at the port of the front door, so the web UI would say that it has no link. It would
  then offer the command for a second link. Thus the front door gives the answer the link that the web UI already has.
  It sends each other part of the answer on as it came. It also sends the full answer on as it came if the answer
  already names a link or if it cannot read the answer.
- **It answers under one path that the web UI does not use**, `/herdr-projects/`: the page files and the calls of the
  board.
- **A fault of the board never breaks the web UI.** If a problem occurs on the side of the board, a request that is not
  for the board still goes through. The board does its work in a thread of its own. Thus, when it reads tasks, it never
  stops the keystrokes of a terminal on their way through.
- **It shows the same Herdr as the web UI.** It finds that Herdr as the web UI does when it starts. First, the session
  that started both plugins (Herdr gives each plugin its socket). Then a socket that the settings of the web UI name.
  This is not always the default session.
- **It follows the web UI** through its restarts and its own updates (the front door keeps its port). It also follows
  the web UI to another port when the web UI takes one. While the web UI is down, the address of the front door answers
  "not answering right now", and the board lets nobody in.

The front door is part of the one page server of the board (`herdr-projects serve`), which also serves the evidence
pages ([task panel](../reference/task-panel.md)). It is one long-lived process, not two. Its bookkeeping is
`data/front-door.json` in [the home folder](../reference/home-folder.md). When the plugin is updated, the process hands
over to a new process on the same port. A terminal that is open in the web UI connects again without help.

## What the link step changes, and how it goes back

The web UI does not share itself. It shows the `tailscale serve` command. Then it finds its link from where that entry
points. The link step changes only that: each private HTTPS entry whose root goes to the port of the web UI then goes to
the port of the front door.

- The board writes down what it changed, in `data/extras.json`.
- The way back touches only an entry that still points at the front door. It points that entry at the port where the
  web UI listens now, also if the web UI took another port in the meantime.
- The link step never touches an entry for another app, an entry for a sub-path, or an entry that is open to the
  internet.

### The two do not pull one link back and forth

Only your **Yes** (or `herdr-projects web-ui link`) takes a link. After that, the front door never takes a link without
you.

- The Settings → Phone of the web UI, opened at the link, continues to show that link as its own. This is because of
  the one answer that the front door corrects.
- The Phone setup of the web UI, run from Herdr on the machine, reads Tailscale itself. It finds no entry at its own
  port, and makes one more link on another port. That link goes straight to the web UI. The web UI finds that link from
  then on, and the board does not touch it.
- `herdr-projects web-ui` lists both links and tells which link has the Board.

### Why the front door has a port of its own

Tailscale proxies only to `127.0.0.1` (checked on Tailscale 1.102: it refuses a target on the other loopback address).
Thus the front door cannot use the same port number as the web UI.

### An offline copy

A browser can save the page of the web UI while the link goes through the front door (the offline copy of the web UI).
That copy keeps the one script line after the link points back. The browser then does not find the script, and nothing
shows. The next load online replaces the copy.

## Who gets in

The web UI protects the board as much as it protects itself.

- **The front door keeps no token, no cookie and no list of devices.** For each call of the board, it asks the web UI
  if that caller is logged in. It sends the headers of the caller exactly as they came. The web UI uses its token and
  its device gate (`server/auth.ts` and `server/access.ts` in the web UI).
- **Each answer that is not a plain yes is a no.** This includes no answer, an answer that the front door cannot read,
  and a web UI that is down.
- **A call that changes something must come from the page of the board.** It must have the same origin and the header
  `x-herdr-projects`. A page of another site cannot send that header. The web UI asks the same of its own controls.
- **Private only.** The front door listens on `127.0.0.1` and on no other address. The only way to it from another
  device is the private link of the web UI (`tailscale serve`). The board never points a link that is open to the
  internet (Tailscale Funnel) at it, and never makes one.

## The web UI on one machine, the tasks on another

The machine where the web UI runs serves the Board. Thus that machine must have the herdr-projects plugin too, on this
version. The other machine must be one of the saved machines of Herdr there (`herdr machine add`, or
`board.machines`), reachable over SSH with `herdr-projects` on its path. The machine
with the tasks needs no web UI and no front door.

The terminal board that runs on the machine with the tasks is the program of that machine. To stay in place in the
terminal view of the web UI, it must have this version of the plugin. For the steps, refer to
[set up the web board](../how-to/set-up-the-web-ui.md#show-the-tasks-of-a-machine-without-the-web-ui).

## The terminal board in the terminal view

Three problems made the terminal board unusable in the terminal view of the web UI
([the facts](../reference/web-ui-terminal-view.md)):

- **The view and the board took the terminal from each other.** The view of a pane in the web UI sets the size of that
  pane. The board took its size back from each such view, and the web UI connected again without help. Thus the two
  took the terminal from each other every four seconds. Now a view that somebody looks through keeps the terminal.
- **`Alt` keys and `Shift+Enter` did nothing.** The terminal of the web UI sends the old byte forms for them. The board
  now reads those forms too.
- **`?` closed again on a phone.** The input line of a phone sends its own `Enter` after the text. The board now
  ignores that `Enter` after `?`, `y`, `n` or space.

### Why the board does not go into an agent on another machine there

The way back from an agent is the Herdr chord `ctrl+b q`. The view of the pane in the web UI takes that chord for
itself. Thus the board would stay inside the agent. For this reason, the board does not change its terminal into the
agent of a task on another machine while the web UI shows it. **Open this task's agent** on the web board is the way
that works from a browser.

## When the web UI changes

- **The web UI updates itself:** you do not have to do anything. The front door changes none of its files.
- **A new design of the web UI removes the foot of the sidebar:** a small floating **Board** button shows in place of
  the **Boards** section, and tells you one time why. The board itself does not change.
- **The web UI gets an add-on slot of its own:** the front door is then not necessary. Until then, a contributor checks
  some parts of each new version of the web UI ([contributing](../contributing/web-ui.md#when-the-web-ui-changes)).
- **The web UI changes its chat input:** a contributor measures the box of the Board against it again, with one
  command ([contributing](../contributing/web-ui.md#when-the-web-ui-changes-its-chat-box)).

## Related pages

- [The web board](../reference/web-ui.md)
- [Set up the web board](../how-to/set-up-the-web-ui.md)
- [Why the chat box is built this way](web-chat-box.md)
- [The terminal board in the web UI's terminal view](../reference/web-ui-terminal-view.md)
- [Other machines](other-machines.md)
- [Testing the web board](../contributing/web-ui.md)
