# Research tasks and reports

This page lists the facts about reports and research-only tasks. It tells where a report goes, what the setting
`research.only` does, what the agent is told, and what the board does at the finish. For the steps, see
[make a research task](../how-to/make-a-research-task.md).

## Reports

**A report never goes into the repo.** A report, a review or a research result is something that an agent writes for
you to read. It is not part of the product. Thus the board never commits it, in no folder. The board keeps it with
the task and gives you a private link to it.

### Where a report goes

The board tells every agent that it starts the same thing, in every repo:

- **The agent writes the report into the task's own folder among the reports of the board**:
  `~/.herdr-projects/reports/<repo>/<number>/` ([the home folder](home-folder.md)). The report is `index.html` (one
  page that stands by itself) or `report.md`.
- That folder is outside every repo. Thus nobody can commit the report by mistake, and the report stays when the
  worktree of the task goes.
- **When the agent finishes, the board gives the report a link.** The board serves it in the same way as the evidence
  of a review: privately on your tailnet with `serve: tailscale`, or on `127.0.0.1` with `serve: local`
  ([settings](settings.md)).

The link opens the first of these files that is there:

1. `index.html`
2. `report.html`
3. `report.md`
4. `README.md`
5. the first file of the folder

### Where you find the link

- The task panel shows the link under **Report**, beside the list and in full screen (`Open report`). Full screen also
  shows the name of the file and the full link.
- `herdr-projects details <id>` has it as `report`.

This is true for every task. A task that changes code and also wrote a report has both: its change in the pull
request, and its report on the task.

## Research-only tasks

**A research-only task changes nothing in the repo.** Its whole result is a report. Work that must follow from it is a
short **Also noticed** list in that report. Nothing adds a task for it.

```
herdr-projects add "Why is the build slow?" --research
```

## The setting `research.only`

| | |
| --- | --- |
| Name | `research.only` |
| Values | `true`, `false` |
| Built-in value | `false` |
| Look-up order | the task → your entry for that repo → you → the repo's own file → built in. The nearest answer counts first. |
| In the repo's own file | `.herdr-projects.json`: `{ "research": { "only": true } }` |

The name reads as what the board calls it everywhere, "research only", in the dotted style of `merge.by` and
`review.when`.

### Ways to set it

| For | How |
| --- | --- |
| one task, when you add it | `herdr-projects add "<title>" --research` (short for `--research.only true`) |
| one task, later | `herdr-projects delegate <id> --research` (`--no-research` turns it off), `herdr-projects set research.only true --task <id>`, or its row on the settings page of the task (`Research only`). The row is on the terminal board, and on the web board under the open task (**All settings**). All of these write the same one setting on the task. |
| a whole repo | `herdr-projects set research.only true --repo <name>`. Every new task there is research only, unless the task says otherwise (`--no-research`, or `set research.only false --task <id>`). |
| everything of yours | `herdr-projects set research.only true` (without `--repo`). The same exception applies. |
| an [automation](automations.md) | `--research` when you make it, or `set research.only true --automation <name>`. Each task that it makes has the setting as its own. |

- **The Coordinator sets it by itself** when you clearly ask for research, a review or a report with nothing to
  change. Examples: "look into why the build is slow and tell me", "write me a report on …". It does not set it when
  you want a change, or when it cannot tell.
- `herdr-projects show <id>` lists it, with where the value came from: `this task`, `your entry for <repo>`, `you`,
  `the repo's file` or `built in`.
- `unset research.only` removes an answer again.

### On the board

- The row says `[research]` after the name of the task. It does so on the board, in the sidebar of Herdr and on the
  web board. This is also true for a task of an automation and for a task on another machine. The label comes from
  the setting, never from the title.
- The task panel has a part **Research only** with the link of the report (or `No report yet`).
- Its settings say `Research  only · no change to the repo`, where an ordinary task says how it merges.

## What its agent is told

- Change nothing in the repo: add, edit and delete no file, and commit nothing.
- The result is a report, saved as above.
- Work that must follow from it is a short "Also noticed" list in that report. Nothing more is done with it, and the
  agent adds no task for it.
- The rules do not mention the check of the repo: there is no change to check.

Saving the report finishes nothing. The agent can save it early and add to it while it still waits for your answers.
You can open it in the meantime. The task is finished when its agent stops without saying that it waits
([an agent that waits for you is not finished](../explanation/waiting-for-you.md)).

## What the board does at the finish

The board looks at the worktree and the branch of the task:

- files that are not committed, new files too;
- everything that the branch holds beyond the main that it started from;
- every commit on it, also a commit that changes no file, and a merge.

A worktree on a branch that is not its own counts as a change.

### Nothing changed (the usual case)

- The board does not bring the branch up to date with main. The check of the repo does not run. Nothing is pushed,
  and no pull request is opened.
- The reviewer reviews the report, unless the task has no review (`reviewer: off` or `review: none`). The reviewer is
  told that the task is research only. It reads the report, and checks it against what was asked and against the code
  and sources that it speaks of.
- A change to the repo that the reviewer finds (a commit in another place, something pushed) is a critical finding. A
  missing report is a critical finding too.
- A review that fails the gate goes back to the agent like any other ([rules of the gate](gate-rules.md)).
- The task ends as **done**, with nothing to merge and with the link of its report on the task. If there is no report
  in the folder, its line ends with `No report was found.`

### Something changed

- Nothing is committed, pushed, opened as a pull request, or merged.
- The task goes back to its agent with the list of what changed. The agent must move the result out of the repo and
  undo every change, commits too.
- That is one round of the review loop. After `review.rounds` rounds (3 by default), the task stops and waits for you.
  Its row says why:
  `Research only, but its agent changed the repo again after 3 rounds (2 changes in the repo): nothing was pushed, opened or merged`.
- From there, you can send it back one more time (`Ctrl+R`).
- If you want the change after all, delegate it again as an ordinary task: `herdr-projects delegate <id> --no-research`.
  This is `set research.only false --task <id>` and a start, in one command.

**A research-only task can never be merged.** The board refuses to approve one, and `merge.by: itself` passes it by.
To merge its change after all, turn the setting off for the task and delegate it again.

## Changing it on a task that is under way

The board reads the setting when the agent of the task starts, as it reads every setting when its step starts. The
task keeps what it was started as until its next start. That is what the agent was told, what the finish line holds
it to, and what the label says.

"Starts" means the start of the program of the agent itself, not the request for it. By then the task has its
worktree. Thus the repo's own file counts as of the commit that the worktree starts from, and the setup of the task
has run.

| The task is | A change of the setting |
| --- | --- |
| not started yet, waiting for a free slot, or its setup still runs | counts at once. The row follows, and the agent starts as the setting says at that moment. |
| already started (working, stopped, waiting for you, failed) | is saved and counts **from its next start**. Delegate it again, and its agent is told the new rules. Until then, `show <id>` and the settings page say so beside the value: `counts from its next start: this run is not research only`. |

- A changed default (for a repo, for you, in the repo's file) never changes a task that was started.
- **Turning it on is refused** where the board cannot honour it, with the reason. This is so when the agent of the
  task has changed the repo (a file, a commit), or when its pull request is open. A research-only task changes
  nothing, so that work would have to be discarded. Undo it or close the pull request first, or add a new task as
  research only.
- The same is true for a task that would start as research only by a default, after it worked as an ordinary task.
  The board refuses to delegate it until the task says what it is (`set research.only false --task <id>`).
- **Turning it off is never refused.** It counts from the next start. From then on, the task is an ordinary task.

Sometimes the setting of a task changed after its agent started. At its next start, the task is told that it is
research only from now on, or that it no longer is. It gets the rules as they are from then on.

A task with the old `--research` mark, from before this was a setting, is still research only. The board reads the
mark as the task's own `research.only`. A task that was started keeps what it was started as.

## A task that works in your own checkout

A task in a repo always has its own worktree now. But some tasks have none. One example is a task that an older
version started in your checkout (it had `--no-worktree`). Another example is a task in a folder of which git cannot
make a worktree. What changes in your checkout can be yours. Thus:

- When the agent starts, the board records the state of the checkout. This is its commit, and every changed file with
  its content, its mode and what of it is staged (the staged mode too).
- The board also reads a file that git was told not to look at (`assume-unchanged`, `skip-worktree`).
- When the task finishes, every file that reads differently, and every commit since, is a change. The task goes back
  as above.
- Its agent is told to undo only what it changed itself, file by file. It must never reset or clean your checkout.
- The board also lists a file that you changed yourself while the agent worked: the board cannot tell whose change it
  was. The agent says so. You retry, or delegate it again with `--no-research`.

The board does not look at a folder that is no repo at all, or at a repo with no commit yet: there is nothing to
compare. There, only the rules of the agent and the reviewer hold.

## Related pages

- [Make a research task](../how-to/make-a-research-task.md)
- [An agent that waits for you is not finished](../explanation/waiting-for-you.md)
- [Rules of the gate](gate-rules.md)
- [Settings](settings.md)
- [The home folder](home-folder.md)
- [Automations](automations.md)
