# The web board's chat box

This page gives the facts about the chat on the web board: the box that you type in at the foot of the
[web board](web-ui.md), the switch over it, and the conversation panel.

The box looks and works as the chat input of the Herdr web UI does. It matches **herdr-web-ui 0.4.8**
([how it was measured](../contributing/web-ui.md#how-the-chat-box-was-measured)). For why it is built this way, refer
to [the explanation](../explanation/web-chat-box.md).

## Controls

| Control | What it does |
|---|---|
| `Enter` | Sends. This is also true on a phone. |
| `Shift+Enter` | Makes a new line. |
| `Ctrl+Enter` or `Cmd+Enter` | Also sends, if "Enter sends" is off in Settings → Chat of the web UI. |
| A keyboard that composes letters | Keeps its `Enter`. |
| `Tab`, or **Send + start** | Sends, and puts an agent on what the message adds. With nothing typed, `Tab` moves on, as `Tab` does on the terminal board. |
| The grip on the top edge of the card | Drag it up to make the box taller: up to half of the window, 480 px at most. Double-click it, or tap it two times, for the automatic height. |
| The grip, with the keyboard | Up and down move one line (two lines with `Shift`). `Home` gives the automatic height. |
| **+** | Attaches files. You can also paste a picture or drop files. |
| The mic | Dictates. |
| `Esc` | Goes one step back, as on the terminal board. The draft stays. |

The browser keeps the height of the box. Pasted words go into the box.

## What is the same as in the web UI

- **The card:** the raised surface of the web UI, a hairline border, its 24 px radius and its shadow. It is as wide as
  the chat column of the web UI, and centred. It is 16 px from the sides and the bottom (12 px from the sides at a
  width of 480 px and less). On a phone it is the glass card of the web UI.
- **The message** is the first row of the card, at full width. It has the type and size of the web UI (15 px with a
  mouse, 16 px on touch so that an iPhone does not zoom). It also has the text colour, hint colour and padding of the
  web UI. It follows the font size in Settings → Chat of the web UI.
- **It grows with the words.** One line is 44 px, and each line adds one line. From ten lines, its height stays the
  same and it scrolls, with the thin bar of the web UI and no arrows.
- **The grip** shows when a mouse points at the card. On touch it always shows.
- **The round Send** is in the lower right: the 32 px circle of the web UI in its primary colour, with an arrow up. On
  touch it is inside a 40 px target. It is faded while there is nothing to send, and while a message is on its way.
- **Focus** belongs to the card. There is never a rectangle on the field inside the card.
- **No browser look** on the field in Chrome, Firefox or Safari: no frame, no focus ring, no inner shadow, no resize
  corner, no scroll arrows.
- **Above the keyboard of a phone:** the Board is as tall as the shell of the web UI, which follows the keyboard.
- **Light, dark and every palette** of the web UI, because the box uses the variables of the web UI.

### Attaching files

- The round **+** in the lower left picks any files. The box also takes a picture that you paste into it, and files
  that you drop on it. The border of the card changes to the accent colour while files are over it.
- Each file is a tile above the message: a thumbnail for a picture, else its name. The tile says `Uploading`,
  `Attached` or `Failed`, and has a cross to remove the file.
- The path of the file goes into the message as `@<path> `, at the start, in the order that you picked the files. The
  agent reads the path in this form.
- Send waits while a file is on its way.
- The limits of the web UI apply: 8 MB for each file, and three files on their way at a time. A bigger file gives a
  message in a band over the box. The number of files at one time is the value of Settings → Chat → Files at once.
- The board keeps the file where the receiver of the words can read it:
  - for the Coordinator: in the folder of the Coordinator;
  - for the agent of a task: in the working folder of the task, under `.herdr-projects/files/` (git ignores it);
  - with no Coordinator: in the folder of the board (`files/` in its data folder).
- Files work on this machine only. The box refuses a file for the Coordinator or an agent of another machine, and
  tells you so.

### Dictation

The mic is beside the add button.

- Tap it to start, and tap again to finish. Or hold it and release it. Or hold `Ctrl+Shift+Space` (`Cmd+Shift+Space` on a Mac).
- While it listens, a pill over the box shows **Cancel**, the status (`Ready`, `Recording`, `Transcribing`, or
  `No microphone input`), the words that it heard, the level, the time, and **Done**. The time counts down in the last
  ten seconds. `Esc` cancels.
- The words go in at the caret, with a space next to a word on each side. The focus does not move.
- If the server of the web UI writes a tidied version later, that version replaces the words, unless you changed them
  in the meantime.
- The box refuses all of a dictation that is too long for the box. A dictation is two minutes at most.
- It uses the engines and settings of the web UI. It uses the server of the web UI if that server has an OpenAI key
  (Settings → Voice input). Else it uses the speech recognition of the browser.
- The mic shows as it does in the web UI: on auto only where it can work, and not on a phone. It uses the language of
  the web UI, and tidies the words or not as Settings → Chat says.
- A device that can only watch dictates nothing.

## What is the board's own

| The board's own | Where it is |
|---|---|
| Who gets your words: the Coordinator (and on which machine, if the board of another machine shows) or the agent of a task, with the task named | The switch over the box tells it ([The switch](#the-switch)), not the box. While the box is empty, its hint tells it too (`Send message to Coordinator`). |
| The lock that keeps a draft with its receiver | A filled chip in the lower left, beside the add button, only while the page shows somebody else: `To: HSW-34 agent · not who is shown above`. An attached file locks the draft, as typing does. |
| The colour of the box by receiver ([colours](colours.md)) | The border of the card: the plain text colour for the Coordinator, the status colour of the task for its agent. On focus, a ring of that same colour goes round it. |
| The two ways to send | The round button and `Enter` send. **Send + start** and `Tab` send and put an agent on what the message adds, if there are words and this applies. **Send + start** is a chip in the shape of the one-tap replies of the web UI. The tooltip of each tells what it does with the words. |
| Who you talk to, and the conversation | Over the box, as wide as the box: the switch, **Hide** and **Show**, and the conversation panel ([The conversation](#the-conversation)). |
| The notice after you send | Above the chat. |
| A draft is never lost | The board keeps it with its receiver, also over a reload. While a message is on its way, the words stay in the box and a second `Enter` does nothing. Words that the board could not take stay in the box. |
| A device that can only watch | The box is faded and tells you so. |

## What the box of the web UI has and the box of the Board does not

The box of the Board does not show these controls, because the Board cannot do what they do:

- **Stop.** In the web UI, the round button changes to Stop while the agent works. On the Board it stays Send. To stop
  the agent of a task, use the button on the open task.
- **The model, the effort and the context ring** between the controls.
- **The menus for commands and files** (`/` and `@`), **one-tap replies**, **the suggestion of the agent**, and **held
  messages**.
- **Escape that stops the agent.** In the web UI, `Esc` with a draft stops the agent. On the Board it goes one step
  back, and the draft stays.

## The switch

The switch is one button over the box. It names who gets your words. Two arrows show that a tap flips it.

| The switch says | When |
|---|---|
| `Coordinator` | The words go to the Coordinator of this machine. |
| `Coordinator on vm-a` | The board of one other machine shows. |
| `New task` | There is no Coordinator. |
| `#179 agent` | The words go to the agent of the selected task. |
| The coordinator of a machine or a repo | The selection is on the heading of that machine or repo. |

- A tap presses the key of the board, as `Space` does on the terminal board. It changes to the agent of the selected
  task, and the next tap changes it back.
- If there is nothing to flip to (no task is selected, or the row is an automation), the switch only names who gets
  your words. It has no arrows.
- The switch flips also while there are words in the box. The words stay locked to their receiver.

Beside the switch (under it on a phone, on up to two lines) is the line that tells who:

- the mark;
- the name of the task, for its agent (the switch already names the Coordinator);
- a grey line that tells where they are, with the progress of the check.

### The draft lock

- The words are locked to the receiver that the switch named when you typed the first word.
- If you then tap another task or the switch, you change what you look at, not who gets the words.
- While the page shows somebody else, the lower left of the box tells it, for example
  `To: HSW-1 agent · not who is shown above`. The frame of the box is then the plain colour.
- The lock includes the machine of the Coordinator. Words typed for the Coordinator of `vm-a` go to `vm-a`, also after
  the view moves to another machine.
- If `vm-a` is not reachable, the words go to nobody.
- The board keeps the draft with its receiver. A reload brings back both.

## The conversation

The conversation panel is over the box, and as wide as the box. It shows the newest message under a line that names
the speaker: `Coordinator:`, `Reviewer:`, `Agent:`, `You:` or `Board:`. It never shows the screen of the program.

- Markdown shows as the terminal shows it: bold is bold, and the marks of a headline and of a code block are not shown.
- The Coordinator of another machine shows too. That machine sends its words with its tasks
  ([other machines](../explanation/other-machines.md)).
- **Full screen** is the arrows in the top-right corner of the panel. It gives the conversation all of the board over
  the box, with all of it open. On a phone the panel is only a few lines high.
- To go back to the board, use the same button, `Esc`, or the back arrow at the top. In full screen, that arrow is the
  arrow of the Board. This is also true on a phone with a task open, where the arrow is usually the arrow of the task.

### Earlier and later

These do what `Alt+K`, `Alt+J` and the wheel do on the terminal board.

- Scroll or swipe in the panel, or use **Earlier** and **Later**. These are the arrows at the right edge of the panel.
  Each shows only if there is more in its direction.
- The panel opens on the newest message from any speaker, and holds only that message. Your own words show only until
  the answer comes, as on the terminal board.
- **Earlier**, the wheel up, or a swipe down at the top of the panel opens the rest of the conversation. The buttons
  move half a panel at a time.
- **Later** at the end closes the panel to the newest message again.
- A new message shows from its start.
- An opened conversation stays open while you talk to the same receiver. It stays on what you read, or on its end if
  you were at the end.
- **Later** at the end, **Hide**, or a talk with another receiver closes it.
- A scroll down past the end, with the wheel or a swipe, does not close it. Only **Later** does.
- When the panel has the focus, page up and page down do what the buttons do.

### Hide and Show

**Hide** and **Show** are beside the switch. They do what `Alt+C` does on the terminal board. They use the key of the
board, so the board remembers the choice, also over a reload. When the message is hidden, the switch, the line that
tells where they are, and the box stay.

## Related pages

- [The web board](web-ui.md)
- [Why the chat box is built this way](../explanation/web-chat-box.md)
- [How the web board works](../explanation/web-ui.md)
- [How the chat box was measured](../contributing/web-ui.md#how-the-chat-box-was-measured)
- [Colours](colours.md)
- [Talk to a Coordinator](../how-to/talk-to-a-coordinator.md)
