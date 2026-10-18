# Why the board looks as it does

This page explains the reasons for the layout of the board: the order of the list, the marks and colours, the rows, the
narrow window, the selection, the chat and the popup. For the facts, see the reference pages in the list at the end.

## The order of the groups

The list shows first what waits for you, then what is in progress, then the to-do tasks, then the done tasks. The done
tasks are history, so they are last. However long the done group grows, it never pushes the to-do tasks down.

When the list is taller than the window, the done rows give way first, then the to-do rows. Thus the live tasks stay on
the screen, because they are the tasks that need your eye. This is also true on a phone, where each live task uses three
to five lines.

## Marks and not only colours

Each state has a mark of its own, so that colour is never the only difference between two states. With colour off
(`NO_COLOR`), the mark alone shows the difference between a task that waits and a task that went wrong.

- Only a moon shows that someone works. Thus `◐` and `◑` always mean an agent or its reviewer.
- With the dots of Herdr (`status_indicators = "dots"`), some marks stay: the `◇` of the board, the `◌` of a wait for a
  slot, and the marks of what a waiting task needs. No dot has their meaning.
- A task that waits for you did not fail. Thus its mark is never a cross. Its mark shows what it needs.
- Finished work that passed has the mark `◆`, in green, not `!`. An exclamation mark there reads as "something is
  wrong", and nothing is wrong.
- The words for the state of a task are in one list. Thus the rows, the State line, the sidebar of Herdr and the web
  page always use the same words.

## One meaning for each colour

Each colour on the board has one meaning. Thus a blue `◑` with a bar is always a running review, and an amber `◐` is
always the agent at work. A bright colour on the board means "this happens now".

- The check of the board has its own colour, a plain light grey. Thus the amber of the agent, the blue of the reviewer
  and the grey of the board are different.
- The board uses its own blue, `#3b82f6`, if the blue of the theme is too near a colour next to it. Thus the blue of the
  reviewer is always different from the colours next to it, on dark and on light backgrounds.

### Quiet tags on done tasks

After a task is merged, its `R#` tag, its `PR#` tag and its time are grey. Nearly every done task passed its review,
and a column of green says nothing. A list of done tasks is then not a wall of green. A done task has nothing live
about it, so its whole right side is quiet.

One merged tag stays red: a task that was merged although its last review failed. It is the only one that you can want
to notice later.

## The rows

### Spacing

There is one cell between two parts, and one wider gap of two cells to the right of the progress. Thus the wider gap
shows where the progress ends, and no other gap competes with it. The percent is at least three cells wide, so that it
does not move when `9%` changes to `10%`.

### Two lines for a live task

- Line 2 is in the brighter grey (`subtext0`), because you must read it. The fainter grey (`overlay0`) is for what you
  can miss: times, counts and tags.
- Line 2 is there from the start. Thus a row does not grow, and nothing under it moves, when its words arrive.
- Done tasks and to-do tasks keep one line, so that many of them show at the same time.

### Tasks that wait for a slot

The tasks that wait for a free slot are last under `working`, not under a heading of their own. You started them, and
they start by themselves, so `working` is where they belong. A heading of their own would hold one or two rows for a
minute. It would also move them under a different heading each time a slot becomes free.

### Automation rows

- The mark of an automation is `⟳`, an arrow that goes round. Every mark of a task is round, a diamond or a sign. Thus
  the two kinds of row are different.
- On or off is always a word, never only a colour.
- The board never cuts the trigger of an automation. Half a trigger is no trigger.
- The row does not show the prompt, the settings, the folder, the firings or a count. Thus it stays as calm as the row
  of a task. A count would not be correct: the board keeps only the last 200 firings, and another machine sends 20.

## The narrow window

### Rows hide the same parts together

The board decides what shows for a whole group, never row by row. Thus the columns start at the same place down the
whole list, and a long name in one group does not cost another group its columns.

### What gives way first

- **The end of a long name gives way before a column (step 4).** Thus one long title does not cost a whole group its
  columns. A title is at most 60 characters, so it loses one third at most.
- **The bar gives way first among the columns.** The words of a live task are now on a line of their own, so the name is
  alone next to the columns and they fit much more often. Before this, a bar needed a short name or a much wider window,
  and the board nearly always cut what the row said. The order was examined again with this change in mind, and it did
  not change. The bar is still the part that a row can best do without, because the percent next to it says the same.
- **Nothing jumps as the numbers change.** The board counts the minutes left at their widest, four cells. Thus a change
  from `9m` to `10m` hides nothing, and a name is not cut at a different place from one minute to the next.

### Three lines on a narrow screen

On a phone (about 50 columns), the name leaves no room next to it. Before, the bar, the percent, the minutes left, the
review tag and the pull request did not show. Now they have a line of their own, the last line of the row.

A task that has only a time for that line has two lines, not three. Thus a screen of rows that need you is not half
made of lines that hold one time.

### Long rows wrap

A phone has about 45 cells for a name. A name cut there with `…` often does not tell you which task it is. Thus, under
80 columns, long names and long text wrap.

### Why 80 columns

At 80 columns, a name of 40 cells fits next to every column at its widest. A name keeps 40 cells before a column goes.
The sum is 80:

| Part | Cells |
| --- | --- |
| The mark | 3 |
| The name | 40 |
| The bar | 8 |
| The percent (`100%`) | 4 |
| The minutes left | 4 |
| `R#12` | 4 |
| `PR#123` | 6 |
| The time | 3 |
| The gaps between them | 8 |

If the screen is narrower, line 1 always goes without some part, so the columns move down to a line of their own.

The board counts the width of the screen, never the names or the numbers on it. Thus all live rows change together. No
row gets or loses that line when a task gets a new name or a pull request. The board counts the whole screen, not the
list. A list made narrow by the task panel, on a wide screen, keeps two-line rows, because the panel shows the rest.

## The selection

- The selected row has a bar in the gutter, and inverse video where no background can be painted. Thus the task that
  the keys act on never depends on colour.
- After an action moves the selected task, the selection stays where you worked. Thus you can go through tasks one after
  another, and you do not have to move back with the arrows.

## The chat

- The board reads the words from the conversation that the program saves, not from its screen. Thus a new version of
  the program that draws its screen differently changes nothing in the chat.
- A reviewer takes no messages, because it judges the work as it is.
- When the message is hidden, the line that names who you talk to still shows. Thus you never send a message without
  knowing who gets it.
- Typed words keep who they are for, and only the draft gets the arrows. Thus an answer typed for the agent of one task
  can never go to the agent of another task.
- While the text box holds text, it has the keyboard. Thus no shortcut starts in the middle of a draft.
- `Ctrl+Z` brings back a draft only to whoever it was for. Thus a reply stays a reply.

## The board tab and the popup

- Going into an agent never ends the board. Thus you come back to the same repo, task, filter and draft.
- A board that you start with `herdr-projects` renames its tab, so that you can tell it apart from your other tabs.
- Herdr ties its popup to the tab that it opened over, and hides it, still running, when another tab shows. The board
  closes the popup at that moment, so that you never see a popup left behind. For this reason, the popup closes and does
  not follow when a command moves Herdr.
- Herdr gives its popup every key. Thus the board reads its own key from the `config.toml` of Herdr to close the popup.

## The board that looks cut off

The board draws inside what the window of Herdr shows, until it takes back the size of its pane. Thus its text box and
hints are never cut off. Its message starts with what frees the size, so that a narrow window still shows that part.
For the full explanation, see [the size of the board](board-size.md).

## Related pages

- [The header and the list](../reference/board-header-and-list.md)
- [The rows of the board](../reference/board-rows.md)
- [Marks and colours on a row](../reference/board-marks-and-colours.md)
- [The board in a narrow window](../reference/board-narrow-window.md)
- [Automations in the list](../reference/board-automations.md)
- [The chat of the board](../reference/board-chat.md)
- [The messages in the chat](../reference/board-chat-messages.md)
- [The board key, the board tab and the popup](../reference/board-tab-and-popup.md)
- [The size of the board](board-size.md)
- [What a key shows at once](under-way.md)
- [How the colours work](colours.md)
