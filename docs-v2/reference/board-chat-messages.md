# The messages in the chat

This page gives the facts about what the message box of the board chat shows: which words, whose words, and how to
scroll back through the conversation. For the parts of the chat and who it talks to, see
[the chat of the board](board-chat.md).

## Only the words of the conversation

The box holds what the Coordinator, the agent or the reviewer said, and what was sent to it. It shows nothing of the
own screen of the program. It shows none of these: a tool call or its output, a spinner, a timing, a token count, a
tip, an input box, a rule, a footer, a start screen, or a logo.

- The board reads the words from the conversation that the program saves itself, not from its screen. Claude Code,
  Codex and pi each keep one. Thus a new version of the program that draws its screen differently changes nothing here.
- **For the other programs of Herdr**, the board does not read the conversation ([agent programs](agent-programs.md)).
  The box shows the words on their screen, under `Agent, on its screen now:`. The board removes what is drawn there
  (rules, boxes, a logo, a bar) by its shape.
- Markdown shows as text:
  - the next lines of a list item stand under its words;
  - code keeps its lines;
  - the board removes the marks of a headline, of bold text and of a code block.
- **A command that you typed to the program** (`/clear`, `/compact`, `/model` …) shows as you typed it. Then the box
  says in plain words what it did: `Chat reset: a fresh session started.`, `Conversation compacted: …`,
  `Model switched to …`.
- A stop shows as `Stopped: interrupted before it finished.`
- The own markup of the program for these (`<command-name>`, `<local-command-stdout>` …) never shows.
- **While the program works and said nothing new**, its last words stay. The line above the box shows what it does now.
- A program that said nothing yet shows: `The agent has just started: nothing to show yet.`
- **A dialog that is only on the screen of the program** (a permission prompt, a question with choices) shows in a line
  of its own, with the way to it: `The agent shows a dialog on its screen that waits for your answer: enter opens it.`
- The dialog of the Coordinator is in the `Coordinator` tab of Herdr. The dialog of a reviewer is in its `Reviewer #2`
  tab in the space of the task.
- **Going into the agent** (`Enter`) opens its real terminal, unchanged.

## While a task is under review

From the start of the review to its end, the box shows what the reviewer says (`Reviewer:`). The board keeps it up to
date as the review continues, for every program that a reviewer can be. Before the reviewer said anything, the box
shows: `The reviewer has just started: nothing to show yet.`

- A reviewer takes no messages. What you type still goes to the agent of the task.
- The review continues while you chat, as it does when you type in the own pane of the agent. It stops only when the
  agent changes the code
  ([chatting with a finished task](../explanation/how-work-gets-done-chat.md#a-review-was-under-way-when-you-wrote)).
- The empty line says both: `Send message to Agent: the review goes on unless the code changes`.
- When the review ends, the chat is the agent's chat again. The result of the review is where it always is: on the row
  and in the task panel.

## The box holds one message

The box holds the newest message, whoever said it.

- When you send a message, it shows under `You:` at once, in the bright text colour, where the message of the agent
  was. An empty line to type on is under it.
- Your message stays until the answer comes. Then only the answer shows (`Agent:` or `Coordinator:`), in grey. This is
  the same for the Coordinator and for the agent of a task. It is also the same when the answer has the same words as
  before.
- The old message never shows under your words. It is above them, with the rest of the conversation.
- This also applies to words that you send while the agent works. Claude Code queues them and they go in during its
  turn. The board reads them there.
- For the Coordinator, this stays true when you close the board and open it again. For the agent of a task, it is true
  while the board stays open.

```
╭────────────────────────────────────────────────────────────────────────────────────────────╮
│ Agent:                                                                                     │
│ Made g.                                                                                    │
│                                                                                            │
│ ❯ Send message to Agent                                                                    │
╰────────────────────────────────────────────────────────────────────────────────────────────╯
```

## Scrolling back through the conversation

The box opens on the newest thing said, and holds only that. What came before is above it.

| Key | What it does |
| --- | --- |
| `Alt+K` | Opens the box to the conversation and scrolls up through it. Each press goes further up. |
| `Alt+J` | Scrolls down, half a box at a time. At the end, one more press closes the box to the newest message again, from its start. |

- `K` and `J` are up and down, as in vim.
- This is the same wherever you opened the box, whoever you talk to, and also after new messages came while it was open.
- The last dozen things said are there. Each one is under the line that names who said it:
  - `You:` for your words, typed in the box, sent with `tell`, or typed in the own pane of the agent;
  - `Agent:` or `Reviewer:` for what it said;
  - `Coordinator:` for what the Coordinator sent to it;
  - `Board:` for what the board sent by itself: its brief, the findings of a review, the word to continue.
- A long text that was sent, such as a brief, shows only its start.
- The mouse wheel does the same, with the pointer over the chat ([the mouse](mouse.md)).
- The keys and the mouse wheel both work while you type. Neither changes the draft.
- The box shows where there is more and names the key:
  - `alt+k more ↑` on its top edge;
  - `alt+j more ↓` at the right of the line under the message. In a narrow window, it shows only `more ↓`, then only
    the arrow.

```
 ● Coordinator  idle
╭───────────────────────────────────────────────────────────────────────────── alt+k more ↑ ─╮
│ 3. The check fails on main too: it is not this task's change.                              │
│ 4. I filed proj/7 for it and put an agent on it.                                           │
│                                                                               alt+j more ↓ │
│ ❯ Send message to Coordinator                                                              │
╰────────────────────────────────────────────────────────────────────────────────────────────╯
```

- **A new message starts at its start.** You read an answer from its first line down, and one key continues from
  there.
- Only a screen (for a program whose conversation the board cannot read) starts at its end, where a terminal has what
  is new.
- **When you open the box to the conversation, it stays open until you close it.** Meanwhile, other things can happen:
  the Coordinator works, a new message comes, the board reads the list again, or tasks change. The box stays on the
  lines that you read. If the oldest messages drop off, the board finds those lines where they moved. If a reply came
  twice, the board keeps the one that you read.
- If you scrolled to the end, the box shows each new message when it comes.
- The box goes back to the newest message only when you take it there. These actions do that:
  - `Alt+J` once more from the end;
  - hiding the chat (`Alt+C`, or `Esc`) and showing it again;
  - talking to someone else: the agent of the task, the agent of another task, or the Coordinator again.
- **A tall draft never removes the message.** In a window too small for both, the draft gives rows back and scrolls
  inside the rows that it keeps. Thus three lines of the message stay, and they still scroll.

## Related pages

- [The chat of the board](board-chat.md)
- [Agent programs](agent-programs.md)
- [Chatting with a finished task](../explanation/how-work-gets-done-chat.md)
- [The mouse](mouse.md)
- [Talk to a Coordinator](../how-to/talk-to-a-coordinator.md)
- [Why the board looks as it does](../explanation/board-design.md)
