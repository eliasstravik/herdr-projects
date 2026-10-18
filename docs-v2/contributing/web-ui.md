# Testing the web board

This page is for contributors who change the web board or its chat box. It tells where the code is, how to try it
against a copy of the web UI, what to check on a new version of the web UI, and what was checked and measured.

## The code

- On the server, the web board is the terminal board opened with no terminal: `crates/web/src/webui/view.rs`.
- The board as data for the page is in `crates/web/src/webui/model/`.
- The button of a key has a name of its own in the words of the board: the `web` field in
  `crates/flow/src/board/keys.rs`, for example "“Undo” brings it back".
- The chat box is `crates/web/src/webui/page/compose.js` and `compose.css`. `compose.css` names the version of the web
  UI that it was measured against.

For why the web board uses the code of the terminal board, refer to
[how the web board works](../explanation/web-ui.md#the-same-code-decides).

## Trying it against a copy

`HERDR_PROJECTS_WEBUI=<port>` (or `host:port`) names a web UI that Herdr does not list as a plugin. Use it to try out a
copy of the web UI, with a scratch Herdr and a scratch [home folder](../reference/home-folder.md). The value `off`
means no web UI.

## When the web UI changes

If the web UI gets an add-on slot of its own, the front door is not necessary. Until then, check these on each new
version of the web UI:

- the foot of the sidebar (`.sidebar-footer`);
- the classes of the **Agents** section that the **Boards** section is built of (`AgentSidebar.tsx` and `.css` in the
  web UI): `agent-section-toggle`, `agent-section-caret`, `agent-list-contents`, `agent-item`, `agent-row`,
  `agent-copy`, `agent-title`, `agent-context`;
- the scrim of the drawer (`.scrim.is-open`);
- its address parameters `ws`, `pane` and `view` (`src/lib/deepLink.ts`), and its `machine` address parameter;
- that `/api/health?scope=bridge` tells who asks, and that `/herdr-projects/` is still free;
- what `/api/access` says about its link (`server/tailscale.ts`);
- that `/api/machines` names the `id`, `kind`, `target` and `state` of each PC.

If the web UI changes its chat input, measure the box again ([below](#when-the-web-ui-changes-its-chat-box)).

## What was checked

On herdr-web-ui 0.4.8 with Herdr 0.9.3: a real web UI behind the front door, in a browser at the size of a phone and of
a desktop, in dark and light.

- Its chat and terminal, files (also ranged), its service worker and manifest, and its push and update calls answer
  the same bytes through the front door as directly.
- The board lists, opens, sends, asks and acts.
- The front door follows a restart of the web UI.
- The link step and its way back change only the private link of the web UI. Its Settings → Phone, at the link, still
  names that link.

The self-update of the web UI was not run for real. The front door changes none of its files and keeps none of its
state, and an update ends in a restart.

### Other machines and the terminal view

Checked on 2026-10-13, herdr-web-ui 0.4.8 with Herdr 0.9.3. Two separate Herdrs on one computer were the two machines.
Each had its own settings, state and board, with both plugins installed. The second was reachable only over SSH, saved
in the first with `herdr machine add`, and added in the first web UI as a PC. A real browser went through the front
door at the size of a phone (390×800) and of a desktop (1280×800).

| What | Result |
|---|---|
| The web board: every machine, one machine, back | The tasks of the other machine show with its name. The chip tells which. |
| The web board on the task of another machine: pin, archive, undo, talk to its agent, approve and merge | That machine did each one (the merge at the size of a phone). The answer shows on the board. |
| What you type while one machine shows | It went to the Coordinator of that machine, which added the task there. |
| **Open this task's agent**, on the task of another machine | It went to the chat of that agent in the web UI, on that PC. |
| The Board opened while the web UI shows a pane of the other PC | The board of that machine, without help. Back on this machine, what it showed before. |
| The terminal board in the terminal view, before the change | The view of the web UI and the board took the terminal from each other every four seconds, with a Herdr window on the session and with none. No `Alt` key did anything. |
| The terminal board there now, of this machine and of the other PC, desktop | It stays. Every `Alt` and `Ctrl` key, `?`, `Shift+Enter`, drafts, find and the arrows did what they do in a terminal. |
| The same on the phone layout | `?` from the input line opens the menu, and the menu stays open. The arrows of the key bar move in it. `Alt` then `m` with direct typing shows every machine. A message from the input line went to the Coordinator. |
| From the terminal board there: an agent put on the task of the other machine with `Tab`, then into it with `Enter` | The agent started there. Going in showed its screen, and `ctrl+b q` did not come back: the view of the web UI disconnected. For this reason, the board does not go in from there now. |
| The mouse on the terminal board there (desktop) | The wheel over the list moves the selection one task at a time. The board highlights a drag over text. The selected row has its colour. |
| The board as the popup of Herdr (`board.open popup`), in the terminal view | The popup is a pane that the web UI lists. It stays, and `Alt+M`, `?` and `Shift+Enter` work in it as in the tab. |
| The other machine on the plugin as it was before this change | **Open this task's agent** still goes to the chat of that agent. Its answer names no space, and the id of a pane starts with the id of its space. |
| The address of the web UI with a PC and no pane (`?machine=<pc>#board`) | The web UI goes to a pane of this machine without help, so the Board is the board of this machine. It follows a pane, not a PC alone. |
| What a program gets from the terminal view | Desktop: old bytes (`esc m` for `Alt+M`, `esc return` for `Shift+Enter`), also after it asked for the newer keyboard protocol. `ctrl+b` is held and sent with the next key. Phone: the input line sends the text, then a return alone 120 ms later, and ignores `Ctrl` and `Alt` on the key bar. The key bar and direct typing send the sequences of the newer protocol. A click and the wheel come as mouse reports. |

Not checked:

- the reserved keys of a browser (the test browser has none: `Ctrl+N` and similar are the known keys of Chrome);
- the Option key of a Mac;
- the wheel over the chat, and the mouse on a phone;
- a plugin so old that it sends no version (only a test covers its message);
- a machine with no web UI or front door of its own;
- a web UI newer than 0.4.8 that takes its PCs from the list of Herdr;
- real separate computers: all of the above was two Herdrs on one computer.

### The Boards section

Checked on 2026-10-14 on the newest code of herdr-web-ui at that time (0.4.8 and later, `38d6863`) with Herdr 0.9.3.
The setup was a scratch Herdr with two agents, the web UI in front of it, and the front door in front of that. The
board had tasks on this machine, one other machine that answered with its tasks, and one that refused the connection
(in place of SSH). A real browser was at the size of a desktop (1280×800) and of a phone (390×800).

| What | Result |
|---|---|
| The sidebar | **Boards** under **Agents**, above **Install app** and **Settings**, in the same type, spacing and caret as **Agents**. This machine, the two others and **All machines**, each with its dot and line. |
| The machine that refused | Dim, `not reachable`, a ring for its dot. **All machines** says `· 1 not reachable`. |
| Pick the entry of the other machine | Its board opened (`Only vm-str`, its Coordinator in the box), and the entry was highlighted. |
| On the phone | Folded at the start as **Agents** is, with its dot beside the heading. Unfolded, the same entries. A pick closed the drawer and opened that board over the page. |

Not checked: a web UI that serves real other PCs (a test checked the names of the PCs), and the section through a
restart and an update of the web UI (the script and its place are the same as for the old Board entry).

## How the chat box was measured

A script, `scripts/chat-box-match.js`, opens the two boxes side by side in a real browser. One is the web UI on the
chat of an agent. The other is the Board through its front door, in front of that same web UI.

- It uses the size of a phone (390×800) and of a desktop (1280×800), in dark and light.
- It reads each box empty, with one line, with fourteen lines, focused, and focused and empty.
- It reads the sizes, places and styles of each box, and compares the pixels of the message field and of the round
  Send.
- It exits with 1 if the two are different in a part that is not the board's own.

```
node chat-box-match.js --web-ui 'http://127.0.0.1:7317/?ws=…&pane=…&view=chat' \
    --board 'http://127.0.0.1:7327/#board' --playwright <the web UI's checkout> [--browser firefox] [--sending]
```

With `--sending`, it also compares the two **while a message is on its way**. It presses enter in each, and holds the
message for four seconds on its way out (the web UI on its socket, the board on its own call). It reads both boxes in
that moment, and fails if it did not catch one of them in it.

**CAUTION:** `--sending` sends real messages, four times: one line to the agent of the pane of that web UI and one to
the board. Run it only against a scratch Herdr and a scratch board.

### Where the script is

The script is a Node program with Playwright. Thus it left the repo when the board became one Rust program. It is in
the history of the repo: `git log --all -- scripts/chat-box-match.js` finds it. The page that it measures
(`compose.js`, `compose.css`) did not change.

The script was never part of `npm run check`. It needs a running web UI with the pane of an agent, the front door and a
browser ([trying it against a copy](#trying-it-against-a-copy)).

### Results

Measured on 2026-10-13 against herdr-web-ui 0.4.8 with Herdr 0.9.3, in Chromium 156 and Firefox 155:

| What | Result |
|---|---|
| The size, place, radius, shadow, background and border width of the card, in all 24 states (six at each of two sizes in two themes), both browsers | The same. |
| The box, type, size, line height, padding, colours, and smallest and largest height of the field | The same. |
| The box, circle, glyph, colours and faded look of the round Send | The same. No pixel is different. |
| The pixels of the field, with the same words | No pixel is different, except approximately 350 at the two rounded top corners of the card. There the border goes through the box of the field. That is the colour of the border, which is the board's own. |
| The pixels of the field while empty | The words of the hint are different (`Message Claude…` in the web UI, the hint of the board here). Nothing else is different. |
| While a message is on its way, both caught in that moment, at both sizes in both themes | The same in both. The words stay in the box. The round Send is faded (0.7) and you cannot push it. The card keeps its focus. The pixels of the field and of the button match as above. When the answer comes, both boxes become empty. |
| On focus | The border of the web UI changes to its accent. The border of the board keeps its receiver colour, with a ring of it. |

Tried for real, through the front door, at both sizes: typing, `Shift+Enter`, sending with `Enter` and with
**Send + start**, the box while a message is on its way, a long draft, a reload with a draft, switching between the
Coordinator and the agent of a task, a draft locked to a task while another task shows and over a reload, the grip
(drag, reload, double-click), and pasting words and a picture. Also in Firefox, at the size of a desktop.

Not checked:

- Safari itself. Its engine could not run on the machine of the measurement. The rules for it are written
  (`-webkit-appearance: none` and the rest), but nobody saw them work.
- The keyboard of a real phone.

## When the web UI changes its chat box

1. Get the script from the history of the repo ([above](#where-the-script-is)).
2. Run the script again against the new version of the web UI.
3. Change `compose.css` as the output of the script tells you.
4. Write the new version in `compose.css` and on this page.

Already known: the code of the web UI after 0.4.8 changes its box. This is its `main` on 2026-10-13, commit `a4b2295`,
which is not released. The script finds 82 differences:

- The message has 4 px less padding on each side (16 px, 12 px on a phone).
- The round Send is 4 px further in with a mouse.
- The model moves to the left, beside the add button.
- A strip with the folder and the branch is under the card, and lifts the card 29 px.

The Board matches the released 0.4.8 until that code is released.

## Related pages

- [How the web board works](../explanation/web-ui.md)
- [Why the chat box is built this way](../explanation/web-chat-box.md)
- [The web board](../reference/web-ui.md)
- [The web board's chat box](../reference/web-chat-box.md)
- [Run the tests](run-the-tests.md)
- [Testing](testing.md)
- [Crates](crates.md)
