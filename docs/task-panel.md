# The task panel

**The task panel shows everything about the selected task.** `Alt+P` opens and closes it beside the list (in a window of
90 columns or more), and your choice is remembered. `Alt+T` shows the whole task full screen; `Esc` or `Alt+T` leaves it.
`Ctrl+E` opens the task's settings.

Closed, each row has the whole width for what it says. Opening or folding the panel never changes who you talk to or
whether the chat's message shows, and `Space` never opens or folds it, at any width.

```
 HSW-84 Max length for task titles

 Status ─────────────────────────────────────────────
 State        ◆ ready to approve · 4th review · 2m ago
 On it        claude
 Change       PR#60 · 19 files, +340 −27

 Description ────────────────────────────────────────
 A task title should have a maximum length.

 Reviews · 4 ────────────────────────────────────────
 History      fail → no verdict → fail → pass
 Review 4     ✓ Pass · 1 minor · 11m ago
              The change meets the task. A title at
              the limit is kept whole.
 Evidence     ↗ Open review

 The agent said ─────────────────────────────────────
 Titles longer than 80 characters are cut between words.

 Settings ───────────────────────────────────────────
 Agent        claude · opus * · default effort
 Plan first   off
 Reviewer     codex · default model · default effort
 Review when  in a repo · with evidence
 Send back    major or worse · 5 rounds max
 Check        npm test · 15m at most
 Merge        after your yes · squash · opens a pull request
              waits for GitHub's checks
 Workspace    from main · removed when done
 Also         one attempt · not research only · no limits
              no fallback
 * this task only
```

## One form for every part

The panel and the full screen have the same parts, in the same order:

- **A headline** in bold, on a thin rule that runs to the edge. One that needs you is in its state's colour.
- **Facts are a label and its value**: the label greyed on the left, the value on the right. Every value starts in the
  same column, so the eye runs down one line. A long value wraps under itself, between its facts where it has them
  (`12m ago` is never split).
- **A text** (the description, what the agent said) is plain lines under its headline.
- **An empty line between two parts**, and between two reviews on the full screen. Nowhere else.
- **Nothing is boxed, nothing sits behind a bar, and no fact is cut with `…`.** Where the panel is too short, whole lines
  and facts give way ([below](#a-short-window)), and the last line says where the rest is.

## The parts, top to bottom

- **The title**, on top, by itself. The task is named as its row names it (`HSW-84`), in the title and in the heading of
  the whole-task page (`Task details  HSW-84 "Max length for task titles"`). Its id, as a command takes it, stands once,
  on the whole task, with the session and its repo's key (`Task id  herdr-projects/84 · HSW`).
- **Status**: where it stands, one fact a row, always in this order:

  | Row | Says |
  |---|---|
  | `State` | its mark and word, which review it is on, since when |
  | `Progress` | the bar, its percent and the minutes left |
  | `Now` | what the agent or reviewer is doing right now, wrapped whole |
  | `On it` | who |
  | `Took over` | each hand-over to [a fallback agent](fallback.md), in plain words |
  | `Made by` | the [automation](automations.md) that made it, if one did |
  | `Change` | the pull request, in the colour of its checks, and the size of the change. The number is a link; full screen, the address is printed too |
  | `Next` | for a task nobody is on: what puts an agent on it |

- **What it needs from you**: a part of its own right under Status, its headline in red: `Asks you`,
  `Asks for your OK`, `Needs a login`, `Review failed` (what the review found, and what the two keys do: send it back,
  or merge anyway), or `Went wrong` with what failed, who can fix it, the `Next step` and the `Details`. A headline is
  never repeated as the first line under it. `Behind main` and `Cut off` (the board tries again by itself) are the same
  part in their own colour, and an archived task says so there.
- **The description**: whole on the full screen, cut after about 400 characters beside the list. If the panel is too
  short for it and the settings, it is cut further, then becomes one line (`Description · alt+t shows it`), then goes:
  all of that before a single setting does.
- **Attempts**, for a task done several times at once: the reviewer's comparison, then each attempt
  ([several attempts](attempts.md)). While the task waits for your pick, it is headed `Pick one of 3`, in red, and is
  what it needs from you.
- **The reviews**, as a history ([below](#the-reviews)).
- **What the agent said**: the newest words of its conversation, never its program's screen (while you talk to its
  agent, that is in the chat instead). During a review it is the reviewer's, and the headline says `The reviewer says`.
  When the review ends it is the agent's again. A dialog that is only on a program's screen is mentioned in a line, with
  the way to it. What the chat shows, and from where: [the board](board.md).
- **Settings**: every setting the task runs with ([below](#the-settings-part)).
- **Session**, full screen only: the agent's context, its tokens, and where it runs in Herdr.

Full screen, the text keeps to 100 columns. Beside the list, when anything is shorter than on the full screen, the last
line says `alt+t shows the whole task`.

## The settings part

Every setting the task runs with, and its value, a few to a row, in plain words. Never a line that says they are the
usual ones.

| Row | Says | Settings |
|---|---|---|
| `Agent` | who works: the program (and its version), its model, its effort; `fast`, `asks before acting` and `words of your own` where they are so | `worker.*` |
| `Takes over` | who takes over when the agent, or the reviewer, cannot go on, under that one's row. Once one of the two has a list both have the row, and the one without says `nobody` | `worker.fallback`, `reviewer.fallback` |
| `Plan first` | `off`, or who approves the plan and who writes it (a planner somebody chose is shown while it is off too) | `plan.approve`, `plan.agent` |
| `Attempts` | how many and by whom, and who picks; only for more than one | `attempts.by`, `attempts.pick` |
| `Reviewer` | who reviews, as `Agent`; `off` where nothing is reviewed, with what somebody chose for it all the same | `reviewer.*` |
| `Review when` | `in a repo`, `always` or `never`, and whether the reviewer brings evidence | `review.when`, `review.evidence` |
| `Send back` | how bad a finding sends the work back, and how many rounds go by themselves | `review.mustFix`, `review.rounds` |
| `Check` | the check's command and how long it may run | `check.command`, `check.timeout` |
| `Merge` | after your yes or by itself, how it lands, and whether a pull request is opened. A research-only task has `Research` here instead: it merges nothing. A [chat-only](chat.md) one has `Chat` (`no change to the repo · ends when you say`), and no `Reviewer`, `Review when`, `Send back` or `Check` rows: none of them runs | `merge.by`, `merge.method`, `merge.pr`, `research.only`, `chat.only` |
| `Limits` | the longest run and the most tokens, both of them (`2h a run · no token limit`); only where one is set, or somebody said `none` | `limit.time`, `limit.tokens` |
| `Workspace` | what the worktree starts from, what sets it up, is copied in and tears it down, and whether it is removed when done | `workspace.*` |
| `Also` | in a word each, what has no row of its own while it is nothing: `one attempt`, `not research only`, `no limits`, `no fallback` | |

- **Only a value set for this task alone is marked**: a `*` after it, in the accent colour, and one dim line under the
  rows, `* this task only` (the first line to go where room is short). Every other value looks the same, whether it is
  your settings, the repo's or built in. The panel never says where a value comes from: `herdr-projects show <id>` does
  ([settings](settings.md)).
- Every setting somebody chose is shown, also one that says `none`, or one that does nothing now.
- A task in no repo has no `Check` and no `Workspace` row.
- The whole task adds where the task is (`Project`, `Worktree` with its folder, `Branch`). Beside the list, only a project
  that is not this board's, and another machine's name (`Machine`), have a row.
- A `Note` row says what the board could not use: a program that is not installed, something the repo's own file was
  refused, a mistake in your settings file.
- A task on another machine has the same rows, from the settings that machine sent.
- The web Board's task panel has them too, right under `Status` and what the task needs from you, above the description,
  the reviews and what the agent said: its page scrolls, so what is on top is what has the room
  ([the board in the web UI](web-ui.md)).

## The settings page

`Ctrl+E` on a task opens its settings full screen: every work [setting](settings.md) by its plain label, its value, and
its name as a command takes it, with `*` on the ones set for this task only. Where any other value comes from is not said
here either: `herdr-projects show <id>` says it.

```
 Settings  HSW-12 "Tidy the first-run questions"
 ────────────────────────────────────────────────────────────────────────────────
 Send back from: critical · [major] · minor · trivial

     Agent's program       claude    worker.agent
   * Agent's effort        high      worker.effort
     …
▌›   Send back from        major     review.mustFix
     Rounds by itself      5         review.rounds
     Merged by             itself    merge.by
     Check command         npm test  check.command

 * this task only
```

| Key | Does |
|---|---|
| `↑` `↓` | move to a row (here they do not move between tasks) |
| `Enter` | change the row, for this task only (below) |
| `Ctrl+U` | put the selected row back to what it would be without this task's own answer |
| `Esc`, or `Ctrl+E` again | back to the task |

- **`Enter`**: a switch or a choice goes to its next value and is saved at once. Anything else is typed in the box at the
  bottom (it starts empty; the page shows the current value) and saved with `Enter` (`Esc` leaves it as it was). A
  refused value says why, with the nearest right one, and changes nothing.
- **The line on top** is the selected row's: what it may be. A choice lists its words with the current one marked; a
  switch reads `on · [off]`; anything typed says what to type (`a time, such as 90s or 15m`).
- `*` marks the rows set for this task only. Every other row is plain, wherever its value comes from. A row that does
  nothing for this task, or counts only from its next start, says so before its name.
- **A setting that cannot apply is not offered**: fast mode, for a program that has none (only Codex has one); a repo's
  setting, for a task in no repo. One that is this task's own all the same still shows, so it can be put back.
- Only this page's keys act while it is open, and the hint line is its own ([keys](keys.md)).
- Board settings are yours only and are not on this page: they are on yours (below). A task on another machine is
  changed there. An archived task's settings can be read, not changed.

### Your own settings

`Ctrl+E` on this machine's heading (or a status group's), or `your own settings` in the `?` menu, opens the same page
for your settings: what every task starts from, board settings too (`Settings  yours: what every task starts from`).
On a repo's heading it is your entry for that repo. The keys are the page's; `Enter` saves as `herdr-projects set` does
and `Ctrl+U` as `unset`. `*` marks what you set, and every other row says where its value comes from (`built in`, or on
a repo's entry `you`). `Esc` goes back to the list. On another machine's heading the board says to change them there
([settings](settings.md)).

## A coordinator in the panel

On a coordinator's row (a machine's or a project's row) the panel shows that
coordinator, in the same form:

- **Status**: `State` (its chat's, `idle` or `working`; on [autopilot](autopilot.md), autopilot's word), `Last`, its
  latest message as the chat shows it, and while on, `Goal` and `Today` (`2 of 5 added · 1 of 2 under way`).
- **Settings**: `Agent` (its program, model and effort), and while on, `Wakes`, `Limits`, `May set`, `May touch` and the
  rest, in plain words. `*` marks this time's values, with `* this time only` under them.
- While autopilot is off, nothing of it shows.
- A `coordinator.*` value changed while its chat runs says `from its next fresh chat`, here and on its settings page.
- `Alt+T` shows it whole, and while on its last 20 wakes: when, why, and whether it answered.

`Ctrl+E` there opens its settings page, in this page's form and keys: its own rows first, under `This coordinator`,
then on this machine's row your own settings, on a project's row your entry for that repo. While autopilot is on,
`Enter` on an autopilot row other than the goal saves it for this time only, and a line under the rows says so. Another
machine's coordinator's page is shown whole, to read; it is changed on that machine.

## The reviews

**Every review is kept, and they read as a history.**

- `History` says how each went, oldest first (`fail → no verdict → pass`).
- A review is a row under its own name: a mark and its result (`✓ Pass · 2 minor`, `✗ Fail · 1 critical`,
  `– No verdict`; the colour only repeats the mark), when, and under them its summary.
- **Beside the list**, the latest one shows, with the first piece of `Evidence` (its name, wrapped whole). Whenever the
  full screen has more of a review (a longer summary, findings, what it checked, more evidence, or its address), the last
  line says so.
- **Full screen** has all of them, newest first, each with its commit and, as facts of their own:
  - its `Findings` (how bad each is, in a column of its own: `critical`, `major`, `minor`; one that did not hold the work
    back is marked `not required` or `pre-existing`);
  - what it `Checked`, and what it left `Unverified`;
  - each piece of **evidence** under its own `Evidence` headline: its name, what it is, and its address printed in full,
    each wrapped under itself and never cut.
- A review that gave no verdict has its row too, with why.
- A review from before the gate keeps its score and advice as they were (`4/5, says merge`), and its `Problems`.
- A review that still stood after main was merged in says so under its result.

`herdr-projects details <id>` prints the same list as `reviews`.

## A narrow panel

In a panel too narrow for a value beside its label (under about 30 columns), the label is a line of its own and its value
is under it, indented. Nothing is cut there either.

## A short window

Whole lines give way, the least needed first. The state and the `Now` text stay whole, and **the settings outlast every
text**: they are shown in full whenever they can be.

1. The empty line on top. Then what the agent said and the review give up lines (the review its summary, then `History`
   and `Evidence`), each down to a few lines. A text that lost lines ends in ` …`.
2. What the agent said, the attempts, the plan and the report go. The review keeps its one row
   (`Review 3  ✓ Pass · 2 minor`).
3. The description gives up lines, down to one that says where to read it (`Description · alt+t shows it`). What the task
   needs from you is down to its first two lines. Then that one line of the description goes.
4. Only now the settings. First the rows that are not settings (`Project`, `Machine`, `Note`). Then one row at a time,
   each whole. The ones that stay longest: a row with a value somebody chose (one that names its place), then `Agent`,
   `Reviewer`, `Review when`, `Send back` and `Merge`, then the rest. The last line under them says how many settings
   have no room and the key that shows them (`13 more settings · ctrl+e shows them`).
5. With no room for a single setting, their headline goes too, and the other parts get their lines back: they give way as
   in 1 and 2, the description last of the texts.
6. The facts said elsewhere (`Task`, `On it`, `Made by`), the review down to its one row, the empty lines between the
   parts, the `Status` headline, the title's second line; then `Change`, the review's row and the title.

What is left then is the state, the progress and the `Now` text. With no room for those and the last line
(`alt+t shows the whole task`), that line goes, then the progress, then the state: the text itself is the last to stay.
Only a text longer than the whole panel is cut, and its last line ends in `…`.

## A part that fails to draw takes only itself

Its headline stays, with one red line under it (`✗ This part could not be shown: …` and the error). Every other part of
the panel is there, and every key works. The same holds for each section of the screen: the list, the panel, the task
details, the menu, the chat, the text box, the hints. One that fails is that red line in its place, and the others stay.
Only a failure outside all of these replaces the board with `view error`.

## Why it looks like this

The panel shows one issue and the history that comes in under it, which is what an issue or pull request page shows. The
form was taken from what already works for that, and fitted to a terminal's text cells:

| From | Taken | Left out, and why |
|---|---|---|
| GitHub's pull request page | A status first, then the checks and reviews summed up as rows of mark, name and result; what blocks the merge named right under the status; the timeline newest-relevant first. | Avatars, badges as coloured pills, a column on the side: no room and no pixels. |
| `gh pr view`, `gh pr status`, `gh pr checks` | Label-and-value rows for the facts; one line per run with its mark, result and age; a result said in words with the colour only repeating it. | Its dot-separated one-liner (`Checks passing - Approved`): that is what the old top of the panel was. |
| lazygit, gh-dash | One accent per state, quiet labels, the selected thing's detail beside the list and whole on its own screen. | Boxes around every pane and tabs inside the detail: a box costs two columns and two lines of a 38-column panel. |
| CodeRabbit, Greptile, Cursor Bugbot | Findings by severity in a column of their own; a verdict line before the detail; what changed since the last round as a history line. | Emoji badges, a score out of five for new reviews (the gate's pass or fail says more), collapsed sections. |
| The `/ui` skill's guidelines | Labels quieter than values in a list of facts; the lightest separation that works (space, then a rule, never a card by default); numbers that line up. | Everything about pixels, radii and shadows. |
| `tui-design` skills on skills.sh | Stable places for the same fact in every state; a mark or a word beside every colour; narrow, ordinary and wide widths each tried; give way by priority instead of cutting. | Borders for emphasis, and their palettes: the board keeps Herdr's. |
