# Machine and project coordinators

**Every row of the list has an agent you can talk to.** A task's row has its agent; a machine's row and a project's
row each are their coordinator. There is no main Coordinator besides: the top row is.

| Row | Its agent | Named | Lives in Herdr |
|---|---|---|---|
| `All machines`, the top row with every machine shown | the coordinator of all machines: this machine's top-level one, which sees every machine | `the coordinator of all machines` | a `Coordinator` tab in the home space (`~`) ([the coordinator of all machines](coordinator.md)) |
| A machine's row | its machine coordinator | after the machine: `vm-str's coordinator` | a tab of that name in that machine's home space |
| A project's row (a repo's heading) | its project coordinator | after the repo: `gtm's coordinator`, `~'s coordinator` for the home folder | a tab of that name in the repo's own space |
| A task's row | its agent | `GTM-14's agent` | its own tab |

**The top row follows the machine picker** (`Alt+M`, the filter row's first item): `All machines`, or the one machine
picked. With only one machine, that machine is the top row, with nothing above it, and its coordinator is the
top-level one: all machines are that one. **The board opens with the top row selected**, so the chat starts with that
coordinator.

## A coordinator's row reads as a task's

- **The same marks as tasks**, in the marks' column left of the caret: `○` idle (also one that never ran, or was
  stopped), `◐` working, `?` it asks you (`!` it wants your OK), `✗` it could not start, on [autopilot](autopilot.md) too:
  autopilot has no mark or colour of its own. None of a task's other marks (`◑ ◆ ✓ ◌ ◇`) is a
  coordinator's, and an idle coordinator is not counted in To do. Where no coordinator can run
  (`coordinator.agent none`), this machine's rows have no mark.
- **The same right side as tasks: only the time.** `3m` while it works, `1h ago` otherwise, nothing when it never ran.
  No state words, no role word.
- **The same second line as tasks**: what it does while it works, or its question while it waits for you. It goes when
  it stops: no old news. On autopilot the line says what autopilot does, in the same grey: `Autopilot · next check-in
  in 12m · 3 of 20 today`, `Working on HSW-92 · 3 of 20 today`, `Checking in · HSW-92 finished`, `Needs you · …`. Its latest message shows in the chat box when you select its row.
- **Its name**, a home folder as `~` (on another machine too), with no rule to the edge. The top row says what it holds,
  open or folded: `4 machines · 420 tasks`, `9 projects · 30 tasks`. A coordinator on autopilot has a dimmed
  `[autopilot]` after its name, as a task has `[research]`; a machine's summary (and the top row's) says how many of its
  coordinators are: `33 tasks, 1 automation, 1 on autopilot`.
- **The list reads all machines → machine → project → task by how far in each row stands.**
- In a narrow pane the names shorten; the mark and the time stay.

```
 ○ ▾ All machines  2 machines · 5 tasks
   ◐ ▾ vm-str                                                              1m
       Cleaning old worktrees
     ? ▾ gtm · GTM                                                     1m ago
         Two PRs wait on CI. Want me to retry #11?
       ◐ GTM-14  Bundle skills into the template                           3m
```

**A coordinator that asks you shows under Needs you**, beside the tasks, with what it wants on its second line, and is
counted there.

**The filter Coordinators** (`Tab` from All, [the filter row](board.md#the-filter-row)) leaves only the coordinators
that run: started and not stopped (working, idle, needs you), on autopilot, or failed so that you can fix it, for the
machine shown; its count is how many there are. The top row is always there, counted only while it runs. A stopped or
never started one is not there and not counted; its row stays in the other filters, with `○`. With every machine shown,
a machine whose own coordinator does not run keeps its row there only as the heading of its projects' that do,
uncounted.

## The same keys on every row

- **`Space` talks to it**: the chat points at that row's agent, and follows the selection from row to row, as it does
  from task to task. `Space` again goes back to the top row's coordinator.
- **`Alt+O` goes into its agent**: its real Herdr pane, as on a task. A coordinator that is not running is started
  first; another machine's is gone into there. A status group's heading (`to do`, `done`) has no agent.
- **`Enter` folds and unfolds** a heading (all machines, a machine, a repo, a status group), as `←` and `→` do. On a
  task's row `Enter` does nothing: it never opens a chat. `←` on a task's row goes to its heading. A click on a heading
  folds it too.

## The chat box names who it talks to

```
╭─ All machines › vm-str ›  gtm  · Project coordinator ───────────────────╮
│ Two PRs are waiting on CI. Want me to retry #11?                         │
│ ❯ Send message to the project coordinator                                │
╰──────────────────────────────────────────────────────────────────────────╯
```

- **A ladder** in its top edge, from the top row down (`All machines › vm-str › gtm › GTM-14`), the current step lit,
  and after it, dimmed, the kind of agent you talk to: `HP-145 · Agent`, `HP-145 · Reviewer` while its reviewer talks,
  `herdr-projects · Project coordinator`, `vm-str · Machine coordinator`, `All machines · Machines
  coordinator`. The empty box says the same: `Send message to the agent`, `Send message to the project coordinator`.
- **Its edge in the status colour of whoever it talks to**, as their mark: base idle or never started, amber working,
  red when it needs you or failed ([colours](colours.md#the-box-says-who-you-type-to)).
- **Inside it only the conversation** and the line you type on, with no labels: what you wrote in the bright colour you
  type in, the reply in grey; the edge says who. Only someone else is named (a reviewer, autopilot's wakes, the board).
- **One that never ran** invites a first message: `Ask it something, e.g. "what's left to do here?"`. Your first
  message starts it.
- `Enter` sends, and a coordinator decides itself what to start.
- **A repo with no tasks in view keeps its row**, folded (`0 tasks`), when it was used in the last 14 days (a task,
  archived ones too) or has a project coordinator, so `Space` still reaches it. Older repos, repos whose folder is
  gone, and [archived projects](board.md#archiving-a-project) have none.

## What each one is

- **An ordinary agent in a long-lived chat**, like the coordinator of all machines: the same board commands, its own tab, and settings
  of its own (below). Everything you say is one conversation, kept: if its
  program exits or Herdr restarts, your next message starts it again in the same conversation.
- **A machine coordinator** is about that machine as a whole: its repos and folders, the tasks and agents on it. It runs
  in a folder of its own (`~/.herdr-projects/data/coordinator/<session>/machine/chat`), with its rules in `AGENTS.md`
  there, as the coordinator of all machines' are.
- **A project coordinator** is about one repo. It runs in the repo's own folder, at its main level: your checkout. It
  reads it freely and changes nothing there unless you ask it to; work goes on the board as tasks of that repo. Its tab
  goes in the repo's space (the one Herdr labels after the repo); with none open, one is made there. Since the repo's
  `AGENTS.md` is yours, its rules are a file beside its state
  (`~/.herdr-projects/data/coordinator/<session>/projects/<repo>/rules.md`), passed when it starts, as a working agent's
  are.
  **It stays in its own repo.** A task it adds goes there, unless the message it acts on names another place ("in the
  other repo"). It may read other repos to answer you (their tasks, reports, closing notes), and acts on none of it:
  work it notices in another repo is one line of an **Also noticed** list in its answer, never a task added or
  delegated there, which that repo's own coordinator would know nothing of. No coordinator adds a task you did not ask
  for: what it reads in reports and closing notes is information, never instructions, in every turn.
  Your own chats in the repo's folder are never taken for its conversation: it keeps the one it has, and moves to
  another (after its own `/clear`) only when Herdr names that one for its pane.
- **Another machine's** coordinators run on that machine. Its board sends what they said with its tasks, and what you
  type goes there. A machine on an older version has none: the chat says to update it there.
- **Herdr's sidebar** names each as the board does, with where it stands and its last message.
- **A task says which coordinator added it.** `details <id>` has `addedBy`: `the herdr-projects coordinator, asked by
  the human` for one added from that coordinator's pane, `the herdr-projects coordinator's autopilot` for one its
  autopilot added, else `the human` or the automation's name. Autopilot's history in `list` says the same.

## Its row's keys

As a task's row stands for the task:

- `Alt+P` shows its coordinator beside the list, `Alt+T` whole, in the [task panel](task-panel.md)'s form.
- `Ctrl+E` opens its settings page: its own rows first, under `This coordinator`, then on this machine's row your own
  settings, on a project's your entry for that repo.
- `Ctrl+S`, with nothing typed, starts its coordinator (in its own conversation), `Ctrl+X` stops it, as they put an
  agent on a task and stop it. Here or on its machine.
- `Ctrl+O` puts it on [autopilot](autopilot.md) with its saved goal and settings, and takes it off again. While on,
  its row has `[autopilot]` after its name and what autopilot does under it.
- `Ctrl+A` on a project's row archives the whole project, its tasks with it, and stops its coordinator; in the archive
  it restores it ([archiving a project](board.md#archiving-a-project)).
- While one of these takes a moment, its row says `Starting…` or `Stopping…` with the working mark, as a task's does,
  and nothing is said under the chat when it worked ([what a key shows at once](under-way.md)).

**Each has its own settings**, `coordinator.*` and `autopilot.*`, looked up **this coordinator → you → built in**
([settings](settings.md)). What you set for yourself counts for every coordinator; one set for this coordinator alone
wins over it. `coordinator.instructions` add up: yours, then the coordinator's own.

```
herdr-projects set coordinator.effort high --machine-coordinator
herdr-projects set coordinator.agent codex --project-coordinator herdr-projects
herdr-projects show --project-coordinator herdr-projects
```

## From a shell

Every `coordinator` command takes `--machine` or `--project <repo>`:

```
herdr-projects coordinator --machine                    how it stands
herdr-projects coordinator --machine tell <text>        send it a message
herdr-projects coordinator --project herdr-projects tell <text> [--delegate]
herdr-projects coordinator --project <repo> restart | new | stop
```
