# The web board's chat

**The chat under the [web board](web-ui.md)'s list is the Herdr web UI's own chat**: the same transcript, box, files,
dictation, Stop and keyboard handling as when you open that agent from the web UI's sidebar. The board docks it under
its list and puts one bar over it: who you talk to, how they stand, Hide and Full screen. The board's **Call** button
stands in the box, beside the mic.

On a phone, and in a narrow window:

```
┌──────────────────────────────────┐
│ ←  Board                   🔍  ⋯ │
│ ▾ needs you 1                    │
│   ! HP-5  Write a haiku …        │
│ ▾ to do 4                        │
│   ○ HP-4  Fix flaky login test   │
│╭───────────── ═══ ──────────────╮│  the chat's sheet, its grip
│ [● Coordinator│○ HP-4 agent]  ⌄ ⤢│  the bar
│                                  │
│   the web UI's own transcript    │
│ ┌──────────────────────────────┐ │
│ │ the web UI's own box         │ │
│ │ + mic ☎                    ↑ │ │  add, mic, Call · Send
│ └──────────────────────────────┘ │
└──────────────────────────────────┘
```

In a wide window the list and the chat stand side by side, the bar at the top of the chat.

**The bar and the chat are one piece.** Under the list they are one sheet with rounded top corners, on the ground the
web UI's chat has, with no line between the bar and the box: the switch starts where the box starts, and the last
button stands over the box's Send. With the chat hidden that is the bar and the box alone, one dock at the foot of the
screen. With the chat shown, its transcript scrolls up under the bar through a short fade, as the web UI's own chat
scrolls under its header. Full screen, while typing on a phone and side by side there is no list above, so no sheet: the
bar heads the chat. On a narrow phone the buttons keep their row, and the switch's names are cut short first.

## The bar

- **Who you talk to**: the web UI's own switch with the two you can talk to, the Coordinator first and the selected
  task's agent second (`HP-4 agent`; on a machine's or a repo's heading, its coordinator). The one you talk to is
  pressed; a tap on the other presses the board's own key, as `space` does on the terminal board. With nobody else to
  talk to (no task selected, an automation's row) it is only their name.
- **How they stand**: each choice has its mark in the board's colours (the dot breathes while it works). Beside it the
  board's words for it (`No agent yet: your message starts one`, a check's progress). On a phone, a word the mark
  already says (`idle`, `working`) steps aside, and a longer one takes a line of its own under the switch.
- **The call's row**: while a [voice call](voice.md) rings or is on, its row stands under the bar, with Mute and End.
- **Hide** (the chevron) keeps only the box: the list gets the screen. **Show** brings the conversation back. It is the
  board's `alt+c`, so the board remembers which, also over a reload.
- **Full screen** (the arrows): the board's header and list step aside and the chat has the whole screen under the bar.
  The same button, in the same place, then shows the arrows going in and goes back; so do Esc and a phone's back
  gesture (or the browser's Back), as full screen is a step in the browser's history.
- **The grip** at the top of the sheet (on a phone, and on a desktop when pointed at) moves where the list ends and the
  chat begins: from a quarter to four fifths of the screen, kept in this browser. The arrow keys move it a step, Home
  puts it back.

## Docked on the web UI's chat

The board asks where the chat of whoever it talks to is, and the web UI under the board goes there in place: its
address names that pane's chat view (`?ws=…&pane=…&view=chat#board`), so Back still closes the board.

- **It only looks.** The Coordinator's pane, a machine's or a repo's coordinator's, or the task's agent's pane while an
  agent is in it now. Nothing is opened again or started for it: a task whose agent is not in its pane is the board's
  own chat until you ask for more (**Open this task's agent** opens its space again).
- **The web UI makes room.** Its pane column leaves the board the room the board covers: at its top on a phone and a
  narrow window, at its left beside the list on a wide one. Its own header and tabs over the chat step aside (the
  board's bar stands for them), and the board goes under the web UI's own menus, sheets and drawer.
- **Another machine.** A task's agent on another machine is docked from that machine's PC in the web UI, while it is
  connected there; the board stays the board of the PC the page was on.
- **Closing the board** puts the web UI back on the pane it showed before.
- **Picking another pane** in the web UI's own sidebar undocks it.
- Where the web UI cannot show them, or does not show them within four seconds (a Coordinator not started yet, a task
  with no agent in its pane, another machine's Coordinator, a web UI this was not made for), **the board's own chat
  stands in**, in the same place and under the same bar: the conversation as the web UI draws its own (your words a
  card on the right, theirs plain prose) over [the board's own box](web-chat-box.md).

## What you type

- **Call** (the phone) stands in the box, right after the mic (after **+** where the mic is off, as on a phone), in the
  web UI's box and in the board's own alike: the board puts it in the web UI's box while docked, and takes it out again
  when the board closes. It starts a [voice call](voice.md) with whoever the chat talks to; while the call rings or is
  on, the same button is red and hangs up. The mic writes your words into the box; Call talks. Each says what it does
  when hovered and to a screen reader (`Dictate (hold)`, `Start a voice call`).

- **To a task's agent**: the web UI's box sends it, as it does in the web UI, with its queue, its `/` menus and its Stop.
- **To a coordinator** (the Coordinator, a machine's or a repo's): the web UI's box, but the board sends the words, as
  its own box does. So the Coordinator knows where you were (the repo and the task you had open,
  [the Coordinator](coordinator.md)), and words wait while it is busy.
  - Enter (or what the web UI's Settings → Chat says sends) and the round Send. While the web UI's own menu is open (its
    `/` commands, its `@` files), Enter and Tab pick from it, as they do there.
  - The words leave the box once the board took them, and the files they carried with them; words it did not take
    stay, with their files. While a file is still uploading nothing goes, as the web UI's own Send waits.
  - There is no second way to send: the coordinator decides what to start. Tab is the web UI's own there.
- Escape in the web UI's box and its menus is the web UI's own.

## Above a phone's keyboard

While you type in the web UI's box with a phone's keyboard up, **the board steps aside to its bar**: the header and the
list go, and the web UI's chat has the room the keyboard leaves, as it handles its own box above the keyboard. When
the keyboard goes down, the list is back. The keyboard is known by the web UI's own mark on the page
(`data-keyboard`). The board's own box, where it stands in, keeps its own way: [above a phone's
keyboard](web-chat-box.md#above-a-phones-keyboard).

## How it was tried

On a copy of Herdr, the web UI (herdr-web-ui 0.4.12) and the board, with a real Coordinator and a real task's agent
(Claude Code), in Chromium and WebKit at an iPhone's size (440×956) and a desktop's (1440×900): the Coordinator and a
task's agent docked, a task with no agent (the board's own chat), working, a long answer, a file attached in the web
UI's box, Hide, Full screen, the More menu over the docked chat, a call ringing, on, muted, ended and failed (with the
call's service played by the page), Enter and Send + start through the board (one message each, the box emptied),
closing the board and Back (the web UI back on its pane). The keyboard was played: the web UI's mark set and its
height made shorter. Not tried on a real phone.

**The bar and the chat as one sheet** were checked on 2026-10-20 against a real web UI (herdr-web-ui 0.4.12) through
the front door, in Chromium and WebKit at a phone's sizes (440×956, 393×852, 375×760) and a desktop's (900×900,
1440×900), dark and light: the chat hidden and shown, docked and the board's own, full screen and side by side, before
and after. The switch's left edge and the box's line up, and the last button stands over Send, at every size. Not tried
on a real phone.

**The Call button in the box** was checked on 2026-10-21 in Chromium at a phone's size (390×844, touch), a tablet's
(820×1180, touch) and a desktop's (1440×900), with the board's real chat over a copy of the web UI's box (herdr-web-ui
0.4.12's markup and styles): in the board's own box and in the web UI's, with and without the mic, no call and a call on;
the web UI drawing its box again, undocking and docking again, and closing the board (nothing left in the web UI's box).
Not tried against a running web UI or on a real phone.
