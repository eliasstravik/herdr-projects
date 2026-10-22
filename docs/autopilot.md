# Autopilot

**Autopilot is a switch on a coordinator.** While it is on, the board sends that coordinator a short wake message in the
chat it already has, whenever its timer or something on the board says there may be work to do. The coordinator then
works toward its goal with ordinary tasks, as you would, within its limits. Off, it is exactly the coordinator it always
was.

Any coordinator can be put on autopilot: the [Coordinator](coordinator.md), a machine coordinator or a project
coordinator ([machine and project coordinators](machine-and-project-coordinators.md)). There is no second agent and no
second conversation: it is the same chat you talk to, and it remembers what you told it.

**`autopilot.approve yes` overrides `merge.by`.** With its default, `no`, a task autopilot added merges only as your
`merge.by` says: under `merge.by you`, it waits for your yes. With `yes`, the coordinator may say that yes itself, so its
tasks merge without you. It is the one setting that does that. Leave it at `no` unless you mean it.

## Turning it on and off

The easiest way is to tell a coordinator in plain words: "go on autopilot: sync never loses data", "put the vm-str
coordinator on autopilot, goal: keep the tests green, two tasks a day", "turn all autopilot off". It runs the commands
below. You never need to type them yourself.

On the board, on a coordinator's row (below), with nothing typed:

| Key | Does |
|---|---|
| `Ctrl+O` | autopilot on, with its saved goal and settings; and off again. On one that says `needs you` or `could not start`, it starts again |

`Ctrl+S` and `Ctrl+X` there start and stop the coordinator itself, as they put an agent on a task and stop it; autopilot
has its own key.

From a shell:

```
herdr-projects autopilot                     this machine's coordinators, each with how it stands
herdr-projects autopilot show [--machine | --project <repo>]   one coordinator's autopilot whole, as it stands now
herdr-projects autopilot on [--machine | --project <repo>] [--goal "<words>"] [--autopilot.<name> <value> …]
herdr-projects autopilot off [--machine | --project <repo> | --all]
```

- **No flag is the main Coordinator**, as with the `coordinator` commands. `--machine` is this machine's coordinator,
  `--project <repo>` that repo's.
- **It needs a goal.** `--goal` saves one as that coordinator's own. With none given it uses the saved one; with none
  saved it is refused: `Tell a coordinator its goal first.`
- **Other values given with `on` are for this time only**, and last until it goes off (below).
- **Not under `coordinator.access ask`**: a wake would stop at the first permission prompt. A coordinator already on
  that is given `ask` shows `needs you`.
- **`on` while it is on** changes what it names (a new goal is saved, a value given replaces this time's of that name,
  the rest stay) and wakes it. From `needs you` it starts again with a clean count.
- **Turning on wakes it**, as soon as any turn of yours is over. On the board it says nothing: the coordinator's row
  shows it (below). The command says what it turned on, with its goal.
- **Off** drops this time's values and every reason it had to wake. A turn under way is not cut off (its chat is yours
  too), but from then on every write of that turn is refused: `autopilot was turned off`.
- **`off --all` is this machine only.** Another machine's coordinators are turned off there, or one by one with
  `Ctrl+O` on their rows.
- `herdr-projects autopilot` lists this machine's coordinators. Another machine's are on its own board.
- **`coordinator new`, `restart` and `stop` leave autopilot as it is.** Its next wake goes to the new chat.
  `coordinator stop` on one on autopilot says so: its next wake starts it again, and `Ctrl+O` turns autopilot off.

## Where it shows

- **A machine's row and a project's row stand for their coordinator**, and the top row (`All machines`, or the only
  machine) for the top-level one; the filter Coordinators (`Tab`) leaves only those
  ([machine and project coordinators](machine-and-project-coordinators.md)).
- **While it is on, its row keeps its coordinator's own mark** (`○` idle, `◐` working, `?` it asks you, `✗` could not
  start), in its own colour: autopilot has no icon and no colour of its own. It shows only in the board's own patterns:

  ```
   ○ ▾ vm-str [autopilot]  33 tasks, 1 automation, 1 on autopilot
       Autopilot · next check-in in 12m · 3 of 20 today
     ○ ▾ herdr-projects · HSW [autopilot]
         Working on HSW-92 · 3 of 20 today
  ```

  - **A tag after its name**, `[autopilot]`, dimmed as a task's `[research]` is.
  - **A status line under it**, in the grey of a task's, the first that holds:

    | Line | When |
    |---|---|
    | `Needs you · …` | it stopped waking (below), with why |
    | `Could not start · <why>` | its program could not be started |
    | `Checking in · HSW-84 finished` | a wake's turn is under way, with why it woke |
    | `Working on HSW-92 · 3 of 20 today` | one of its tasks is under way (and today's adds of `tasksPerDay`) |
    | `Autopilot · asleep outside mon-fri 09:00-18:00 · 3 of 20 today` | outside its hours |
    | `Autopilot · at today's limit · 5 of 5 today` | no room for more tasks |
    | `Autopilot · next check-in in 12m · 3 of 20 today` | waiting for its next reason to wake |

  - **Its machine's summary** (a folded machine's row, and the top row) says how many coordinators there are on autopilot:
    `33 tasks, 1 automation, 1 on autopilot`.
  - The web board shows the same, in its own row.
  - Off, nothing of autopilot shows.
- **The state word**, on its panel and from `herdr-projects autopilot`, the first that holds:

  | Word | Means |
  |---|---|
  | `needs you` | it stopped waking: its wakes went unanswered, or `coordinator.access` is `ask` |
  | `could not start` | its program could not be started, with why |
  | `working` | a wake's turn is under way |
  | `asleep` | outside its hours: `waits for its hours` where `next in` goes |
  | `at its limit` | no room for more tasks, and none of its tasks needs looking after |
  | `on` | waiting for its next reason to wake |

- **Its panel.** `Alt+P` shows the coordinator beside the list in the [task panel](task-panel.md)'s form, `Alt+T` the
  same whole, with its last 20 wakes (when, why, answered). While on: its goal, today's counts
  (`2 of 5 added · 1 of 2 under way`) and its settings in plain words.
- **Its wakes show in its chat**, under the name `Autopilot`, as the board's own words stand under `Board`, never as
  your words: only why it woke (`Regular check-in`, `HSW-84 finished`), in the chat's usual greys.
- **No colours of its own** ([colours](colours.md)).

## When it wakes

It has three kinds of reason:

- **Its timer**, `autopilot.every`, counted from the end of its last wake (or its last look that found nothing new).
  Missed ticks are not caught up.
- **Board events** in its area, `autopilot.when`: `done` (a task was merged or done), `stuck` (a task it may look after
  starts to need looking after: once, not again while it stays so), `added` (a task was added by anyone but this
  coordinator). Only what happens after it was turned on counts, and each fires once.
- **Being turned on.**

**The free look.** Before a timer wake, the board looks for itself, at no cost: no model, no tokens. Only if something
changed since its last wake ended does the timer wake it: the tasks in its area and their stages, its goal and settings,
or whether today's limits have room. **A new day with room counts as a change**, so a coordinator left on wakes at
least once a day while it has room. What its own turn did wakes nothing: the look is taken when its turn ends.

**No wake while** it is off; it says `needs you`; it is outside `autopilot.hours`; it is within `autopilot.gap` of the end
of its last wake's turn or of a turn of yours; `autopilot.wakesPerDay` are used today; it is mid-turn, or a message of
yours waits for it; or it has no room and none of its tasks needs looking after. The reasons are kept and go out as one
wake when that ends. A reason that is only the timer is dropped if the look then finds nothing new.

**You come first.** At most one wake waits in its queue. When a message of yours is queued, a wake not yet typed in is
taken out again, and its reasons wait. Only the wake of turning on skips the gap after your turn.

**The wake message** always starts with `Autopilot:`, and says only why it woke, in plain words:

```
Autopilot: HSW-84 finished, HSW-88 has a question
```

| Reason | Words |
|---|---|
| turned on (or changed) | `Turned on` |
| its timer, and the board changed | `Regular check-in` |
| `done`: a task merged or done | `HSW-84 finished` |
| `stuck`: it failed | `HSW-84 is stuck` |
| `stuck`: its agent asks a question | `HSW-84 has a question` |
| `stuck`: it waits for a yes autopilot may give | `HSW-84 is ready to approve` |
| `added`: a task was added by someone else | `HSW-84 was added` |

No goal, no limits and no counts: **its rules say to read its autopilot first in every wake**, as it stands now, with
`herdr-projects autopilot show` (`--machine`, `--project <repo>`): its state, goal, today's counts, its tasks under way
and those that need looking after, and its settings in plain words (what it may set and touch, whether it may say yes
or add releases). So a change between wakes is always picked up, and nothing is repeated in the chat.

Only the board writes a wake. Any other message that starts with `Autopilot:` (from the chat box, `coordinator tell`,
another machine) reaches it as `(not a wake) Autopilot: …`, and so does one that starts with an earlier version's
`Autopilot woke:`. A coordinator that is not running is started first, in the same conversation, as your first message
would start it.

- **Answered** is a wake whose turn ended within 15 minutes. After `autopilot.needsYouAfter` unanswered in a row, its state
  is `needs you` (its row's line: `Needs you · …`) and no more wakes are sent until `Ctrl+O` or `autopilot on`. One answered sets the count back to zero.
- **A start that failed** is not a wake: it counts toward no limit, waits for the gap before the next try, and its row's
  line says `Could not start · <reason>` until a start works.

## What it may do in a wake

A turn that a wake started is autopilot's until a message of yours is typed in: what follows is on your word. In
autopilot's turn only these commands go through. Everything else (a merge, a gate override, editing or renaming a task,
`set`, automations, repos, a message to another coordinator) is refused, with the rule and, where there is one, the
setting that would allow it.

- **Reads** about its area: `list`, `show`, `details`, `autopilot` (and `autopilot show`), `coordinator where`. The board is its only memory:
  `list` also gives it its area's finished, stopped and archived tasks of the last 30 days, each with who added it and
  the start of its closing note.
- **`add`**, in its area, within `autopilot.tasksPerDay` and `autopilot.tasksAtOnce`. A setting on it that
  `autopilot.mayChange` does not allow is not applied, and the answer names it. With `autopilot.startAtOnce yes` it starts
  at once, or after one of its own tasks.
- **`delegate`** its own tasks, and under `autopilot.scope all` your to-do tasks in its area, on its own machine.
- **`tell`** the agent of a task it may look after that asks a question; never into a permission or approval prompt.
- **`retry`** a task it may look after, at most `autopilot.retries` times. Answering a question uses no retry.
- **`stop`** and **`archive`** its own tasks, not one waiting for your yes.
- **`approve`** a plan or a merge, only under `autopilot.approve yes`, on tasks it may look after. Never `--anyway`.
- **`autopilot off`**, for itself, another or `--all`. `autopilot on` for another only under `autopilot.switchOthers
  yes`, and only with that one's saved goal and settings.

**It cannot widen its own powers.** In autopilot's turn it cannot raise its own limits, write a setting or turn itself
on with other values.

**How it counts, everywhere:** tasks **added** today, of `tasksPerDay`, and tasks **under way**, of `tasksAtOnce`: from
started (or added, when chained) until merged, done, stopped or archived. Waiting for you counts. Days are this
machine's.

**What it may look after**: the tasks it added, and under `autopilot.scope all` your to-do tasks it started. Never a task
of yours it did not start. One of them **needs looking after** when it failed (the check, the review after its rounds, a
limit, its agent's program, its setup) or its agent asks a question; under `autopilot.approve yes`, also when it waits
for a yes autopilot may give. One that fails or asks again after its retries are used is **left to you**: it wakes
nothing more, and autopilot may not touch it.

**Its tasks are ordinary tasks**, with the same check, review, gate and merge, by their repo's settings plus what
`mayChange` allowed. Three things differ:

- Their agent and reviewer may read the board, and `report` on their own task. Every other board command is refused, so
  nothing its tasks do adds work outside its limits. A command is its agent's (or reviewer's) when it runs in that
  pane in a terminal the agent was seen in: Herdr gives a closed pane's id to the next pane of its space, so a
  coordinator in a done task's old pane is that coordinator, not the done task's agent.
- Under `autopilot.repoFileWaits yes`, a change that touches the repo's own `.herdr-projects.json` never merges by
  itself, whatever `merge.by` or `autopilot.approve` say. Its row says it waits for your own yes.
- An approve from autopilot's turn is refused on any task whose change touches that file, whoever added it.

**Only you switch it on.** A task's agent or reviewer can never turn autopilot on or off, or write a `coordinator.*` or
`autopilot.*` setting, not even by asking a coordinator. Turning off is open to you and to any coordinator's turn.

**Doing nothing is fine.** Its rules say: when the goal looks met or nothing is worth doing, say so in one line and stop.
Never invent work. No release task (a version bump, a tag, a publish) unless `autopilot.releases yes`; with `no`, it
says when it thinks one is due.

## Its settings

A coordinator has settings as a task has, looked up by the same rule, the nearest answer wins:
**this time → this coordinator → you → built in** ([settings](settings.md)). They are `coordinator.*` (its program,
model, effort, fast mode, access, your words) and `autopilot.*`.

```
herdr-projects set autopilot.every 2h --project-coordinator herdr-projects
herdr-projects set coordinator.agent codex --machine-coordinator
herdr-projects show --main-coordinator
```

- **You** is your `config.json`, as for every setting: a `coordinator.agent` you set there counts for every coordinator.
- **This coordinator** is the `coordinators` part of that same file:
  `{"coordinators": {"main": {…}, "machine": {…}, "projects": {"herdr-projects": {…}}}}`. `set`, `unset` and `show`
  take `--main-coordinator`, `--machine-coordinator` or `--project-coordinator <repo>`.
- **This time** is the values given with `autopilot on`, `autopilot.*` only, kept until it goes off.
- **`autopilot.goal` is for one coordinator only**: never yours for all, never for this time.
- **Nothing of them is read from a repo's `.herdr-projects.json`**, so code written by agents can never change them.
- `coordinator.instructions` add up: yours, then the coordinator's own.
- A changed `coordinator.*` counts from its next fresh chat, and the panel says so. A `coordinator new --agent` pick
  still wins for the chat it starts.

On the board, `Ctrl+E` on a coordinator's row opens its settings page, in the task settings page's form and keys. Its
own rows come first, under `This coordinator`; then on this machine's row your own settings, on a project's row
your entry for that repo. While autopilot is on, `Enter` on an autopilot row other than the goal saves it for this time
(`*`), and a line under the rows says so. The goal always saves as this coordinator's own. `Ctrl+U` puts a row back.

| Setting | Built in | In short |
|---|---|---|
| `autopilot.goal` | `none` | what it works toward |
| `autopilot.every` | `6h` | its timer |
| `autopilot.gap` | `10m` | the least time between wakes |
| `autopilot.wakesPerDay` | `24` | the most wakes a day |
| `autopilot.when` | `done stuck added` | the events that wake it |
| `autopilot.hours` | `any` | when it may wake (`mon-fri 09:00-18:00`) |
| `autopilot.tasksPerDay` | `5` | the most tasks it adds a day |
| `autopilot.tasksAtOnce` | `2` | the most of its tasks under way |
| `autopilot.mayChange` | `none` | the task settings it may give its tasks |
| `autopilot.scope` | `own` | `all`: it may also start your to-do tasks |
| `autopilot.approve` | `no` | it may say yes for its tasks: overrides `merge.by` |
| `autopilot.switchOthers` | `no` | it may turn others on by itself |
| `autopilot.releases` | `no` | it may add a release task |
| `autopilot.retries` | `1` | retries of one task before it is left to you |
| `autopilot.startAtOnce` | `yes` | its tasks start at once |
| `autopilot.needsYouAfter` | `3` | unanswered wakes before `needs you` |
| `autopilot.repoFileWaits` | `yes` | a change to `.herdr-projects.json` waits for you |

What each means in full is in [the list](settings.md#the-list).

## The levels

| Coordinator | Its area |
|---|---|
| the main Coordinator | everything the board shows, every machine |
| a machine coordinator | that machine's repos |
| a project coordinator | its repo |

- Two on one repo (the main Coordinator and that repo's) each look after their own tasks and count their own limits.
  There is no limit across coordinators.
- **They can wake each other**: a task one adds is `added` in the other's area. Each is still held by its own gap,
  wakes a day and tasks a day.
- Under `autopilot.scope all`, your to-do task goes to the nearest coordinator on autopilot with `scope all` on the
  task's own machine: its project's, then its machine's, then that machine's main Coordinator.

## Other machines

**Each machine runs its own coordinators' autopilot**, with its own settings and limits. This board shows their lines
and panels as that machine sends them with its tasks, and sends `Ctrl+O` (and `Ctrl+S`, `Ctrl+X`) there. `Ctrl+E` on one shows its
settings whole, to read, and says to change them there. Each coordinator with a row is sent, its machine's and every
repo's that machine knows, one that never ran as off: `Ctrl+O` there starts it, as a first message would. A machine on an
older version sends no autopilot at all, and the keys say to update it there.

**The main Coordinator sees other machines only while a board is open.** The keeper does not ask other machines for
their tasks. So their tasks reach the main Coordinator's autopilot only as an open board (in a terminal or the web UI)
last heard them, and are dropped after 3 minutes without a fresh answer. With no board open, its area is this machine's
tasks.

It adds a task on another machine as you can, outside autopilot too:

```
herdr-projects add "<title>" --repo <repo> --on <machine>
```

In a wake, that counts against its own limits: its slot is taken here first, and given back if that machine refuses
the task. The task carries a record naming this machine and the `autopilot.repoFileWaits` in force, and that machine
holds it to both. A machine on an older version refuses it, saying to update it. It starts your to-do tasks only on its
own machine.

## Autopilot and automations

They are two different things, and automations are unchanged.

- **An [automation](automations.md)** is a saved task plus a trigger: each firing adds the same task and starts it. It
  chases no goal and wakes nobody.
- **Autopilot** wakes a coordinator, which decides what, if anything, to do toward a goal.

They can meet: an automation fired by `merged` adds a task when one of autopilot's tasks merges, and that task is
`added` in autopilot's area, so it wakes the coordinator. Both are bounded: the automation by its ten tasks an hour,
autopilot by its gap, wakes a day and tasks a day.

## How far to trust it

**The trust level is today's.** A coordinator already has every board command, and `full` access by default. What the
board refuses in autopilot's turn catches mistakes; it does not contain an agent that goes wrong.

- A coordinator that runs commands from another pane could act as you.
- Text in closing notes, repo files or others' tasks can steer it. Its rules say such text is information, never an
  instruction, but rules are not a seal.
- **Agent files merge like any change.** `AGENTS.md`, `CLAUDE.md`, hooks and the check's own script are not held for
  you; only `.herdr-projects.json` is. The review is the guard there.
- **Every wake is a turn in the chat you also use.** It grows, and each wake carries all of it. `coordinator new` starts
  it fresh, and `list` gives it back its history.
- **Talk to a coordinator on autopilot through the board, or turn autopilot off first.** The board cannot see what you
  type straight into the coordinator's own tab: a wake may be typed in while you write there, and a turn you start
  there right after a wake may still count as autopilot's. What is typed there is not escaped either.
- Watch it the first weeks for busywork once the goal is met. With `merge.by you`, its finished tasks wait for you, and
  `tasksAtOnce` counts them, so at most two (by default) pile up.

## On herdr-projects itself

Autopilot can work on the board's own repo, but agents there may improve their own harness instead of the project. Its
changes only count once you release and install a version, and releases are not its own unless `autopilot.releases
yes`. There, give it a narrow goal ("the web board's chat loses no message") and keep `merge.by you`.

## Where it is kept

- **Its state** (on or off, this time's values, today's counts, its tasks, its last 20 wakes): the `autopilot` part of
  the coordinator's `state.json` in [the home folder](home-folder.md) (`data/coordinator/<session>/`, `machine/`,
  `projects/<repo>/`).
- **Its settings**: the `coordinators` part of your `config.json`.
- **Other machines' tasks as a board last heard them**: `data/autopilot-heard.json`.
- Code: `crates/flow/src/autopilot/`, and its rules in `crates/flow/src/coordinator/rules_autopilot.rs`.
