# The chat of the board

This page gives the facts about the chat at the bottom of the board: who it talks to, its parts, how to hide the
message, and who gets the words that you type. For what the message box shows, see
[the messages in the chat](board-chat-messages.md).

## Who the chat talks to

There is one chat, at the bottom of the board. It is the same chat for everyone you talk to. It always talks to
someone, and it always shows who.

- **The Coordinator**, until you press `Space` on an empty text box. `Space` changes it to the agent of the selected
  task. `Space` again changes it back. Ask the Coordinator a question, tell it to do something, or tell it what you
  need.
- **The arrows only move the selection.** They never change who you talk to.
- While you talk to the agent of a task, the chat follows the selection. If you select a different task, you talk to
  the agent of that task, and the chat shows its latest message.
- **A task with no agent to talk to** still shows its name, and where your message goes:
  - for a to-do task, the message starts an agent on it;
  - for other tasks, the message goes to the Coordinator, about that task.
- An archived task is read-only.
- **With no task selected** (words on their way to another machine, or an empty list), the chat names the Coordinator
  until a task is selected again.
- **If the task you talk to goes away** (it is removed), you are back with the Coordinator, never with the agent of a
  task next to it.
- **On the heading of a machine or of a repo**, `Space` talks to the own coordinator of that machine or of that repo
  ([machine and project coordinators](../explanation/machine-and-project-coordinators.md)).

## The parts of the chat

The chat looks the same for the Coordinator and for the agent of a task, at every width of the window.

```
 ○ Coordinator  idle
╭───────────────────────────────────────────────────────────────────────────── alt+k more ↑ ─╮
│ Coordinator:                                                                               │
│ Added proj/4: Add an undo. An agent is on it.                                              │
│                                                                                            │
│ ❯ Send message to Coordinator                                                              │
╰────────────────────────────────────────────────────────────────────────────────────────────╯
  alt+c hide the latest message · ? all keys
```

### 1. The line that shows who you talk to

- The line has three parts:
  - the mark of the board for their state (the same mark as on the row of the task);
  - the name, in bold;
  - one greyed text: what the row of the task says (the "doing now" line of the Watcher), or its state.
- For the Coordinator, the greyed text is `idle`, `working`, `needs you` or `new reply`. It also shows how many of your
  messages still wait to be sent.
- The board cuts the line with `…` where it does not fit.
- The board never cuts the name to make room. A name longer than the window uses a second line. In a window too short
  for that, the draft keeps the line.
- The line names no program, model, effort or keys.
- One thing gets a greyed line of its own under it: a default that changed while a Coordinator runs, with the command
  that starts a new Coordinator with the new default.

### 2. The rounded box

- The box shows, in this order:
  1. a line that names who said the newest thing: `Coordinator:`, `Agent:`, `Reviewer:` or `You:`;
  2. the newest thing said in that conversation, in grey;
  3. an empty line;
  4. the line that you type on. The prompt is only `❯`.
- While the line that you type on is empty, it shows in grey who gets your message, by the name of their tab:
  `Send message to Coordinator` or `Send message to Agent`. It shows nothing else.
- The frame of the box and its `❯` also show it by colour: the plain text colour for the Coordinator, and the status
  colour of the task for its agent ([colours](colours.md)).
- **The box grows with a long message**, up to two thirds of the window, and becomes small again for a short one. The
  list above it gives up those rows and keeps the selected task in view.
- Under the whole task (`Alt+T`) and the `?` menu, the message uses at most one third of the window. These pages are
  there for you to read.
- `Alt+C` hides the message and shows it again.
- If there is nothing else to step back from (a draft, find, the whole task, the filter, the archive), `Esc` also hides
  the message. It does this before it does anything to close the board.
- On the web board, `Esc` does the same, while nothing is typed and find is closed.

### 3. The hint line

- One greyed hint line is under the box. It shows only the keys that do something now.
- While you type, it shows what the keys do with your words:
  `enter send to the Coordinator · tab send, and put an agent on what it adds · shift+enter new line · esc put away`.
- What each hint says, and what gives way in a narrow window: [the hint line](keys-hint-line.md).

## Hiding the message

```
 ○ Coordinator  idle
──────────────────────────────────────────────────────────────────────────────────────────────
❯ Send message to Coordinator
──────────────────────────────────────────────────────────────────────────────────────────────
  alt+c show the latest message · ? all keys
```

- **One key shows and hides the message: `Alt+C`**, on an empty text box (as `Alt+P` for the side panel). It works the
  same in both directions, for the Coordinator and for the agent of a task.
- The hint line names the key. The board remembers your choice.
- When the message is hidden, only two lines show: the line that shows who you talk to, and the line that you type on,
  between two thin rules. Thus you never send a message without the name of who gets it.
- While the message is hidden, no other key shows it. `Esc` never shows it.
- If a Coordinator could not start, the chat shows why, also while the message is hidden. `Alt+C` also puts that error
  away, until the next error.
- If there is no message to show (a task with no agent on it, the board with no Coordinator), the chat shows the plain
  line between two rules, hidden or not.
- The whole chat of the agent of a task is its own pane. `Enter`, with nothing typed, goes into it.
- The window never pushes off the text box and its hints. The message gives way first.
- In a very narrow window, the box stays whole. The message wraps inside it. If the placeholder does not fit, the board
  cuts it with `…`.

### When the chat folds by itself

The chat folds as `Alt+C` folds it (its head line and the plain text box) in these cases:

- on a page that is there to be read: the whole task, the settings page, the `?` menu;
- in a window of fewer than 14 rows.

The page, or the list, gets those lines. When you go back, the chat is back.

- The whole task shows what its agent said.
- A start that failed still shows why.

## Typed words keep who they are for

While the box holds text, the hint line under it shows who gets the text. Examples: `enter send to HSW-34's agent`,
`enter send to the Coordinator`, `enter add it as a task`.

- The hint names the task by its tag, also while the list is scrolled to other tasks.
- The edge of the box names nobody.
- Who gets the text does not change until you send the words or put them away.
- While the box holds text, the arrows move in the draft, the mouse wheel moves only the view of the list, and a click
  selects nothing.

**While the text box holds text, it has the keyboard.** Only `Enter`, `Tab`, `Esc` and `Ctrl+C` act on the board.

- `Esc` puts a draft away.
- `Ctrl+Z` brings the draft back, but only to whoever it was for.
- A draft never changes who you talk to. If the chat names somebody else, the chat says who the words were for:
  - for the Coordinator: `Space` goes back to it;
  - for the agent of a task: select that task, then press `Space` if you talk to the Coordinator.
- The words stay put away until you are there.

## Related pages

- [The messages in the chat](board-chat-messages.md)
- [The header and the list](board-header-and-list.md)
- [Talk to a Coordinator](../how-to/talk-to-a-coordinator.md)
- [The Coordinator](../explanation/coordinator.md)
- [Keys](keys.md)
- [The hint line](keys-hint-line.md)
- [The mouse](mouse.md)
- [The chat box of the web UI](web-chat-box.md)
- [Why the board looks as it does](../explanation/board-design.md)
