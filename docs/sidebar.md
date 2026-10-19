# Herdr's sidebar

**Herdr's sidebar shows a small copy of each live agent's row on the board**, in the same words: the task's number and
title, who is on it and where it stands, and the percent and time to go. Not the row's second line, what the agent says
it does (`Reading the list code`, the question itself): that stays on the board, and row 2 already says where it stands.

There is one entry per live agent: a task's agent, its reviewer while a review runs, and the Coordinator. A task nobody
is on (to do, done and cleaned up) has no pane in Herdr, so no entry.

**A done task leaves the sidebar by itself**: once it is merged (here or on GitHub), finished with nothing to merge, or
called done by you, its whole space closes. That is the setting `workspace.cleanup`, on unless you turned it off. A
space that stays has a reason, and the task's line says it (`Kept: … (uncommitted changes)`): see
[when a task is done](how-work-gets-done.md#when-a-task-is-done) and
[a done task stays in the sidebar](troubleshooting.md#a-done-task-stays-in-herdrs-sidebar).

To get all three rows, let the board add its sidebar rows to Herdr's config on its first screen ([installing](install.md)).

## Mission Control

Mission Control nutzt ausschließlich die native Agentenliste für offene Agenten-Tasks. Die erste Zeile nennt Task-ID und Titel statt des gemeinsamen Arbeitsbereichs. Der aktuelle Orchestrator heißt `Orchestrator (<CLI>)`. Projektwahl und Eingabe bleiben in Mission Control; es gibt dort keine zweite Taskliste und keine zweite Ausgabeansicht.

Ein gestoppter Task mit erhaltenem Terminal bleibt als wartender Eintrag sichtbar. Der dafür gemeldete Agent `herdr-task` ist ausschließlich ein Anzeigeeintrag: die internen Agentenabfragen filtern ihn aus, und vor einem echten Worker-Start wird seine Anzeigeautorität freigegeben. Backlog-Tasks ohne Terminal bleiben im klassischen Projects-Board.

Nach einem Providerwechsel wird die Ausgabe eines früheren Orchestrators unter `coordinator/<session>/handover-*.terminal.txt` im Datenordner gesichert. Sein Pane schließt erst, wenn er ruht und der neue Orchestrator läuft. Task-Panes und fremde Terminals werden dabei nicht geschlossen.

## The three rows of an entry

```
 ◐ HSW-152 Show automations in the m…      1. Herdr's dot, the task's number and title
   agent · working · R#1 PR#113 · 6m       2. who is on it, where it stands, review round, pull request, tags, time
   8% 16m                                  3. the percent and the time to go
```

### Row 1: Herdr's dot, the number and the title

As on the board: `HSW-152 …`, the repo's key and its number ([task numbers](task-numbers.md)). The number comes first
and Herdr cuts a row at its end, so a narrow sidebar cuts the title, never the number. The machine's name stands before
it only where the sidebar shows more than one machine: that is Herdr's own `machine` token.

### Row 2: who is on it and where it stands

From the one list of status words (`crates/core/src/status_words.rs`):

| The row on the board | Row 2 of the entry |
|---|---|
| `◐` the agent works | `agent · working` |
| `◑` under review | `reviewer · under review` on the reviewer's entry, `agent · under review` on the agent's |
| `◇` the board starts it, sets up, checks, pushes, merges, or waits on the way (its check's turn, its turn to merge, GitHub's checks) | `board · starting`, `board · setting up`, `board · checking`, `board · pushing`, `board · merging` |
| `◌` waits for a free slot, `Q#2` | `queued`, and row 3 says `Q#2`: its place in the queue, where a working task has its percent |
| `◌` queued after other tasks ([one task after another](after.md)) | `queued after SKL-6` |
| `?` asks you | `asks you` |
| `!` asks for your OK | `asks for your OK` |
| the sign-in mark | `needs a login` |
| `◆` passed, waits for your yes | `ready to approve` |
| `!` its review failed | `review failed` |
| `×` went wrong (a failure, a merge or a pull request that failed) | `went wrong` |
| finished, but its branch cannot be brought up to date | `behind main` |
| waits for you, with no kind (a reviewer's dialog, stopped without a summary) | `needs you` |
| stopped, put back | `to do` |
| done, while its space is still open | `done` |

- Somebody is named only while somebody is on it. A waiting task says what it waits for.
- Then the review round and pull request as the row writes them (`R#1 PR#113`), the task's tags (`[research]`,
  `[auto]`), and last the row's time: `6m` for how long it has worked, `2m ago` for how long it has waited.
- The time is by the minute (`<1m` before the first): Herdr shows what it was last told, and the board does not write
  every second. It stands last, so a narrow sidebar cuts it first.

### Row 3: the numbers, bare

The percent (or the place in the queue) and the time to go, where the row shows them: `8% 16m`, `20h` from a thousand
minutes on. Never the word `left`: the sidebar is narrow, and a number after a percent can only be the time to go. Empty
when the row has no numbers.

### The reviewer, the Coordinator and the spaces

- **The reviewer's entry** names the task the same way, with `reviewer` as who. During a review the task has two
  entries, each for its own pane: `reviewer · under review` is the reviewer's, `agent · under review` the agent's whose
  work is reviewed. The rest of their words are the same, and each has its own dot.
- **The Coordinator** is named `Coordinator` and shows what the line above the board's box says: `Coordinator · idle`,
  `Coordinator · working`, `Coordinator · asks you`, with what waits to go in (`· 1 message waits to be sent`) and its
  latest message under it. Herdr's own words for it would be `sw-coordinator` and `done`.
- **The spaces part** follows the same rule: a task's space is named with the task's number and title
  (`HSW-152 Show automations…`), with the row's percent and time to go under it. A task waiting for you to pick one of
  its attempts has none: its attempts' entries say `pick one`.
- **Only the task's own pane and space.** herdr hands the id of a closed pane or a removed space out again (after it
  restarts, or once you removed a done task's space yourself), but never a terminal's. A pane under the task's id in a
  terminal the task was never seen in is not the task's, even in its own folder: the board names nothing there and
  takes nothing off, and the task's row says its agent is gone. Its space is named only while its pane is in it, and a
  coordinator's pane or space is never a task's, whatever the task's old link says. After herdr restarts, the pane is
  the task's again once herdr shows the task's agent running in its new terminal.

## With and without the plugin's sidebar rows

Herdr's default sidebar shows a space's name, the tab's name and the agent's shown name, and no custom value. So:

| | With the rows ([installing](install.md)) | Herdr's default rows |
|---|---|---|
| Row 1 | `HSW-152 Show automations…` | `HSW-152 Show automations… · Agent`: the tab's name too |
| Row 2 | `agent · working · R#1 PR#113` | the same |
| Row 3 | `8% 16m` | not shown: Herdr's default rows show no custom value |
| A space | its name, its branch, the percent and time to go | its name and its branch |

The plugin's rows hide a task's own tab label (`Agent`): it says nothing the next row does not. Reviewers appear as panes in that tab.
A tab of your own named exactly `Agent` is hidden on row 1 too. An agent that is not the board's keeps Herdr's own
entry: its space, its tab, its name.

## What is Herdr's own

- **The dot is Herdr's**, in Herdr's colours: its own reading of the pane (working, idle, done, blocked). The board writes
  no state to Herdr. Telling Herdr a task is blocked would make Herdr stop watching the pane
  ([checked on Herdr](checked-on-herdr.md)). So a task that asks you has Herdr's done or idle dot, and the words beside
  it say exactly where it stands.
- **The cutting is Herdr's.** Herdr shares a row's width between its values and ends a cut one with `…`. At its default
  width of 26 cells a long row is cut; `sidebar_width = 36` under `[ui]` in Herdr's config shows more. Herdr keeps at
  most 80 characters of any value: the board cuts a longer line itself, with `…`.

## What Herdr 0.9.3 cannot do here

- **The Coordinator's first row reads `~ · Coordinator`**, not `Coordinator` alone. It lives in a tab of the home space,
  and a sidebar row can hide a value only by the value itself: hiding `~` would hide it for every agent of yours in the
  home space.
- **No marks of the board's own.** The row's `?`, `!`, `◑`, `◇`, `◌` and `◆` cannot be drawn: an entry's icon is Herdr's
  state icon, which only Herdr's own five states have. The words carry it.
- **No colour by status.** Herdr can colour a value by rules on its text, set in your config. The plugin's rows only dim
  the Coordinator's message, as the board greys it.
- **No bar.** The percent and time to go are there; the row's bar is not.
- **Not in the collapsed sidebar or a phone-width window.** Those keep Herdr's own compact layouts, which sidebar rows do
  not change (Herdr's configuration page says so; not tried here).
- **Another machine's tasks** are written by the plugin on that machine, into that machine's Herdr. They show here when
  this Herdr is connected to it and that machine runs this version of the plugin.

## How it gets there

- **One writer, no words of its own.** The board has one writer for all of it (`crates/flow/src/flow/sidebar.rs`). It
  draws the task's row with the board's own code, without colours (`crates/flow/src/flow/sidebar_entry.rs`).
- **When.** Whoever ticks (the keeper or an open board) writes an entry once a second when a word of it changed: as
  often as the board redraws its own rows. When a task comes to wait for you, goes wrong, is taken up again or ends,
  whatever did that writes the entry at once, without waiting for the tick.
- **No fights.** The Watcher and the check only keep what they find on the task ([the Watcher](watcher.md),
  [the check's progress](check-progress.md)). They write nothing to Herdr themselves, so no two writers fight over a
  line.
- **After a Herdr restart.** Herdr keeps none of this when its server restarts. The board notices on its next tick, by
  Herdr's own reading of the pane (the shown name it wrote is gone), and writes every entry of that Herdr again.
- **Nothing stale is left.** What was written is replaced, never left to expire. A task that is stopped, done or moved
  on gets its new words at once, or within the second. An entry goes with its pane (Herdr drops what was written about a
  closed pane). A task archived or removed while its pane lives on has the board's words taken off it.
- **As what.** It is written as Herdr's display metadata (`herdr pane report-metadata`, `herdr workspace report-metadata` and the space's name), from the source `plugin:herdr-projects`. Row 2 is the agent's shown name, row
  3 the token `progress`. The token `summary` is only the Coordinator's: on a task's pane and space it is cleared, so a
  line an older version wrote goes away by itself.
