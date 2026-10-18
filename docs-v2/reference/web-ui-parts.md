# The parts of the web board

This page tells which part of the web board does the job of each part of the terminal board. Use it to find a
button or a view on the web board.

## The web board and the terminal board

The web board does all that the terminal board does, with buttons and touch in place of keys. For the chat, the
switch and the box, refer to [the web board's chat box](web-chat-box.md).

| On the terminal board | On the web board |
|---|---|
| The list, its groups, marks, progress, and review and pull request tags | The same list ([The list](#the-list)). |
| Moving between tasks | Tap a task to select it ([Selecting a task](#selecting-a-task)). |
| The task panel | The open task ([The open task](#the-open-task)). |
| The settings page (`Ctrl+E`) | **All settings**, under the open task. |
| The hint line | The buttons at the top of the open task, with the same words. |
| The `?` menu | **More…**, and `⋯` in the header. |
| Whose board you see (`Alt+M`) | An entry in the **Boards** section, or **Other machines** in **More…**. |
| A question (approve a merge, remove a task) | A sheet with the same words and answers. |
| The message of the board to you | A short notice above the chat. |
| The automations | The same group in the same place ([Automations](#automations)). |
| Find, the archive, grouping by repo, all done tasks, other machines | The search button, and **Show** in **More…** ([Filters](#filters)). |
| A machine whose tasks the board cannot read | One notice over the list ([Machines that do not answer](#machines-that-do-not-answer)). |
| Going into the agent of a task | **Open this task's agent**: the view of the web UI for that pane. It shows the chat, or the terminal if no agent is in the pane. |

### The list

- Each status has the colour that the web UI uses for it. A task that nobody started has the dim text colour of the
  web UI.
- A pull request tag is a link.
- The agent at work is a full dot. Its reviewer is the right half of a dot. The steps of the board are the lower half
  of a dot. The page draws all three at one size. (As text, the two half discs were a third of the size of the dot.)
- Each other mark is the character of the terminal board: `✗` for a cross (there is only one cross mark), `◆` for ready to approve, `@` for a
  sign-in.
- The headings are the same as on the terminal. When every machine shows, each machine has a heading over its groups.
- Tap a heading to fold or unfold it ([list headings](list-headings.md)).

### Selecting a task

- A tap on a task selects it, as the cursor does on the terminal board. Nothing opens.
- The selected row has a bar in its gutter and a tint.
- The selected row has an **Open task** button (**Open automation** on the row of an automation). It shows the task
  whole. `Enter` on the selected row does the same.
- On a wide screen, if a task is open beside the list, the open task follows the selection. The side panel of the
  terminal board does the same.

### The open task

- The open task shows every part of the task panel whole: the status, what it needs from you, the description, the
  reviews, the evidence, what the agent said, the settings and the session.
- The evidence shows as links.
- The settings are immediately under the status and what the task needs from you. Thus a long description never
  pushes them off the first screen.

**All settings** is under the open task, and under an open automation. It is folded until you open it, or until you
pick `this task's settings` in **More…**.

- Each setting shows its plain label, its meaning where necessary (`Research only`), and its value.
- A setting that is set for this task only shows `* this task only`.
- A switch or a choice is a list to pick from. Each other setting is a box that saves when you press `Enter`.
- **Put back** removes the value that the task has for itself.
- If the board refuses a value, it tells you in its own words and changes nothing.
- A device that only watches cannot change a setting. The settings of an archived task are read-only.

### More…

**More…** shows only the actions that apply now and that are not a button already. It uses the words of the buttons,
for example `Remove this task` and `Show every machine's tasks`. A key that has no button on the page has no entry.

### Questions and notices

- For a merge or a remove, neither answer is the filled button. After a failed review, the yes answer is
  **Merge anyway**.
- A notice that tells of an error stays until you dismiss it.

### Automations

The automations are a group in the part of the list of each machine, after the task groups of that machine. When you
group by repo, they are under the repo. This is the same on a phone and on a wide screen.

- The heading is in the purple of the automations, and it folds as the other headings do ([colours](colours.md)).
- The mark is `⟳` (an arrow that goes round), at the size of the round marks. It is dim while the automation is off.
- The word `on` or `off` is where a task has its number. If the list is too narrow for the numbers, the word goes with
  them, so that each title starts at one left edge. It is then the first word under the name: `off · every 2h`.
- The name is where a task has its title.
- The next or last firing is where a task has its time, for example `next in 4m`.
- Under the name are the trigger and, if the automation was turned off, the reason. The page never clamps this to two
  lines.
- The task that the last firing made is where a task has its tags.
- A task that an automation made has its `[auto]` tag in that purple.
- A row that you open shows the automation whole, with the same buttons. Each firing is a link to its task.
- The automation of another machine has a name as the tasks of that machine do ([automations](automations.md)).

### Filters

- The search button finds tasks. **Show** in **More…** shows the archive, grouping by repo, all done tasks and other
  machines.
- A filter that is on shows as a chip under the header. Its × puts back the plain view.
- Whose tasks show is a chip too (`All machines`, `Only vm-str`). A tap on it lists every board, as the **Boards**
  section does, so that you can pick another. Its × shows the tasks of this machine only.

### Machines that do not answer

One notice over the list tells which machines do not answer:
`vm-a and vm-b do not answer, so their tasks are not shown. The board keeps asking by itself.`

- Under it is one line for each machine, with the reason, for example `vm-a: cannot connect (its name is not known).`
  **What to do** is behind it.
- The × dismisses the notice in that browser. It shows again if another machine stops answering, or if a machine
  answers for a different reason.
- A machine that the board asks for the first time has one quiet line: `Asking vm-a for its tasks…`

## Related pages

- [The web board](web-ui.md)
- [The web board's chat box](web-chat-box.md)
- [How the web board works](../explanation/web-ui.md)
- [Keys](keys.md)
- [List headings](list-headings.md)
- [Automations](automations.md)
- [Colours](colours.md)
