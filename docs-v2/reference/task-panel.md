# The task panel

This page tells what the task panel, the whole task and the settings page show. The task panel shows all the data
about the selected task.

## Open and close the panel

| Key | Does |
|---|---|
| `Alt+P` | Opens and closes the panel beside the list. This works in a window of 90 columns or more. The board remembers your choice. |
| `Alt+T` | Shows the whole task, full screen. `Esc` or `Alt+T` closes it. |
| `Ctrl+E` | Opens the settings page of the task. |

When the panel is closed, each row of the list has the full width for its text. When you open or fold the panel, the
person or agent that you talk to does not change. The chat message also stays shown or hidden. `Space` never opens or
folds the panel, at any width.

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
 Reviewer     codex · default model · default effort · fast
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

## The form of each part

The panel and the whole task have the same parts, in the same order. Each part has the same form:

- **A headline** in bold, on a thin rule that goes to the edge. A headline that needs you is in the colour of its
  state.
- **Facts** are a label and its value. The label is grey, on the left. The value is on the right. All values start in
  the same column, so that your eye can go down one line. A long value wraps under itself, between its facts where it
  has them. A fact such as `12m ago` is never split.
- **A text** (the description, or what the agent said) is plain lines under its headline.
- **An empty line** is between two parts, and between two reviews on the full screen. There are no other empty lines.
- **No box, no bar behind text, and no fact cut with `…`.** If the panel is too short, full lines and facts go away,
  and the last line tells where the rest is ([a short window](../explanation/task-panel.md#a-short-window)).

## The parts, from top to bottom

- **The title** is on top, alone. It names the task as its row does (`HSW-84`). The heading of the whole task names it
  the same way: `Task details  HSW-84 "Max length for task titles"`. The id of the task, in the form that a command
  takes, shows one time, on the whole task. It shows with the session and the repo key:
  `Task id  herdr-projects/84 · HSW`.
- **Status** tells the condition of the task, with one fact on each row, always in this order:

  | Row | Says |
  |---|---|
  | `State` | its mark and word, which review it is on, since when |
  | `Progress` | the bar, its percent and the minutes left |
  | `Now` | what the agent or reviewer is doing right now, wrapped whole |
  | `On it` | who |
  | `Took over` | each hand-over to [a fallback agent](../explanation/fallback.md), in plain words |
  | `Made by` | the [automation](automations.md) that made it, if one did |
  | `Change` | the pull request, in the colour of its checks, and the size of the change. The number is a link; full screen, the address is printed too |
  | `Next` | for a task nobody is on: what puts an agent on it |

- **What the task needs from you** is a part of its own, immediately under Status. Its headline is red: `Asks you`,
  `Asks for your OK`, `Needs a login`, `Review failed`, or `Went wrong`.
  - `Review failed` tells what the review found, and what the two keys do: send it back, or merge anyway.
  - `Went wrong` tells what failed, who can repair it, the `Next step` and the `Details`.
  - The first line under a headline never repeats the headline.
  - `Behind main` and `Cut off` are the same part, in their own colour. For `Cut off`, the board tries again
    automatically.
  - An archived task tells that it is archived in this part.
- **The description** shows in full on the full screen. Beside the list, the board cuts it after approximately 400
  characters. If the panel is too short for the description and the settings, the board cuts the description more.
  Then it becomes one line (`Description · alt+t shows it`). Then it goes. All of this occurs before one setting goes.
- **Attempts**, for a task that is done several times at the same time: the comparison of the reviewer, then each
  attempt ([several attempts](../explanation/attempts.md)). While the task waits for you to pick one, its headline is
  `Pick one of 3`, in red. Then it is what the task needs from you.
- **The reviews** show as a history ([the reviews](#the-reviews)).
- **What the agent said** shows the newest words of its conversation. It never shows the screen of its program. While
  you talk to its agent, these words are in the chat instead. During a review, these are the words of the reviewer,
  and the headline is `The reviewer says`. When the review ends, they are the words of the agent again. If a dialog is
  only on the screen of a program, one line tells about it and how to go to it. What the chat shows, and from where:
  [the board](board-header-and-list.md).
- **Settings** shows every setting that the task runs with ([the settings part](#the-settings-part)).
- **Session** shows only on the full screen: the context of the agent, its tokens, and where it runs in Herdr.

On the full screen, the text uses a maximum of 100 columns. Beside the list, if a part is shorter than on the full
screen, the last line says `alt+t shows the whole task`.

## The settings part

The settings part shows every setting that the task runs with, and its value. It puts a few settings on each row, in
plain words. It never shows a line that says that the settings are the usual ones.

| Row | Says | Settings |
|---|---|---|
| `Agent` | who works: the program (and its version), its model, its effort; `fast`, `asks before acting` and `words of your own` where they are so | `worker.*` |
| `Takes over` | who takes over when the agent, or the reviewer, cannot go on, under that one's row. Once one of the two has a list both have the row, and the one without says `nobody` | `worker.fallback`, `reviewer.fallback` |
| `Plan first` | `off`, or who approves the plan and who writes it (a planner somebody chose is shown while it is off too) | `plan.approve`, `plan.agent` |
| `Attempts` | how many and by whom, and who picks; only for more than one | `attempts.by`, `attempts.pick` |
| `Reviewer` | who reviews, as `Agent`; `off` where nothing is reviewed, with what somebody chose for it all the same | `reviewer.*` |
| `Review when` | `in a repo`, `always` or `never`, and whether the reviewer brings evidence | `review.when`, `review.evidence` |
| `Send back` | how bad a finding sends the work back, and how many rounds go by themselves | `review.mustFix`, `review.rounds` |
| `Check` | the check's command and how long it can run | `check.command`, `check.timeout` |
| `Merge` | after your yes or by itself, how it lands, and whether a pull request is opened. A research-only task has `Research` here instead: it merges nothing | `merge.by`, `merge.method`, `merge.pr`, `research.only` |
| `Limits` | the longest run and the most tokens, both of them (`2h a run · no token limit`); only where one is set, or somebody said `none` | `limit.time`, `limit.tokens` |
| `Workspace` | what the worktree starts from, what sets it up, is copied in and tears it down, and whether it is removed when done | `workspace.*` |
| `Also` | in a word each, what has no row of its own while it is nothing: `one attempt`, `not research only`, `no limits`, `no fallback` | |

- **The panel marks only a value that is set for this task alone.** The mark is a `*` after the value, in the accent
  colour. One dim line under the rows says `* this task only`. This line is the first line to go when the space is
  small. All other values look the same: your settings, the settings of the repo, and the built-in values. The panel
  never tells where a value comes from. `herdr-projects show <id>` tells it ([settings](settings.md)).
- The panel shows every setting that somebody chose. This includes a setting that says `none`, and a setting that has
  no effect now.
- A task in no repo has no `Check` row and no `Workspace` row.
- The whole task adds where the task is: `Project`, `Worktree` with its folder, and `Branch`. Beside the list, only a
  project that is not the project of this board, and the name of another machine (`Machine`), have a row.
- A `Note` row tells what the board could not use. Examples: a program that is not installed, a setting that the board
  refused in the repo's own file, a mistake in your settings file.
- A task on another machine has the same rows, from the settings that the other machine sent.
- The task panel of the web board has these rows too. They are immediately under `Status` and what the task needs from
  you. They are above the description, the reviews and what the agent said. The page of the web board scrolls, so the
  top of the page has the most space ([the web UI](web-ui.md)).

## The settings page

`Ctrl+E` on a task opens its settings, full screen. The page shows every work [setting](settings.md) with its plain
label, its value, and its name as a command takes it. `*` marks the settings that are set for this task only. The page
does not tell where a different value comes from. `herdr-projects show <id>` tells it.

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
| `↑` `↓` | Moves to a row. On this page, these keys do not move between tasks. |
| `Enter` | Changes the row, for this task only. |
| `Ctrl+U` | Puts the selected row back to the value that it has without the answer of this task. |
| `Esc`, or `Ctrl+E` again | Goes back to the task. |

- **`Enter`** moves a switch or a choice to its next value and saves it immediately. For all other rows, you type the
  value in the box at the bottom. The box starts empty, and the page shows the current value. `Enter` saves the value.
  `Esc` keeps the value as it was.
- If the board refuses a value, it tells why, with the nearest correct value. Nothing changes.
- **The line on top** belongs to the selected row. It tells what the row can be. A choice lists its words and marks the
  current one. A switch shows `on · [off]`. A typed value tells what to type (`a time, such as 90s or 15m`).
- `*` marks the rows that are set for this task only. All other rows are plain, wherever their value comes from. If a
  row has no effect for this task, or has an effect only from the next start, the row tells so before its name.
- **The page does not offer a setting that cannot apply.** Examples: fast mode for a program that has no fast mode
  (only Codex has one), and a repo setting for a task in no repo. If such a setting is set for this task all the same,
  it shows, so that you can put it back.
- While the page is open, only its own keys act. It has its own hint line ([keys](keys.md#keys-on-the-settings-page)).
- Board settings are for you only. They are not on this page (`herdr-projects show`).
- You change a task on another machine on that machine.
- You can read the settings of an archived task, but you cannot change them.

## The reviews

**The board keeps every review, and the reviews show as a history.**

- `History` tells the result of each review, oldest first (`fail → no verdict → pass`).
- Each review is a row under its own name. The row shows a mark and its result (`✓ Pass · 2 minor`,
  `✗ Fail · 1 critical`, `– No verdict`) and the time. The colour only repeats the mark. The summary is under the row.
- **Beside the list**, the latest review shows, with the first piece of `Evidence` (its name, wrapped in full). If the
  full screen has more of a review, the last line tells so. "More" is a longer summary, findings, what the review
  checked, more evidence, or its address.
- **The full screen** shows all reviews, newest first. Each review shows its commit, and these facts:
  - its `Findings`, with the severity of each in a column of its own: `critical`, `major`, `minor`. A finding that did
    not stop the work is marked `not required` or `pre-existing`.
  - what the review `Checked`, and what it left `Unverified`.
  - each piece of evidence under its own `Evidence` headline: its name, what it is, and its full address. Each wraps
    under itself and is never cut.
- A review that gave no verdict also has its row, with the reason.
- A review from before the gate keeps its score and advice as they were (`4/5, says merge`), and its `Problems`.
- A review that stayed valid after main was merged in tells so under its result.

`herdr-projects details <id>` prints the same list as `reviews`.

## A narrow panel

If the panel is too narrow for a value beside its label (less than approximately 30 columns), the label is on a line
of its own. Its value is under it, indented. Nothing is cut there either.

## Related pages

- [How the task panel is made](../explanation/task-panel.md)
- [Settings](settings.md)
- [Change a setting](../how-to/change-a-setting.md)
- [Keys](keys.md)
- [The gate](../explanation/gate.md)
- [The web UI](web-ui.md)
