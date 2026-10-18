# The colour of the text box

This page tells how the colour of the text box shows who gets your message, and how it shows the state of a task. For
the other colours of the board, see [colours](colours.md).

The text box sends to the Coordinator or to the agent of the selected task. Its frame (the rounded box, or the two
lines of the plain box) and its prompt mark `❯` show which:

| The box sends to | Colour of its frame and `❯` |
|---|---|
| the Coordinator (also: a new task, find, a task whose message goes to the Coordinator or into its description) | the theme's plain text colour |
| the agent of a task, or a task where your message starts an agent | the status colour of that task, the same colour as its mark in the list |
| a machine's coordinator ([machine and project coordinators](../explanation/machine-and-project-coordinators.md)) | the colour of the machine's heading: the theme's accent (blue in most themes) |
| a repo's coordinator | the colour of the repo's heading: the brighter grey (Herdr's `subtext0`) |

A machine's or a repo's coordinator also has the colour of its heading in these places:

- its name on the line above the box (`vm-str coordinator  working`: the name is in its colour, and the state is grey);
- the speaker names in the chat (`You:`, `vm-str coordinator:`);
- its name in the hints (`space talk to the vm-str coordinator`, `enter send to the vm-str coordinator`).

The web board does the same for the frame of the text field, the name above it and the speaker names, in the colours of
its headings. The Coordinator keeps its plain look.

The box of an agent shows the state of its task:

| State | Colour of the box |
|---|---|
| the agent works | amber |
| the task needs you | red |
| ready to approve | green |
| under review | blue |
| queued (for a free slot, or after other tasks) | pink |
| done | teal |
| nobody started the task, or the task is archived (read-only) | dim grey |
| the board runs its own check on the task | the plain light grey of the mark ([check progress](../explanation/check-progress.md)) |

- The colour changes with the status, also while you type.
- The colour is the same in every box: under the list, in the reply view, under the task panel and full screen, in the
  popup, for the tasks of this machine and of another machine, and on the web board (the frame of the text field).
- The box and the mark always have the same colour.
- Other clues: the line above the box names the recipient. The grey words in the empty box name it
  (`Send message to Coordinator`, `Send message to Agent`, `your message starts an agent on it`). While you type, the
  hint line names it (`enter send to the Coordinator`, `enter send to HSW-34's agent`). With `NO_COLOR`, the board
  draws no colour, and each state has its own mark.

On the web board, done is a teal of the board's own.

## Related pages

- [Colours](colours.md)
- [Why the board uses these colours](../explanation/colours.md#why-the-box-has-the-colour-of-the-recipient)
- [The chat of the board](board-chat.md)
- [Machine and project coordinators](../explanation/machine-and-project-coordinators.md)
- [Check progress](../explanation/check-progress.md)
