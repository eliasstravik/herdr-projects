# The board in a narrow window

This page gives the facts about what the rows of the board hide when the window is narrow, how rows look under 80
columns, and what the board shows when it looks cut off.

## Rows hide the same parts together

The board decides what shows for one group at a time, never row by row. A group is the rows under one heading:
`working`, `done`, and so on.

- Each part shows on every row of the group that has it, or on no row of the group.
- Example: if one name under `working` leaves no room for a bar, no row under `working` has a bar.
- A long name under `done` has no effect on `working`.
- The columns still start at the same place down the whole list.
- On a screen under 80 columns, the columns of a live row are on its last line ([narrow screens](#narrow-screens)).
  The order below then applies to the one-line rows only. Those rows show no time there.

## The order in which parts give way

The parts give way in this order, first to last. The name wins over all of them.

1. What one-line rows say after their name (the note of a to-do task). The board cuts each one to its own room. If one
   of them does not have 20 cells (or its whole length, if that is less), no row of the group shows its words. The
   words of a live task are on its second line, and this step does not apply to them.
2. The bar.
3. The word `ago`, on the whole list at the same time, because it is to the right of every column. It goes if that
   keeps a column other than a bar somewhere on the list, or cuts a name less somewhere on the list.
4. The end of a name longer than 40 cells. The board cuts it with `…`, down to 40 cells, before its group loses a
   column.
5. The percent and the minutes left, together.
6. The `R#` tag and the pull request, together. A review never shows without its pull request, and a pull request
   never shows without its review.
7. The time.
8. The number of the task, on the whole list at the same time, so that the names stay in one column. It goes if it
   leaves the name of a row fewer than 16 cells, with nothing else next to it.
9. The rest of the name.

More rules:

- A title is at most 60 characters ([titles](titles.md)). Thus step 4 cuts at most one third of a name.
- A name of 40 cells or fewer is never cut while a column shows.
- A name is never cut for a bar.
- The words of a live task are on a line of their own. Thus its name is alone next to the columns, and the columns fit
  much more often.
- From 110 columns, every live row has all its columns, the bar too, whatever its name.
- The minutes left are counted at their widest, four cells, when the board decides what shows. Thus `9m` changing to
  `10m` hides nothing.
- The minutes left are never wider than four cells. From a thousand minutes, a row shows hours (`17h`).
- The board draws a name in the room that it counts in this way. Thus the name is not cut at a different place from one
  minute to the next.

Why the order is as it is: [why the board looks as it does](../explanation/board-design.md#what-gives-way-first).

## Narrow screens

Under 80 columns, the row of a live task has three lines. The last line holds the bar, the percent, the minutes left,
the review tag and the pull request ([why 80 columns](../explanation/board-design.md#why-80-columns)).

```
 working
 ◐ Third row line on narrow screens for progress
   Writing the tests for the third line
   ██████████████░░░░░░░░ 35% 14m left        9m
 ◐ Show progress while the board runs the check
   Reviewing progress display fix against earli…
   █████████░░░░░░░ 62%  4m left  R#2 PR#82  38m
 ◐ Grey out review tag and time once merged
   Just started                              20s
```

- **Line 1**: the mark and the name, with the whole width.
- **Line 2**: what the task says, as on a wide screen.
- **The last line**: the bar, the percent, the minutes left, the `R#` tag, the pull request and the time, in that
  order and in their usual colours.
  - It starts where the title starts: to the right of the mark and of the numbers of the tasks. Thus a row has one left
    edge.
  - It ends at the right edge.
  - Each part is as wide as the widest of its kind on the list. Thus the parts start under each other on every live
    row.
  - The bar uses all the room that is left. Every bar has the same length, and the line fills the width.
  - The time ends at the edge, with or without `ago`.
- If a task has only a time for that line (no progress, no review, no pull request), it has no such line. Its time is
  at the end of what it says, and the row has two lines.
- Done rows and to-do rows have one line.
- The whole screen width counts, not the width of the list. A list that the task panel makes narrow on a wide screen
  keeps two-line rows. The panel shows the other parts.
- From 80 columns, nothing wraps, and every row is as on a wide screen.

### Long rows wrap

Under 80 columns, long rows wrap. The board does not cut them.

```
 working
 ◐ Research capy.ai and other coding agents for
   ideas we could use on the board
   Researching coding agents and harnesses for
   ideas, reading their docs and changelogs one …
   █░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   3%   34s

 to do
 ○ Check herdr-projects /autoproject for ideas w…
 ○ Bring the board to herdr's web UI
 ○ Rethink how the Coordinator works, restarts
   and remembers what it was told before
```

- **The name of a live task uses up to two lines. What the task says also uses up to two lines.** The board cuts each
  one with `…` only after its second line.
- Text that fits one line uses one line. A live row has at least three lines and at most five lines. The line of the
  bar and the time is always the last one.
- **The selected task shows its whole name**, on as many lines as it needs, in every group (the third to-do task above).
  For this, it does not show the tags next to its name. When you select a different task, it has one line again.
- **Done rows and to-do rows have one line, without their time** (`5h`). The name uses that room. The tags (`R#`,
  `PR#`) still show if there is room next to the name. The time of a task is in its panel.
- **Wrapped lines start under the name**, never under the mark. Thus the marks stay in a column of their own.
- In the view of all machines, a name that wraps has the line to itself, without the name of the machine before it.
- In the view of all machines, what a task says starts under the name of the machine, if a long name of a machine
  leaves it fewer than 20 cells a line.
- No limit on the length of a name or a sentence changed. Only how the board draws them changed.
- The highlight covers every line of the selected row. The arrows still move from task to task.
- The list counts each row at its own height. Thus the rules for a list taller than the window, and the scroll, are
  the same ([a list taller than the window](board-header-and-list.md#a-list-taller-than-the-window)).

### Very narrow screens

Under 38 columns, with every column there, the parts give way in this order:

1. The word `ago`.
2. The bar.
3. The `R#` tag, with the pull request.
4. The percent, with the minutes left.

The time stays.

## A board that looks cut off or half empty

What changes the size of the board, and why the board does not draw a size that lasts only a moment:
[the size of the board](../explanation/board-size.md).

- **A view that somebody looks through** (the board open in the terminal view of the web UI) keeps the terminal. The
  first line of the board says so.
- **The board takes back the size of its pane from any other view, by itself, within a few seconds.** It disconnects
  the remote view. Until then, the board draws inside what the window of Herdr shows. Thus its text box and its hints
  are never cut off.

The rest of this section is what you see until then, and when the board cannot take the size back.

A remote view of the pane of the board is a `herdr terminal attach`. The web UI of Herdr starts one for each pane that
it shows. The remote view sets the size of that pane to its own size. It keeps that size while it runs, also after its
browser tab is closed. The window of Herdr then shows a cut-out of a board drawn for a different size.

The board sees this within ten seconds. It then shows a message on its first line, at the top left, where a cut-out
still shows it. The message starts with what frees the size, so a narrow window still shows that part:

```
 ! Shown cut off: a remote view holds this pane at 435×58 (herdr's own window: 184×78). ctrl+k ends that view
   (open 40m); closing it where it runs does the same.
```

- The message is a sentence. The board wraps it and never cuts it. It names no process to kill.
- **`Ctrl+K` ends that view from the board.** The key belongs to the board only while such a view holds the pane.
- If you close the view where it runs, the pane also gets its own size back at once.
- The message does not show outside Herdr, in a tmux inside the pane, or in the popup of Herdr. The popup has no pane
  of its own.

## Related pages

- [The rows of the board](board-rows.md)
- [The header and the list](board-header-and-list.md)
- [Automations in the list](board-automations.md)
- [Titles](titles.md)
- [The size of the board](../explanation/board-size.md)
- [The web UI](web-ui.md)
- [Why the board looks as it does](../explanation/board-design.md)
