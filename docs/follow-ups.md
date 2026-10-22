# Follow-ups: what a finished task left open

**A finished task keeps the work its closing note left open, and says so on its row.** An agent often ends with things
its task did not do: a manual check on staging, a step on a platform (a service account still missing in production),
a change another repo still needs, and the work it came across on the way. Agents add no task for these, and neither
does the Coordinator unasked. Once the task is merged its pane and worktree are gone, so the board keeps them for you as
the task's **follow-ups**.

## Where they come from

When an agent's turn ends, the board reads its closing note (or its last message, where it wrote no note) for these
lists, plain code and no model:

- **Still open**: what the task needs that the agent could not do here. Its rules tell it to write this list.
- **Also noticed**: unrelated work it came across (its rules always asked for this list).
- A list headed **Follow-ups** counts as Still open.

Each item is one line. Two marks in it are read and taken out of its words:

```md
## Still open
- Check the new payments page on staging (blocks)
- [repo: web] Show the new field on the settings page

## Also noticed
- `src/old.rs`: a stale comment about the retry
```

- `[repo: <name>]`: the item belongs to another repo. Added as a task, it goes there.
- `(blocks)`: the change is not complete or usable without it.

A later round's note that lists none (`Fixed the two findings.`) keeps the list from before. One that lists some
replaces it; an item you already added as a task or hid keeps that, matched by its words.

## Where you see them

Only once the task is finished (merged, or done): before that the work is still going on.

- **The row** says `2 follow-ups` where a done row says nothing else, on the board and on the web board. (herdr's
  sidebar does not: a merged task's pane is gone, and with it its entry there.) It counts the open ones; added and hidden ones are not counted.
- **The task panel**, beside the list and full screen, has a **Follow-ups · 2 open** part: each one
  numbered, with its repo and `(blocks)`, and the commands below.
- **`herdr-projects details <id>`** has them under `followups`: every item with its number, words, repo, `blocks`, kind
  (`open` or `noticed`) and state (`open`, `added` with the task it became, or `hidden`), and how many are open.
- **`herdr-projects list`** ends a finished task's line with `· 2 follow-ups` (`followups` in `--json`).

Archived tasks keep them: archiving tidies the list, not the work it left.

## All of them at once

```sh
herdr-projects followups               # every finished task with open follow-ups, archived ones too
herdr-projects followups --repo web    # of one repo
herdr-projects followups PRJ-12        # one task's, every one of them, with what became of each
```

## Add as task, or hide

You decide what becomes work. Each takes the task and the follow-up's number:

```sh
herdr-projects followups add PRJ-12 2      # a new task, not started
herdr-projects followups hide PRJ-12 3  # hidden from the row and the lists
herdr-projects followups open PRJ-12 3     # back again
```

**Add** makes a task in the follow-up's repo (`[repo: …]`, where the board knows that repo; else the repo of the task
it came from, and its description says so). It is not started: it stands under to do like any task you add. Its title
is the follow-up's words, and its description names the task it came from, the list it was on, its pull request and
`herdr-projects details <id>` of it. A follow-up is added once; the task's list then says `added as PRJ-31`.

Or ask the Coordinator: "add the staging check from PRJ-12 as a task", "hide the rest".

## Made tasks by themselves: `followups.add`

```sh
herdr-projects set followups.add todo              # yours, for every task
herdr-projects set followups.add todo --repo api   # for one repo
```

| Value | When the task merges |
|---|---|
| `none` (built in) | the follow-ups stay on it, for you to add or hide |
| `todo` | each open one is added as a task, as `followups add` would, not started |

It is read when the task merges, so a change counts for tasks still under way. With it on, the task panel's Merge row
says `follow-ups become tasks`.

## What the Coordinator is told

- Its rules say what follow-ups are, that `board list` gives each finished task's count and `board details <id>` the
  list, and that right after a task merges or is done it tells you in one short line what the task left open, the
  `(blocks)` ones first.
- On autopilot, the wake for a task that merged says it: `PRJ-12 merged, with 2 follow-ups open (board followups PRJ-12)`.
- It adds or hides one only when you ask, as with any task.
