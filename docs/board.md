# The board: the list and the chat

**The header says where you are**, on one line: whose tasks show, then how many stand in each group (`herdr-projects
proj · 3 need you · 1 ready · 2 working · 12 to do · 30 done`). With other machines in view that comes first, in the
accent colour, and is never cut (names too long for the line go on to the next one, whole): `all machines: otherbox`, or
`only otherbox` ([other machines](other-machines.md)); otherwise it is the repo new tasks go to. There is no folder path
on it. A filter has a line of its own under it, in the accent colour, which says what it is, how many tasks match and
the key that clears it: `filter "import" · 1 of 33 · esc clears`; with nothing found the list says `Nothing matches
"zzz".`

**The list.** Under the header, the tasks in five groups, in the order they need you:

| Group | Holds |
|---|---|
| `needs you` | a question, something that went wrong, a merge that failed, finished work whose review failed |
| `ready to approve` | finished work that passed its review (or had none) and waits for your yes |
| `working` | an agent or the reviewer is on it |
| `to do` | nobody has started it |
| `done` | finished, newest first |

Done is history, so it is last: however long it grows, it never pushes the to-dos down. First what waits for you
(`needs you`, `ready to approve`), then what is being worked on (the agent, the reviewer, the board's check), then
`to do`, then `done`. A pinned task stays on top, above all of them. Grouped by repo (`Alt+S`) each repo's rows follow
the same order, so a repo's done rows are below its to-dos. Only the latest five done tasks show; the line that
counts the rest (`↓ 73 more done (alt+d shows them)`) is under them, at the bottom, and `Alt+D` shows them all. `↑`
and `↓` follow the order shown (`PgUp` and `PgDn` go ten tasks at a time), and changing the grouping keeps the
selected task selected. The archive (`Alt+A`) is a list of its own, newest first, and no part of this order.
The arrows stop on the headings too, and `Enter` on one folds it to its one line, and unfolds it; with every machine
shown each machine has a heading of its own: [list headings](list-headings.md).

**The automations are the last group**, under the heading `automations`, below every task group (also when the
list is grouped by repo); with none there is no heading. The heading and the mark of each row are in a colour of
their own, a purple no status has ([colours](colours.md)). One row per automation, **in the list's own columns**,
drawn by the code that draws a task's row:

```
 to do
 ○ HSW-2 Write the changelog                                                                  1d ago
 ○ HSW-1 Fix the footer on the pricing page                                                   3h ago

 automations
 ⟳ on    Look for old dependencies  every 2h                                 ✓ HSW-4 next in  4m
 ⟳ on    Release note after a merge  when a task is merged, in every repo            skipped  5h ago
 ⟳ off   Nightly audit  daily 03:00                                                      never fired
         it fired 30 times in an hour
```

| Where a task's row has | An automation's row has |
|---|---|
| its mark | `⟳` (U+27F3), in the automations' colour while it is on and the same one dimmed while it is off: an arrow going round, where every task's mark is round, a diamond or a sign |
| its tag (`HSW-12`) | the word `on` or `off`, in the same grey. It fills the numbers' column, so every name starts where the titles start |
| its title | its name, plain as a to-do's title; greyed while it is off |
| what it says, after the title | its trigger in plain words (`every 2h`, `when a task is merged, in every repo`), in the same grey; then its repo when that is not the one the board shows, and its machine (`on otherbox`) on the board of every machine |
| its review tag and pull request | the task its last firing made, by that task's own mark and number (`✓ HSW-4`: done; `◐ HSW-7`: an agent is on it), and the word that goes with the time (`next in`, `fired`, `skipped`) |
| its time | how long until it fires (`4m`), or how long ago it last did (`2h ago`), in the list's time column, under the tasks' times. `never fired` stands there for one that never did |
| a second line, for a live task | a second line only when it has something to say: why the board turned it off. Where the trigger has no room beside the names it stands there too, on every row of the group at once |

What gives way when room is short does so as on a task's row and by the same code (the order is below, under
"When the window is narrow"):
what the row says (to the line under the name, never cut: half a trigger is no trigger), then the part of a
long name past 40 cells, the last task, the firing, the numbers' column, and then the rest of the name.
Where the numbers' column goes, the word `on` or `off` goes with it, so every name still starts where the titles
start, and it is then the first word of what the row says (`on · every 2h`): on or off is a word at every width,
never colour alone. Under 80 columns a row is its name, and under it the trigger with the
firing at the end of that line, as a live task's row has its time there; a long name wraps on the selected row and
is cut with `…` on the others. The selected row has the same bar and tint, a click selects it, and `↓ 3
automations` says how many do not fit.

**Not on the row**, so that it stays as calm as a task's: the prompt, the settings, the folder, the list of firings
and why one was skipped, the hour a schedule is next due, and how many times it fired (only the last 200 firings are
kept, and 20 are sent from another machine, so a count would not be true). All of that is the automation, whole
(`Enter`).

`↑` and `↓` go from the last task on to them. On such a row no task is selected, so the keys that act on a task do
nothing and the row has keys of its own (open it, turn it on or off, fire it now, remove it, its settings), which
the hint line shows: [automations](automations.md). One that is off is drawn as quietly as a done task: its mark
dimmed, its words greyed. Find (`Ctrl+F`) matches them too, by name, trigger or repo; the archive has
none. No task gives way for them: where the list is longer than its room, the tasks fold as they always do and the
automations are what is below.

**The list says what it hides.** A list longer than its room keeps its last line to say how many tasks are not on
the screen, folded ones included: `↓ 11 more`, and `↑ 4 more` once you have scrolled. Automations that do not
show are counted apart and by their name, so the line says what it is that is hidden: `↓ 3 more, 2 automations`,
`↓ 7 automations`, `↑ 4 more, 1 automation`. A heading never stands with
nothing under it at the bottom of the screen. (A list of fewer than eight lines keeps every line for tasks.)

**The selected row has a mark.** Every line of it starts with a bar in the gutter (`▌`), in every list (tasks and
automations, the archive, the settings page, the `?` menu), on the theme's own tint. Where no background can be
painted (`NO_COLOR`, or a theme with none for it) the row is in inverse video instead, so which task the keys act on
never hangs on colour. A click on a row selects it ([the mouse](mouse.md)).

**The selection stays where you were working.** When an action (put an agent on it, say yes, send it back, stop,
pin, archive) moves the selected task to another group, elsewhere in the list or out of it, the selection does not go
with it: it is on the next task below in the same group, or the one above when the task was the last there; when the
group is left empty, on the next group's first task (on its heading when that group is folded), else on the group
above. So you go through tasks one after another without arrowing back. A folded group a task moves into stays
folded; under a filter the next task that matches it is the one; grouped by repo the same holds inside the repo. When
the task leaves the list (archived), the message says which task the selection is on now. An action that does not
move the task leaves the selection on it. With the task open whole (the panel) or while you talk to its agent, the
selection stays on that task, and so it does while you are in its agent.

Each state has a mark of its own (`◐` the agent is working, `◑` the reviewer is working, `◇` the board's
own step, `◌` queued: it waits for a free slot, or for the tasks it is tabbed to start after
([one task after another](after.md)), `✗` went wrong, `◆` ready to approve, `✓` done, `○` to do; the `?`
menu has the list), so no
two are told apart by colour alone. **Only a moon says someone works**: `◐` and `◑`, an agent or its reviewer. Every
mark is one column wide. herdr's `status_indicators = "dots"` brings its dots back for working (the agent's and
the reviewer's) and done; the board's `◇`, the `◌` of a wait for a slot and what a waiting task needs keep their
marks with dots too, as no dot means those.

A task that waits for you is not one that failed, so its mark is never a cross. It says what the task needs, the
`kind` of the terminal program-status spec ([OSC 7501](https://mitchellh.com/writing/program-status-osc7501)):

| Mark | It needs | Its State line |
|---|---|---|
| `?` | an answer: the agent asks something and waits | `asks you` |
| `!` | a yes or no: a permission dialog, a merge that did not go through | `asks for your OK` |
| `!` | your call on finished work whose last review failed: send the review back, or merge it anyway | `review failed` |
| `@` | a sign-in: the agent's program is logged out | `needs a login` |
| `!` | you, and the board cannot tell for what (the kind is not known) | `needs you` |
| `✗` | nothing of these: it went wrong | `needs you` |

All of them are under `needs you` and in its colour, red; with colour off (`NO_COLOR`) the mark alone tells a task that
waits from one that went wrong. Finished work that passed and waits for your yes is none of these: its mark is `◆`, in
green, under `ready to approve` (an exclamation mark there would read as "something is wrong", and nothing is). Every
mark is a character every monospace font has; the web board draws the same ones. The words for where a task stands are
one list (`crates/core/src/status_words.rs`), which the rows, the State line, herdr's sidebar and the web page all read.
The records the board writes to the terminal carry no mark: they name the state and the kind
(`state=blocked:kind=question`) and the terminal draws its own.

A row reads: the task's tag, greyed (`HSW-134`, its repo's short key and its number:
[task numbers and repo keys](task-numbers.md)), the
name, then on the right two groups with a wider gap between them: the progress (the bar, the percent and the minutes
left), and then the review, the pull request and the time:

```
 needs you
 ! #98  Add an undo for clear                                         R#2 PR#18 12m ago
        Review failed · 1 critical, 1 major
 ready to approve
 ◆ #99  Max length for task titles                                    R#3 PR#60  2m ago
        Pass · 1 minor
 working
 ◐ #134 Tidy the first-run questions            ███░░░░░ 40% 12m left  R#1 PR#17  5m
        Running the tests
 done
 ✓ #76  Blue while the reviewer works                                 R#1 PR#76  1h ago
```

**The spacing is one rule: one cell between two things, and one wider gap of two cells on a row, right of the
progress.** So the wider gap says where the progress ends, and no other gap competes with it. Each thing is a column
down the whole list, as wide as its widest value in view and no wider (the percent three cells at least, `45%`, so it
does not move when `9%` turns `10%`). The numbers end under each other, so the only other blank cells are those before a
shorter one: `5m` under `12m`, `R#1` under `R#12`. A pull request starts under the others, and a column a row has
nothing for is left blank. The last line of a row on a narrow screen (below) follows the same rule.

**A live task's row is two lines.** Live is every task under `needs you`, `ready to approve` and `working`: an agent,
the reviewer or the board's own check is on it, or it waits for you. The first line is the mark, the name and the
columns. The second is what the task says, greyed, under the name and starting where the name starts: what it is doing
now, how its review came out with a short tally of what it found (`Pass · 2 minor`, `Review failed · 1 critical, 1
major`, the result in its own colour), or what it waits for. While the reviewer works, it says only what the reviewer is
doing. It is in the brighter of the board's two greys, herdr's `subtext0`, because it has to be read (the fainter one,
`overlay0`, is for what may be missed: times, counts, tags); what a task asks of you is in the plain text colour. The
minutes left say what they are (`25h left`); the time after them is how long the task has worked, or how long ago it
stopped. In the all-machines view the machine is the heading above its tasks, never in a row ([list
headings](list-headings.md)), so the second line starts under the task's name as everywhere. That line has the row's
whole width, so it is rarely cut; one longer than the window is cut at its end with `…`, never wrapped (on a narrow
screen it wraps: below).

A done or to-do task has nothing to say and keeps one line, so that many of them show at once. (A to-do task that
carries a note says it after its name, on that one line.)

The second line is there from the start, so a row does not grow, and nothing under it moves, when its words arrive.
It is never empty and never a bare `…`: a task with no words of its own says something true and quiet there (`Just
started`, `Yours: no agent on it` for a task you marked as your own, `Not reviewed` for finished work nobody reviewed,
or the plain word for where it stands).

The highlight of the selected task covers all its lines. `↑` and `↓` go task by task, never onto a second line.

**On a narrow screen a live task's row is three lines.** On a phone (about 50 columns) the name leaves no room
beside it, so the bar, the percent, the minutes left, the review tag and the pull request used to be hidden. Under 80
columns they have a line of their own, the last of the row:

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

- **Line 1** is the mark and the name, with the whole width.
- **Line 2** is what the task says, as on a wide screen.
- **The last line** is, in this order, the bar, the percent, the minutes left, the `R#` tag, the pull request and the
  time, in their usual colours. It starts where the title starts (right of the mark and of the tasks' numbers, so a row
  has one left edge) and ends at the right edge. Each of them is as wide as the widest of its kind on the list, so they
  start under each other on every live row, and the bar takes all the room that is left: every bar is the same length,
  and the line fills the width. The time ends at the edge with the word `ago` or without it.
- A task with nothing for that line but a time (no progress, no review, no pull request) has no such line: its time
  stands at the end of what it says, and the row is two lines. A screen of rows that need you is not half made of
  lines that hold one right-aligned time.
- Done and to-do rows stay one line.

**On a narrow screen long rows wrap instead of being cut.** A phone has about 45 cells for a name, and a name cut
there with `…` often does not say which task it is. So under 80 columns:

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

- **A live task's name takes up to two lines, and so does what it says.** Each is cut with `…` only after its
  second line. What fits one line takes one: a row is three lines at least and five at most, with the line of the
  bar and the time always the last.
- **The selected task shows its whole name**, on as many lines as it needs, whichever group it is in (the third
  to-do above). It gives up the tags beside its name for that. Move on, and it is one line again.
- **Done and to-do rows stay one line, without their time** (`5h`): the name has that room, and the tags (`R#`,
  `PR#`) still show where there is room beside it. The time of a task is in its panel.
- **Wrapped lines start under the name**, never under the mark, so the marks stay a column of their own. In the
  all-machines view a name that wraps has the line to itself, without the machine's name before it; and what a task
  says starts under the machine's name when a long one would leave it fewer than 20 cells a line.
- No limit on how long a name or a sentence may be has changed: this is only how they are drawn.
- The highlight covers every line of the selected row, the arrow keys still go task by task, and the list counts
  each row at the height it has: what gives way in a list taller than the window, and the scroll, are as below.

From 80 columns on, nothing wraps and every row is as it was.

**Where it switches: 80 columns of screen.** At 80 a name of 40 cells (what a name keeps before a column goes, below)
sits beside every column at its widest: the mark (3), the name (40), the bar (8), the percent (4, `100%`), the minutes
left (4), `R#12` (4), `PR#123` (6), the time (3) and the gaps between them (8). Narrower than that, line 1 always goes
without something, so the columns move down; from 80 on the row is the two lines it was, and nothing changes. It is the
screen's width that is counted, never the names or the numbers on it: all live rows switch together, and no row gets or
loses that line when a task is renamed or gets a pull request. And it is the whole screen, not the list: a list made
narrow by the task panel beside it, on a wide screen, keeps two-line rows (the panel shows the rest).

In a width too small even for that line (under 38 columns with every column there) things go in this order:
the word `ago`, the bar, the `R#` tag with the pull request, the percent with the minutes left. The time stays.

**A list taller than the window.** The done rows give way first: the list shows the latest five of them, and when it
still does not fit, one fewer for each line it is short, down to the newest one (the line under them says how many more
there are, and `?` has the key that shows all of them). Then the to-dos, from the last one up, down to the first, with a
line under them that counts the rest (`… 4 more to do`). So the live tasks, which are the ones that need your eye, stay
on the screen, also on a phone where each takes three lines to five. The task you have selected is never folded away,
and nothing is while you search or have asked for all done.

A to-do that gave way is still one `↓` away: the arrow keys go through every to-do, and the ones that show are then
the ones that end at the selected task. To-dos give way only on a list that has live tasks: with none there is
nothing to keep in view, and the list scrolls as it always did, with no line spent on a count.

Only when the list is still too tall does it scroll, and then by whole rows: a row of more than one line is never
shown in part. Its lines come into view together, and where only some would fit at the bottom those lines are left
empty. (In a list squeezed to a single line, the selected task shows its name.)

**Each colour has one meaning.** Amber: the agent is working. Blue: the reviewer is working. Green: it passed and is
ready for you. Red: it failed, or it needs you. For as long as the reviewer works, the row's mark, its bar, the
percent, the minutes left and the `R#` tag are blue, and so are the same things in the task panel, on the details
screen and on the chat line. The mark says it without colour too: the half moon faces the other way, `◑`, while the
reviewer works, and is the agent's `◐` again once the task goes back to its agent. Only those two moons say someone
works. The board's own steps are no moon but a `◇`: starting, setting up, running the check, pushing and merging,
and the waits on the way, for the check's turn, for its turn to merge and for GitHub's checks on its pull request
(`Waiting for GitHub checks · 2 of 3 passed`). A task that waits for a free slot has an empty, dotted ring, `◌`, in
a pink of its own (below). An agent or a reviewer whose turn was cut off and that waits to try again keeps its moon:
it is still the one on the task. So a blue `◑` with a bar says a review is running, an amber `◐` that the agent is
at work, a `◇` that the board is on it, a pink `◌` that it has not started yet, and a green `!` that the task is
ready for your yes. A group's heading keeps the colour of
its group: a task under review is filed under `working`.
A task nobody has started (`to do`: its heading and its `○`) is a dim grey of its own, never the plain text colour,
and always readable: every grey on the board is held to a floor of contrast with the background.
Which colours are herdr's and which the board's own, and how the board follows herdr's theme: [colours](colours.md).

The blue is the theme's own `blue` where that one is told apart from everything it sits beside: the theme's green,
its red, its amber, the light blue of herdr's own dot for done and idle (its `teal`), muted text, plain text and the
background. In many themes it is not (Dracula's blue is its teal, Rosé Pine's its green, Catppuccin's a pale one near
its muted text): there the board uses a blue of its own, `#3b82f6`, which reads on a dark and on a light background
alike. Colours you set yourself (`[theme.custom]`) are held to the same rule: a `blue` that is your red, or that is no
blue at all (a green, a purple, a grey), is not used,
and where the board's blue is too near one of your colours it takes another blue that is not. With the terminal's own
colours it is the terminal's blue.

**A task that waits for a free slot.** At most `board.agents` agents work at once. A task started while every slot is
taken waits its turn: its row has the mark `◌` in pink, says `Waiting for a free slot` on its second line, and has its
place in the queue where a task at work has its percent: `Q#1` gets the next free slot, `Q#2` the one after.

```
 working
 ◐ HSW-151 Rewrite the board in Rust on Ratatui           █████░░░  65%  3m       9m
           Reading the list code
 ◌ HSW-152 Show automations in the main list                         Q#1           2m
        Waiting for a free slot
 ◌ #153 Fix the footer                                               Q#2          40s
        Waiting for a free slot
```

- **The order is the one the board starts them in**: the first to wait first. When a slot frees, `Q#1` starts and
  the others move up. A task's attempts each wait for a slot of their own and are counted the same way.
- **They stand last under `working`, in that order**, not under a heading of their own. You started them and they
  start by themselves, so `working` is where they belong; a heading of their own would hold one or two rows for a
  minute and move them under another heading each time a slot frees. Their mark, their colour and their `Q#` tell
  them from the tasks at work, and the order says which one is next. (Grouped by repo there is no status heading:
  they stand among their repo's tasks.)
- The same place is said everywhere: the task panel's State line (`◌ waiting for a free slot, Q#2`), `list`
  (`· waiting for a free slot, Q#2`; `--json` has `queue: 2`), `details` (`queue: { place, words }`), the message
  when you start it, herdr's sidebar ([sidebar](sidebar.md)) and the web Board.
- A narrow board keeps it too: there `Q#2` stands with the time at the end of the row's second line, so a waiting
  task's row is two lines, not three.

`R#2` is the review the task is on, or the last one it had. It belongs to the pull request, not to the progress: it sits
one cell left of the `PR#`, and the tags end under each other down the list. Its colour says how that review went, and
it has no mark beside it: green, it passed; red, it failed, and it stays so while the agent fixes what the review found;
blue, the reviewer is on it right now, whatever the review before it said. The three are told apart in the light themes
and the dark ones alike. A review from before the gate goes by its advice. Once the task is merged (through its pull
request, or here on this machine with none) the tag goes quiet, the same grey as its merged `PR#` tag: nearly every done
task passed its review, and a column of green says nothing. One stays red: a task merged although its last review failed
(you approved it anyway), the only one worth noticing later. The task panel and the details page do the same with how
each review went, and so does the archive. A task that was never reviewed has no tag. Every row that has had a review
keeps it: under review, back with the agent, waiting for your yes, done.

**When the window is narrow, rows hide the same things together.** What is shown is decided for one group at a time
(the rows under one heading: `working`, `done`, ...), never row by row: each thing is on every row of the group that
has it, or on none. So if one name under `working` leaves no room for a bar, no row under `working` has a bar. A
long name under `done` costs `working` nothing, and the columns still start at the same place down the whole list.
(On a screen under 80 columns a live row's columns are on its last line, above: what follows is then about the
one-line rows, which have no time there.)

Things give way in this order, first to last, and the name wins over all of them:

1. what one-line rows say after their name (a to-do task's note): each is cut to its own room, and when one of them
   has not even 20 cells (or its whole length, when that is less), none of the group's rows says anything. A live
   task's words are on its second line and take no part in this;
2. the bar;
3. the word `ago`, on the whole list at once since it sits right of every column: it goes when that keeps a column
   other than a bar, or cuts a name less, somewhere on the list;
4. the end of a name longer than 40 cells: it is cut with `…`, down to 40, before its group loses a column;
5. the percent and the minutes left, together;
6. the `R#` tag and the pull request, together: a review is never shown without its pull request, nor the other
   way round;
7. the time;
8. the task's number, on the whole list at once, so that the names stay in one column: it goes when it would leave a
   row's name fewer than 16 cells with nothing else beside it;
9. the rest of the name.

Step 4 is why one long title does not cost a whole group its columns: a title is at most 60 characters
([titles](titles.md)), so it loses a third at the very most, and a name of 40 cells or fewer is never cut while a
column shows. A name is never cut for a bar.

With what a live task says on a line of its own, its name is alone beside the columns, so they fit far more often: a
title is at most 60 characters, and from 110 columns on every live row has all of them, the bar too, whatever its
name (before, a bar needed a short name or a much wider window, and what the row said was nearly always cut). The
order above was looked at again with that in mind and is unchanged: the bar is still what a row can best do without,
since the percent beside it says the same.

What is shown does not jump as the numbers tick: the minutes left are counted at their widest, four cells, when
this is decided, so `9m` turning into `10m` hides nothing. They are never wider: from a thousand minutes on a row
shows them as hours (`17h`). A name is drawn in the room counted that way too, so it is not cut at another place
from one minute to the next.

`PR#18` is the pull request, and its colour says how its checks went, by the same rule as the `R#` tag and with no mark
beside it: green, they pass; red, they fail; plain, there is no result yet (they are running, or there are none). A
merged pull request goes quiet, grey like the time beside it: nothing there needs your eye, so a list of done tasks is
not a wall of green. The greens and reds are the theme's own, the ones the `R#` tag uses, and the three are told apart
in the light themes and the dark ones alike. The task panel and the details page colour it the same way. A bare time is
how long it has been working; `ago` is since it stopped. On a finished task the time is quiet like its merged tags, in
the very same grey: a done task has nothing live about it, so its whole right-hand side (review tag, pull request, time)
is quiet, and a bright colour on the board means "this is happening now". That grey is the theme's own for greyed-out
words, made readable where the theme's is too near the background ([colours](colours.md)). The time of a task that is
not done is drawn as it always was. A row never repeats what its heading says.

**While the board runs the check itself** (at the finish line, and again after a catch-up with main), the row shows
real progress, not the Watcher's guess: the bar and the percent are the count the check reports, the minutes left
come from how long its last full runs took on this machine. How a check reports, and what a repo's own check can
do: [the check's progress](check-progress.md).

```
 ◇ Add a greeting                                    █████░░░  65%  3m       9m
   running the check · tests 31 of 47
 ◇ Fix the footer                                                     6m       2m
   waiting for its turn · 2 ahead
```

- While it waits its turn (one test run at a time on a machine) the row says how many are ahead and shows the
  minutes, with no bar: nothing of its own moves yet.
- Code that passed already is answered at once: `the check already passed for this exact code`, no bar, and on it goes.
- A check that reports no count shows the time it has run against last time (`running the check · 2m of about 5m`),
  and never a percent.
- When nothing moves for far longer than a run takes, the row says so: `the check stands still · nothing finished for
  13m · tests 31 of 47`, `waiting for its turn unusually long · …`, `the check runs far longer than usual · …`.
- Its colour is its own: the mark, the bar, the percent and the minutes left are a plain light grey (the theme's
  `subtext0`) while the board checks, so the agent's amber, the reviewer's blue and the board's grey read apart.

The same line, bar, percent and minutes show in the task panel, on the details page, on the chat line and in
herdr's sidebar, and the rows of a section still give the same things up together.

On the chat line the bar, the percent and the minutes left stand right after the words, in the row's colour, for
the task whose agent you talk to, while the board runs its check. They are never cut: when the
line is short the words are cut first (down to twenty cells), then the bar goes, then the numbers.

**One chat, at the bottom.** The text box is a chat, and it is the same chat whoever you talk to. It always talks to
someone and always says who: the Coordinator, until `Space` (on an empty text box) flips it to the selected task's
agent; `Space` again flips it back. Ask it something, tell it to do something, or just say what needs doing.

The arrow keys only move the selection; they never change who you talk to. While you talk to a task's agent the
chat follows the selection: move to another task and you talk to that task's agent, with its latest message shown.
A task with no agent to talk to is still named, with where your message goes instead: it starts an agent on a task
that is to do, goes to the Coordinator about that task otherwise, and an archived task is read-only. With no task
selected (words on their way to another machine, an empty list) the chat names the Coordinator until a task is
selected again. If the task you talk to goes away (removed), you are back with the Coordinator, never on
to a neighbour's agent.

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

The chat is three things, and looks the same for the Coordinator and for a task's agent, at every window width:

- **One line that says who you talk to**: the board's own mark for where they stand (the mark the task's row has),
  the name in bold, and one greyed line: what the task's row says (the Watcher's "doing now" line) or where it stands,
  for the Coordinator `idle`, `working`, `needs you`, `new reply`, and how many of your messages still wait to be
  sent. It is cut with `…` where it does not fit; the name is never cut to make room for it (a name longer than the
  window takes a second line; in a window too short for that, the draft keeps the line). No program, model or effort
  is named there, and no keys. (One thing gets a greyed line of its own under it: a default that changed while a
  Coordinator runs, with the command that starts a new one on it.)
- **One rounded box**: the newest thing said in that conversation, in grey, under a line that names who said it
  (`Coordinator:`, `Agent:`, `Reviewer:`, `You:`), then an empty line, and the line you type on (the prompt is only
  `❯`). While that line is empty it says in grey who gets your message, by the name of their tab: `Send message to
  Coordinator`, or `Send message to Agent` for a task's agent. Nothing else is in it. The box's frame and its `❯`
  say it by colour too: the plain text colour for the Coordinator, the task's status colour for its agent
  ([colours](colours.md)). A long message wraps, and the
  box grows with it: it takes the lines the message needs, up to two thirds of the window, and shrinks back for a
  short one. The list above gives up those rows and keeps the selected task in view. (Under the whole task, `Alt+T`,
  and the `?` menu, which are there to be read themselves, the message keeps to a third of the window.)
  `Alt+C` hides the message and shows it again; with nothing else to step back from (a draft, find, the whole
  task, the filter, the archive), `Esc` hides it too, before it does anything about closing the board. On the web
  board `Esc` does the same, while nothing is typed and find is closed.

**Only the words of the conversation are in the box**: what the Coordinator, the agent or the reviewer said, and
what was sent to it. Nothing of the program's own screen is: no tool call and none of its output, no spinner, timing
or token count, no tip, no input box, rule or footer, no start screen or logo. The board reads them from the
conversation the program itself saves (Claude Code, Codex and pi each keep one), not from its screen, so a new
version of a program that draws its screen differently changes nothing here. For herdr's other programs, whose
conversation the board does not read ([agents](agents.md)), the box shows the words on its screen instead. Markdown reads as text: a list item's
next lines stand under its words, code keeps its lines, and the marks of a headline, of bold and of a code block
are left out.

- **While it works and has said nothing new**, its last words stay, and the line above the box says what it is
  doing now. One that has said nothing at all yet says so in words: `The agent has just started: nothing to show
  yet.`
- **A dialog that is only on its screen** (a permission prompt, a question with choices) is said in a line of its
  own, with the way to it: `The agent shows a dialog on its screen that waits for your answer: enter opens it.` The
  Coordinator's is in herdr's `Coordinator` tab, a reviewer's in its `Reviewer #2` tab in the task's space.
- **A program whose conversation the board cannot read** (none of the three it starts today) is the one exception:
  the words on its screen show, under `Agent, on its screen now:`, with what is drawn there (rules, boxes, a logo,
  a bar) left out by its shape.
- **Going into the agent** (`Enter`) is its real terminal, untouched.

**While a task is under review, the chat is its reviewer's.** From the moment the review starts until it ends, the
box shows what the reviewer says (`Reviewer:`), kept fresh as it goes, for every program a reviewer can be. Before
it has said anything: `The reviewer has just started: nothing to show yet.` A reviewer takes no messages (it judges
the work as it is), so what you type still goes to the task's agent. The review goes on while you chat, as it does
for typing in the agent's own pane, and stops only when the agent changes the code
([chatting with a finished task](how-work-gets-done.md)). The empty line says both: `Send message to Agent: the review
goes on unless the code changes`. When the review
ends the chat is the agent's again, and the review's result is where it always is: on the row and in the task
panel.
- **One greyed hint line under the box**, with only the keys that do something right now. While you type, it says
  what the keys do with your words (`enter send to the Coordinator · tab send, and put an agent on what it adds ·
  shift+enter new line · esc put away`). What each hint says, and what gives way in a narrow window, is in
  [the hint line](keys.md#the-hint-line).

**The box holds one message: the newest, whoever said it.** What you just sent shows under `You:` the moment you send
it, and stays there until the answer comes; then the answer alone (`Agent:`, or `Coordinator:`), for the Coordinator
and for a task's agent alike. That holds also for words you send while the agent works: Claude Code queues them and
they go in during its turn, and the board reads them there. Your words are then one scroll up, with the rest of the
conversation.

```
╭────────────────────────────────────────────────────────────────────────────────────────────╮
│ Agent:                                                                                     │
│ Made g.                                                                                    │
│                                                                                            │
│ ❯ Send message to Agent                                                                    │
╰────────────────────────────────────────────────────────────────────────────────────────────╯
```

**The box scrolls through the conversation's words.** It opens on the newest thing said and holds only that. What
was said and sent before is above it: `Alt+K` opens the box to the conversation and scrolls up through it, `Alt+J`
down, half a box at a time (K and J, as up and down in vim), and `Alt+J` once more from its end closes it to the
newest message alone again, from its start. That holds wherever the box was opened, whoever you talk to, and also
after new messages came while it was open. The last dozen things said are there, each under the line that names who said it, as in the
Coordinator's chat: `You:` for your words (typed in the box, with `tell`, or in the agent's own pane), `Agent:` or
`Reviewer:` for what it said, `Coordinator:` for what the Coordinator sent it, and `Board:` for what the board sent
on its own (its brief, a review's findings, the word to go on). A long text sent, such as a brief, shows only its
beginning.
The mouse wheel does the same with the pointer over the chat ([the mouse](mouse.md)). Both work while you type, and
neither touches the draft. The box says where there is more and names the key: `alt+k more ↑` on its top edge,
`alt+j more ↓` at the right of the line under the message (in a narrow window only `more ↓`, then only the arrow).

```
 ● Coordinator  idle
╭───────────────────────────────────────────────────────────────────────────── alt+k more ↑ ─╮
│ 3. The check fails on main too: it is not this task's change.                              │
│ 4. I filed proj/7 for it and put an agent on it.                                           │
│                                                                               alt+j more ↓ │
│ ❯ Send message to Coordinator                                                              │
╰────────────────────────────────────────────────────────────────────────────────────────────╯
```

A new message starts at its beginning: an answer is read from its first line down, and one key goes on from there
(started at its end, you would scroll up to find where it begins and then down again to read it). Only a screen, shown
for a program whose conversation cannot be read, starts at its end, where a terminal has what is new.

Opened to the conversation, the chat stays open until you close it: whatever happens meanwhile (the Coordinator at
work, a new message, the list read again, tasks changing), it stays on the lines you were reading, found again where
they moved as the oldest messages drop off (also in a reply said twice: the one you read). Scrolled to its end, it
shows each new message as it comes. `Alt+K` goes further up with each press; it goes back to the newest message only
when you take it there: `Alt+J` once more from its end, hiding the chat (`Alt+C`, or esc) and showing it again, or
talking to another (the task's agent, another task's, back to the Coordinator).

A tall draft never takes the message away: in a window too small for both, the draft gives rows back and scrolls
inside what it keeps, so that three lines of the message stay, and they still scroll.

**The chat folds where the lines are needed elsewhere.** On a page that is read (the whole task, the settings page,
the `?` menu) and in a window of fewer than 14 rows the chat is its head line and the plain text box, as `Alt+C`
folds it, and is back when you return: the page, or the list, has those lines. The whole task says what its agent
said itself. A start that failed still says why.

**Typed words keep who they are for.** While the box holds text, the hint line under it says who gets it (`enter send
to HSW-34's agent`, `enter send to the Coordinator`, `enter add it as a task`), naming the task by its tag, also while
the list is scrolled to other tasks; the box's edge names no one. That does not change until the words are sent or put
away: the arrows are the draft's, the mouse wheel moves only the view of the list, and a click selects nothing. So an
answer typed for one task's agent can never be sent to another's.

**What you just sent takes the message's place.** Send something and the box shows your words, in the bright text colour
under `You:`, where the agent's message was, over an empty line to type on. The old message is never shown under them
(it is above, with the rest of the conversation). When the agent says something new, that takes their place, in grey
(also when it answers in the very same words as before). For the Coordinator this holds when the board is closed and
opened again; for a task's agent, for as long as the board stays open.

```
 ○ Coordinator  idle
──────────────────────────────────────────────────────────────────────────────────────────────
❯ Send message to Coordinator
──────────────────────────────────────────────────────────────────────────────────────────────
  alt+c show the latest message · ? all keys
```

One key shows and hides the message, the same key both ways and the same for the Coordinator and a task's agent:
`Alt+C` (on an empty text box, like `Alt+P` for the side panel); the hint line names it, and the board remembers
which. Hidden, there is only the line that says who you talk to and the line you type on, between two thin rules: a
message is never sent blind. Nothing else shows or hides it: `Esc` never does. A Coordinator that could not be
started says why even while the message is hidden; `Alt+C` puts that error away too, until the next one. The whole
chat of a task's agent is its own pane: `Enter`, with nothing typed, goes into it. Someone who has no message to show
(a task with no agent on it, the board with no Coordinator) gets the plain line between two rules, whether the chat is
hidden or not. The text box and its hints are never pushed off the window: the message gives way first. In a very
narrow window the box stays whole: the message wraps inside it, and a placeholder that does not fit is cut with `…`.

**Going into an agent never ends the board.** `Enter` on a task, with nothing typed, moves herdr to that agent's
real pane; it is the same from the row, with the chat hidden, and while the chat talks to that agent. The board keeps
running: come back to it, or press the key you gave the board, and it is as you left it, the same repo, the same task
selected, the same filter and draft. While you are away the selection stays on that task, also when its agent stops or
finishes and the task moves to another group or no longer matches what you searched for (its row stays until you move
on).

The key opens the board in a herdr tab of its own, named `board`, in the space you are in, and from then on goes to
that running board from anywhere instead of opening a second one. A board you started yourself in a pane
(`herdr-projects` typed there) is found the same way, and the key puts the keyboard on it also when an agent's pane
is split beside it in the same tab. Pressed on the board itself, the key takes you back to the pane you came from, so
it is a way there and a way back; when that pane is closed, or you started the board by hand, the board says on its
message line that it is the board already. Closing the board (`Ctrl+C` twice) closes its tab. Outside a herdr client
(plain SSH) the terminal becomes the agent's until you detach, and the board is back.

**The board as a popup.** `herdr-projects set board.open popup` makes the same key open the board as a popup, and
take it away again ([settings](settings.md)); `unset board.open` brings the tab back. It is herdr's own popup: a
framed window with the board's name, in the middle of herdr's screen, nine tenths of its width and height. It is no
pane and no tab and belongs to none: herdr draws it over whatever is there and changes nothing under it, so the
panes, their sizes and a zoomed pane are as they were when it closes. While it shows it has the keyboard and the
mouse, as herdr's own menu has: nothing behind it takes a key or a click.

- **The key again closes it**, and so does `Esc` once there is nothing left to step back from (a draft, the whole task,
  the filter, the archive and a chat message that shows come first, as on every board), and `Ctrl+C` twice. What you
  typed, the task you were on and the filter are there when it opens again.
- **`Enter` on a task closes the popup and lands in that task's agent.** The first three times it says for a moment
  how to get back.
- **It is showing or it is closed, never left behind.** Nothing in herdr's own window can switch tab or space under
  it. When herdr shows another tab all the same (a command moved it there, or a second window on the same session
  did), the popup closes at that moment, with what you typed saved; the key opens it there.
- **It opens on the task you were looking at**: pressed in the pane of a task's agent or reviewer, that task is
  selected, also when the filter left from last time would hide it; a view of one other machine only gives way to
  this machine's tasks, so the task shows and the chat talks to this machine. Anywhere else it opens where it was.
- **Words left in its box stay with their repo.** The popup opens in the repo of the pane you are in, and what you
  type is sent from there. With a draft waiting from last time it opens in the repo the draft was typed for and says
  so, so `Enter` never sends it from another one; once it is sent or put away, the popup opens where you are again.
  Words put away with `Esc` keep their repo too: `Ctrl+Z` in a popup that opened elsewhere takes the board to the
  repo they were typed for, and says so. When that folder is gone, the board stays where it is and says which repo
  the words would be sent from now.
- **A board in a tab keeps running beside it.** Each has its own note of where it is, and its own selection, filter
  and words (the popup's are in `view-popup.json`, and the first popup starts from the tab's view without its
  draft). There is one keeper, one Watcher and one Coordinator whichever board you look at, and the approve action
  for the focused pane is asked by the board that was shown for it.
- **When herdr will not open it**, the board opens in a tab and says why on its message line. herdr has one popup
  for everybody: while another plugin's, or a `type = "popup"` command of your own config, is open, the board's
  cannot open.

`popup` meant something else before: a pane in herdr's overlay, which filled the tab it was opened over and stayed
with that tab. That is gone, and `popup` is this one; there is no third value. A setting that says `popup` already
needs no change.

What still differs from herdr's own menu, and why. All of it is herdr 0.9.3 (and its newest build, 2026-10-12),
where a plugin has no other way to draw over the window ([what was tried](checked-on-herdr.md)):

- **herdr's own keys do not work while it shows.** herdr hands its popup every key, the prefix and the board key
  included, and starts no action. So the board reads its own key itself: it knows the key from herdr's `config.toml`
  (the prefix key, then the key) and closes on it. No other key of herdr's does anything in the popup; close it
  first. A prefix key pressed by itself does nothing, as in herdr.
- **The same in another popup**: while another plugin's popup shows, the board key is typed into that one. herdr
  starts no action, so no code of the board runs: it cannot open, and it cannot say why. Close that popup first.
- **It covers the panes, not herdr's sidebar and tab bar**: herdr centres its popup on the part of the window the
  panes have. Those two take no click while it shows.
- **herdr ties it to the tab it opened over** and hides it, still running, when another tab shows. The board closes
  it at that moment instead, so that is never seen; it is why the popup closes, and does not follow, when a command
  moves herdr elsewhere.
- **Nothing behind it is dimmed**, and its size is the board's choice, not herdr's menu's.

What herdr would need for the real thing is in [what was checked on herdr](checked-on-herdr.md), under "missing in
herdr".

**A board that looks cut off or half empty.** (What changes the board's size at all, and why a size that lasts a moment
is not drawn for: [the board's size](board-size.md).) A view somebody is looking through, the board open in the web UI's
terminal view, keeps the terminal, and the board's first line says so. From any other view the board takes its pane's
size back by itself within a few seconds: the remote view described here is disconnected, and until then the board draws
inside what herdr's window shows, so its text box and hints are never cut off. What follows is what you see until then,
and when the board could not take the size back. A remote view of the pane the board runs in (a `herdr terminal attach`,
which herdr's web UI starts for each pane it shows) sets that pane's size to its own and holds it for as long as it
runs, also after its browser tab is gone. herdr's own window then shows a cut-out of a board drawn for another size. The
board notices within ten seconds and says so on its first line, top left where a cut-out still shows it, with what frees
the size first so a narrow window keeps it:

```
 ! Shown cut off: a remote view holds this pane at 435×58 (herdr's own window: 184×78). ctrl+k ends that view
   (open 40m); closing it where it runs does the same.
```

It is a sentence, wrapped and never cut, and names no process to kill: `Ctrl+K` ends that view from the board (the key
is the board's only while such a view holds the pane), and closing it where it runs gives the pane its own size back
at once too. The line is not shown
outside herdr, in a tmux inside the pane, or in herdr's popup (which has no pane of its own).

**The board's tab is named `herdr-projects`.** Run `herdr-projects` in a terminal inside herdr and the tab that
terminal is in gets that name for as long as the board is open, so you can tell it from your other tabs. Only that one
tab is renamed. When the board closes (also when it closes to take you into an agent) the tab gets back the name it had
before; a name you gave it yourself while the board was open is kept. A board that was killed leaves the name, and the
next board that opens and closes in that tab puts the old one back. Nothing is renamed by the popup (it is no tab and
has none of its own), in the tab your board key opens (that one is named `board`, above), outside herdr, in a tmux
inside the pane, or by a subcommand that opens no board (`add`, `list`, `show` and the rest). The code is
`name_board_tab` in `crates/board/src/board/run/place.rs`.

While the text box holds text it owns the keyboard: only `Enter`, `Tab`, `Esc` and `Ctrl+C` act on the board, so no
shortcut fires in the middle of a draft. `Esc` puts a draft away; `Ctrl+Z` brings it back only to whoever it was for, so
a reply stays a reply. It never changes who you talk to: when the chat names somebody else, it says who the words were
for (the Coordinator: `Space` goes back to it; a task's agent: select that task, and `Space` if you talk to the
Coordinator), and the words stay put away until you are there.
