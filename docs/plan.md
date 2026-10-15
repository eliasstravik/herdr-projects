# A plan first

**With `plan.approve` on, the agent writes a plan before it builds, and nothing in the repo is changed until the plan
has its yes.** A misunderstanding caught in two minutes saves a wrong build caught in forty.

```
herdr-projects delegate 12 --plan.approve you                                    # show me the plan first
herdr-projects delegate 12 --plan.approve you --plan.agent claude/claude-opus-5-5 # a strong model plans, the usual one builds
herdr-projects set plan.approve reviewer --repo my-app                           # in this repo the reviewer checks every plan
```

Or tell the [Coordinator](coordinator.md): "show me the plan first", "have Opus plan it".

```
task ─► the plan is written (nothing changes) ─► yes: you, or the reviewer ─► the agent builds that plan ─► check, review, merge
                     ▲                                │
                     └──── sent back with words ◄─────┘
```

## The two settings

| Setting | Values | Means |
|---|---|---|
| `plan.approve` | `none` (built in), `you`, `reviewer` | `none`: no plan, the agent builds at once. `you`: the agent writes its plan and the task waits as **plan ready** for your yes. `reviewer`: the reviewer checks the plan against the task's description, and the build goes on by itself when the plan passes |
| `plan.agent` | `none` (built in), or a program with an optional model after a slash (`claude/claude-opus-5-5`) | who writes the plan. `none`: the working agent. A program: a planner of its own |

- Both are work [settings](settings.md) like every other: looked up task → your entry for the repo → you → built in; set
  with `set`; taken by `add --delegate` and `delegate` as `--plan.approve you`; held by an [automation](automations.md);
  listed by `show`.
- **A repo's own file cannot set them**, so code an agent writes cannot switch a plan's approval on or off.
- **When they are read.** Both are read when the agent first starts on the task: set on a task whose agent already
  builds, they change nothing about that run. `plan.approve` is read once more when the plan is written, for who says
  yes: if it was turned off meanwhile, nobody is asked and the build goes on.
- A new round of a task that landed (delegated again after its clean-up) starts with a new plan.

## Plan ready: say yes, or send it back

With `plan.approve: you`, the task waits in a state of its own, **plan ready**. It stands under **needs you**: the row
says `Plan ready · <the plan's first words>`, the State line and Herdr's sidebar say `plan ready`, and `board.notify`
fires as for anything that needs you.

The task panel shows the plan under **Plan**: where it stands, who wrote it, the plan's own words (its first lines beside
the list, all of it full screen), and **`Open plan`**, a private link served the way a report's is. The web Board shows
the same part. You do one of two things:

- **Yes**: the approve key (`ctrl+l`), or `herdr-projects approve <id>`. The plan is approved as it stands and the build
  starts.
- **A reply**: `space`, then your words, or `herdr-projects tell <id> <text>`. The plan goes back to its writer with
  your words and is written again. The task is plan ready again when the new plan is there.

## The reviewer's look

With `plan.approve: reviewer`, the reviewer (`reviewer.agent`, in a tab named `Plan review`) reads the plan and the code
it speaks of, and answers as it does for work: does building this plan do what the task asked, and a list of findings by
severity. [The gate](gate.md) decides by the same rule (`review.mustFix`):

- the plan passes: the build goes on, and nobody is asked;
- it fails: the plan goes back to its writer with the reviewer's points, by itself, up to `review.rounds` times;
- still failing then: the task is plan ready, with what the reviewer found in the plan's part, and the yes is yours.

A look at a plan is not one of the task's review rounds: the first review of the work is still `Reviewer #1`.

## After the yes

The agent builds that plan.

- The plan's own writer hears `Your plan is approved` in its conversation. Claude Code is started again in the same
  conversation with its usual access, since its plan mode does not end by itself.
- After a planner, the working agent starts with the task and the plan's file.
- The review of the finished work is told of the approved plan. It checks the work against the task **and** the plan,
  and says where the build left the plan (`Left the plan` under the review in the task panel). Leaving the plan is a
  finding only when the result is worse for it.

## Who writes the plan

### The working agent (`plan.agent: none`)

It is told to change nothing; to write what it understood, what it will change and where, in which order, how it will be
checked and what it is unsure of; and to stop.

- **Claude Code uses its own plan mode**: it starts with `--permission-mode plan` in place of its access flags. In that
  mode it cannot edit unasked, and it presents its plan with its own tool (`ExitPlanMode`), which would ask in the
  terminal. The board takes the plan there instead, through a hook it passes at launch, and tells the agent to stop.
- **Every other program** (Codex, pi, Herdr's others: [agents](agents.md)) saves the plan as a file in the task's own
  folder.

### A planner of its own (`plan.agent: <program>`)

- It works in the task's worktree, in a tab of its own named **`Planner`** ([a task's tabs](tabs.md)), on its own
  program and model, while the working agent's tab `Agent` waits beside it.
- While it writes, it is the task's agent in every way: the row, the Watcher's line, the chat, a reply and the stop key
  are its.
- A planner that is not installed here: the working agent plans, and the task's settings say so in a `Note`.
- After the yes its tab closes, and **its time and tokens are kept on the task** (the plan's part of the task panel:
  `Planner used 51s · 18k in · 29 out`).

### The plan file

One file, `plan.md`, in the task's own folder on the board
(`~/.herdr-projects/data/tasks/<repo>/<n>/artifacts/plan/`), never in the repo.

- An agent that stopped without saving one: its last message is the plan.
- Nothing said either: `No plan was written`, which goes back to the agent like any failure.
- A plan that went back and was not saved again is the old one. Then what its writer said last is the new plan; with
  nothing said, there is no new plan. An older file is never what gets the yes.

## Nothing in the repo is changed before the yes

The board does not only ask for it. Four things hold, from the moment the plan step begins until the build starts:

1. **The plan's writer runs where the worktree is a read-only file system.** The board starts its program inside a mount
   of its own (bubblewrap: `bwrap --ro-bind`), where the task's worktree and its git folder cannot be written. A write, a
   delete, a commit, and also a `chmod` that would give the permission back, are answered `Read-only file system`,
   whatever the program's own access.
   - This holds for every program (Claude Code, Codex, pi and Herdr's others), for a planner of its own, and for the
     reviewer that checks a plan.
   - Everything else is as on the machine, so the plan can be saved in the board's folder.
   - After the yes, the program is ended and started again the usual way, outside that mount, in the same conversation.
   - **Where no such mount can be made** (bubblewrap is missing, or user namespaces are not allowed), **no plan is
     written in a repo at all**. The task does not start, and says what is missing and the two ways on: install
     bubblewrap, or `set plan.approve none --task <id>`. A lesser guard is never used instead.
2. **The worktree's files are read-only too.** Every file and folder in it loses its write permission, ignored files
   too, and so does the worktree's own git folder (its index, its `HEAD`). Whatever runs outside the mount is answered
   `Permission denied`. The permission comes back when the build starts (and before the worktree is removed).
3. **The board looks for itself**, when the plan's turn ends and once more right before the build starts. If something
   changed all the same (somebody took the read-only bit off), it undoes it in the task's own worktree: the branch goes
   back to the commit it stood on, added files go, and so do ignored files that were not there when the step began.
   What was undone is kept as `changed-too-early.patch` beside the plan, and the task says how much was put back
   (`Put back  2 changes made in the repo before the yes`).
4. **Nothing is built on a change the board may not or could not undo.** The plan is not shown and the build does not
   start: the task says `The repo was changed before the plan was approved`, with what changed. Sent to the agent, it
   undoes it. A yes already given stands, and the start is tried again once the change is gone.

And while the plan step is open, no turn's end counts as finished work: nothing is committed, checked, pushed, opened as
a pull request or reviewed.

What the mount is not: a wall against a program that sets out to get around it (one that asks Herdr to run a command in
another pane, say). It stops every write made the ordinary way, with or without the write permission put back. The
board's own look, and the hold on the build, are for the rest.

### A task in your own checkout

A task with no worktree of its own differs in one way: the board never takes the write permission off your files and
never undoes anything there, because what is there may be yours.

- Its plan's writer runs in the read-only mount all the same: your checkout is a read-only file system for that program
  only, never for you.
- Rule 4 holds for whatever changes there meanwhile: it goes back to the agent (`review.rounds` times, then it waits for
  you), and nothing is built until it is gone.
- A task in a repo always gets a worktree of its own when it is first delegated.

A folder that is no repo has nothing to guard: there the plan is written and approved the same way, with no mount.

## Several attempts on one task

The plan belongs to the task, not to an agent or a worktree: one file, written once, approved once. With
[several attempts](attempts.md) (`attempts.by`):

1. The task is not split when it starts. One writer makes the plan (the working agent, or the planner), and it is
   approved once.
2. Only then is the task split. Every attempt gets that same plan in its first prompt, and builds it.
3. The worktree and tab the plan was written in become attempt A's. Each other attempt gets its own beside it.
4. From there on, all is as for any task done several times: the check, the pick, one review of the chosen attempt,
   which is checked against the task and the plan.

`attempts.by` is read for this when the build starts, so a count changed while the plan waits counts.
