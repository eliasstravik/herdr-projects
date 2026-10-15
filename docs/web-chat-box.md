# The web board's chat box

**The box you type in at the foot of the [web board](web-ui.md) looks and works like the Herdr web UI's own chat
input**, so going from the Board to an agent's chat and back feels like one program.

It is not the web UI's code. The web UI's input is part of its bundled app, whose built files change with every
release, so the Board loads nothing from them. It is the same thing built again in the Board's own page
(`crates/web/src/webui/page/compose.js`, `compose.css`), from the web UI's CSS variables and its `DESIGN.md`, and then
measured against the original.

## Using it

- **Enter sends, shift+enter is a new line**, on a phone too. If the web UI's Settings → Chat has "Enter sends" turned
  off, ctrl or cmd with enter sends here as well. A keyboard that composes letters keeps its enter.
- **Tab, or Send + start**, sends and puts an agent on what it adds (more below).
- **The grip** on the card's top edge makes the box taller: drag it up (up to half the window, 480 px at most).
  Double-click or tap it twice for the automatic height. With the keyboard: up and down a line (two with shift), and
  Home. The height is kept in this browser.
- **+** attaches files. Pasting a picture or dropping files works too.
- **The mic** dictates.
- **Escape** goes one step back, as on the terminal board. The draft stays.

## What is the same as the web UI

Measured against **herdr-web-ui 0.4.8** (see [how](#how-it-was-measured)):

- **The card**: the web UI's raised surface, a hairline border, its 24 px radius and its shadow. As wide as its chat
  column and centred; 16 px from the sides and the bottom (12 px from the sides at 480 px and under). On a phone it is
  the web UI's glass card.
- **The message** is the card's first row, at full width: the web UI's type and size (15 px with a mouse, 16 px on touch
  so an iPhone does not zoom), its text and hint colours, and its padding. It follows the web UI's own Settings → Chat
  font size.
- **It grows with the words.** One line is 44 px; each line adds one. From ten lines on it stays that tall and scrolls,
  with the web UI's thin bar and no arrows.
- **The grip** shows when the card is pointed at with a mouse, and always on touch.
- **The round Send** in the lower right: the web UI's 32 px circle in its primary colour with an arrow up (inside a
  40 px target on touch). Faded while there is nothing to send and while a message is on its way.
- **Attaching files**, as in the web UI:
  - The round **+** in the lower left picks any files. A picture pasted into the box, or files dropped on it, are taken
    too (the card's border turns to the accent while files are over it).
  - Each file stands as a tile above the message (a picture's thumbnail, else its name), saying Uploading, Attached or
    Failed, with a cross to take it out.
  - Its path goes into the message as `@<path> `, at its start, in the order the files were picked, which is how the
    agent reads it. Send waits while one is on its way.
  - The web UI's limits: 8 MB a file (a bigger one says so in a band over the box), three on their way at a time, and as
    many at once as its Settings → Chat → Files at once says.
  - The file is kept where whoever gets the words can read it: in the Coordinator's own folder, in a task's working
    folder under `.herdr-projects/files/` (git is told to leave it alone), or, with no Coordinator, in the board's own
    folder (`files/` in its data folder).
  - Only on this machine: a file for another machine's Coordinator or agent is refused, and the box says so.
- **Dictation**, as in the web UI, with the mic beside the add button:
  - Tap it to start and tap again to finish, or hold it and let go (or hold ctrl+shift+space, cmd on a Mac).
  - While it listens, a pill over the box shows Cancel, where it stands (Ready, Recording, Transcribing, or No
    microphone input), the words heard so far, the level, the time (the last ten seconds counted down) and Done. Escape
    cancels.
  - The words go in at the caret, with a space against a word on either side, without taking the focus. Where the web
    UI's server writes a tidied version afterwards, that replaces them unless you changed them meanwhile.
  - A dictation too long for the box is refused whole. At most two minutes.
  - It uses the web UI's own engines and settings: its server when it has an OpenAI key (Settings → Voice input there),
    else the browser's own speech recognition. The mic shows as the web UI's does (on auto only where it can work, and
    not on a phone), in its language, tidied or not as its Settings → Chat says.
  - A device that may only watch dictates nothing.
- **Focus is the card's**, never a rectangle on the field inside it.
- **No browser look** on the field in Chrome, Firefox or Safari: no frame, no focus ring, no inner shadow, no resize
  corner, no scroll arrows.
- **Above a phone's keyboard**: the Board is as tall as the web UI's own shell, which follows the keyboard.
- **Light, dark and every palette** of the web UI: they are its variables.

## What is the board's own, in that shape

| The board's | Where it is in the box |
|---|---|
| who gets your words: the Coordinator (and on which machine, where another one's shows) or a task's agent, with the task named | said by the switch over the box ([the web UI](web-ui.md)), not in it. The box's own hint says it too while empty (`Send message to Coordinator`) |
| the lock that keeps a draft with who it was typed for | a filled chip in the lower left, beside the add button, only while the page shows somebody else: `To: HSW-34 agent · not who is shown above`. Attaching a file locks it as typing does |
| the box's colour by who you type to ([colours](colours.md)) | the card's border: the plain text colour for the Coordinator, the task's status colour for its agent. The web UI's border is grey and turns to its accent on focus; here the border already says who, so focus draws a ring of that same colour round it |
| the two ways to send | the round button and enter send. **Send + start** and tab, while there are words and it applies, send and put an agent on what it adds. **Send + start** is a chip of the web UI's (the shape of its one-tap replies). Each says in its tooltip what it does with the words |
| the switch between the Coordinator and the task's agent, and the conversation | over the box, as wide as it: the web UI's segmented switch, Hide and Show, and the conversation in a panel of its own that scrolls back ([the web UI](web-ui.md)) |
| the notice after sending | above the chat, as before |
| a draft is never lost | it is kept with who it is for, also over a reload. While a message is on its way the words stay in the box and a second enter does nothing. Words the board could not take stay in the box |
| a device that may only watch | the box is faded and says so |

## What the web UI's box has and the Board's has not

Left out, because the Board cannot do it, rather than shown as a dead control:

- **Stop.** The web UI's round button turns into Stop while the agent works. The Board's stays Send; stopping a task's
  agent is a button on the open task.
- **The model, the effort and the context ring** between the controls: the Board's box talks to different agents.
- **The menus for commands and files** (`/` and `@`), **one-tap replies**, **the agent's suggestion**, and **held
  messages**: the Board hands your words on as they are.
- **Escape stopping the agent.** In the web UI, Escape with a draft stops the agent. On the Board it goes one step back,
  as on the terminal board, and the draft stays.

## How it was measured

A script, `scripts/chat-box-match.js`, opens the two side by side in a real browser: the web UI on an agent's chat, and
the Board through its front door in front of that same web UI. At a phone's (390×800) and a desktop's (1280×800) size,
in dark and light, with the box empty, with one line, with fourteen lines, focused, and focused and empty, it reads each
one's sizes, places and styles, and compares the pixels of the message field and of the round Send. It exits with 1 when
the two differ in anything but what is the board's own.

    node chat-box-match.js --web-ui 'http://127.0.0.1:7317/?ws=…&pane=…&view=chat' \
        --board 'http://127.0.0.1:7327/#board' --playwright <the web UI's checkout> [--browser firefox] [--sending]

With `--sending` it also compares the two **while a message is on its way**. It presses enter in each and holds the
message back for four seconds on its way out (the web UI's on its socket, the board's on its own call), reads both boxes
in that moment, and fails if either was not caught in it. That sends for real, four times: one line to the agent of that
web UI's pane and one to the board. So run it against a scratch Herdr and a scratch board.

Where the script is: it is a Node program with Playwright, so it left the repo when the board became one Rust program.
It is in the repo's history (`git log --all -- scripts/chat-box-match.js` finds it); the page it measures (`compose.js`,
`compose.css`) is unchanged. It was never part of `npm run check`: it needs a running web UI with an agent's pane, the
front door and a browser ([trying it against a copy](web-ui.md#trying-it-against-a-copy)). `compose.css` names the
version it was measured against.

Measured on 2026-10-13 against herdr-web-ui 0.4.8 with Herdr 0.9.3, in Chromium 156 and Firefox 155:

| What | Result |
|---|---|
| the card's size, place, radius, shadow, background and border width, in all 24 states (six at each of two sizes in two themes), both browsers | the same |
| the field's box, type, size, line height, padding, colours, least and greatest height | the same |
| the round Send's box, circle, glyph, colours and faded look | the same; no pixel differs |
| the field's pixels, holding the same words | none differ, but for some 350 at the card's two rounded top corners, where the border runs through the field's box: that is the border's colour, which is the board's own |
| the field's pixels while empty | the hint's words differ (`Message Claude…` there; the board's own hint here), nothing else |
| while a message is on its way, both caught in that moment, at both sizes in both themes | the same in both: the words stay in the box, the round Send is faded (0.7) and cannot be pressed, the card keeps its focus; the field's and the button's pixels match as above. When the answer comes both boxes empty |
| on focus | the web UI's border turns to its accent; the board's keeps who it is for, with a ring of it |

Tried for real, through the front door, at both sizes: typing, shift+enter, sending with enter and with **Send + start**,
the box while a message is on its way, a long draft, a reload with a draft, switching between the Coordinator and a
task's agent, a draft locked to a task while another one shows and over a reload, the grip (drag, reload, double-click),
pasting words and pasting a picture. In Firefox too, at the desktop's size.

Not checked: Safari itself (its engine could not run on the machine this was measured on: the rules for it are written,
`-webkit-appearance: none` and the rest, but not seen), and a real phone's keyboard.

## When the web UI changes its box

Run the script again against the new version (from the repo's history, above). What it prints is what to change in
`compose.css`. Then name the new version there and here.

Already known: the web UI's code after 0.4.8 (its `main` on 2026-10-13, commit `a4b2295`, not released) changes its box,
and the script says so (82 differences). The message has 4 px less padding at each side (16 px, 12 px on a phone), the
round Send stands 4 px further in with a mouse, the model moves to the left beside the add button, and a strip with the
folder and the branch hangs under the card, which lifts the card 29 px. The Board matches the released 0.4.8 until that
is released.
