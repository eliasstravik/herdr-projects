# A plan first

This page explains how a plan before the build works, and why the board keeps the repo unchanged until the plan gets
its yes.

## Why a plan first

With `plan.approve` on, the agent writes a plan before it builds. Nothing in the repo changes until the plan gets its
yes. You can find a misunderstanding in two minutes, before a wrong build that takes forty minutes.

```
task ─► the plan is written (nothing changes) ─► yes: you, or the reviewer ─► the agent builds that plan ─► check, review, merge
                     ▲                                │
                     └──── sent back with words ◄─────┘
```

The yes comes from you (`plan.approve: you`) or from the reviewer (`plan.approve: reviewer`). The plan can come from
the working agent or from a planner of its own (`plan.agent`). The facts are in
[plan settings](../reference/plan-settings.md).

A repo's own file cannot set `plan.approve` or `plan.agent`. Thus code that an agent writes cannot turn a plan's
approval on or off.

## What the plan holds

The working agent is told to change nothing. It is told to write these things, and then to stop:

- what it understood;
- what it will change, and where;
- in which order;
- how it will check the work;
- what it is not sure of.

Claude Code uses its own plan mode. In that mode it cannot edit when nobody asked it to. It presents its plan with its
own tool, `ExitPlanMode`, which usually asks in the terminal. The board takes the plan there instead, through a hook,
and tells the agent to stop.

Claude Code's plan mode does not end by itself. Thus, after the yes, the board starts Claude Code again in the same
conversation, with its usual access.

## The reviewer's look

With `plan.approve: reviewer`, the reviewer reads the plan and the code that the plan speaks of. It answers as it does
for work: does a build of this plan do what the task asked, and a list of findings by severity.
[The gate](gate.md) decides by the same rule as for work.

## The review after the build

The review of the finished work checks it against the task and the approved plan. It says where the build left the
plan. Leaving the plan is a finding only when the result is worse because of it.

## Nothing in the repo changes before the yes

The board does not only ask the agent to change nothing. Four things hold, from the start of the plan step until the
build starts.

1. **The plan's writer runs where the worktree cannot be written.** The board starts its program inside a mount of its
   own. The platform's own tool makes the mount. In it, the task's worktree and its git folder cannot be written. A
   write, a delete, a commit, and a `chmod` that gives the permission back, all fail, whatever the program's own
   access is.
   - This holds for every program (Claude Code, Codex, pi and Herdr's others), for a planner of its own, and for the
     reviewer that checks a plan.
   - All other things are as on the machine. Thus the plan can go in the board's folder, and the program keeps its own
     session files.
   - After the yes, the board ends the program and starts it again the usual way, outside that mount, in the same
     conversation.
   - Where no such mount can be made, no plan is written in a repo at all. The task does not start. The board never
     uses a weaker guard instead.
2. **The worktree's files are read-only too.** Every file and folder in it loses its write permission, ignored files
   too. The worktree's own git folder (its index, its `HEAD`) also loses it. Thus a program that runs outside the mount
   also cannot write there. The permission comes back when the build starts, and before the board removes the
   worktree.
3. **The board looks for itself.** It looks when the plan's turn ends, and one more time just before the build
   starts. If something changed all the same (for example, somebody removed the read-only bit), the board undoes it in
   the task's own worktree. The branch goes back to the commit it was on. Added files go. Ignored files that were not
   there at the start of the step also go. The board keeps what it undid as `changed-too-early.patch`, beside the plan.
4. **Nothing is built on a change that the board must not or could not undo.** The plan is not shown, and the build
   does not start. When the problem goes to the agent, the agent undoes the change. A yes that you already gave
   stays, and the board tries the start again when the change is gone.

While the plan step is open, no end of a turn counts as finished work. Nothing is committed, checked, pushed, opened as
a pull request or reviewed.

### What the mount does not do

The mount is not a wall against a program that tries to go around it, for example one that asks Herdr to run a command
in another pane. It stops every write made the ordinary way, with or without the write permission put back. The
board's own look (rule 3) and the hold on the build (rule 4) are for all other cases.

### The same guard on Linux and on macOS

Linux uses bubblewrap, and macOS uses its own sandbox. The guarantee is the same on both: the program cannot write
under the worktree and its git folder, and it cannot remove the guard.

macOS allows no sandbox inside another. A program that runs its own commands in its own sandbox would see every command
fail inside the board's sandbox. Thus, for the plan step, the board turns off the program's own sandbox, and the
board's sandbox is the guard. The details are in
[plan settings](../reference/plan-settings.md#the-programs-own-sandbox-on-macos).

## A task in your own checkout

A task with no worktree of its own is different in one way. The board never removes the write permission from your
files, and it never undoes anything there, because what is there can be yours.

- Its plan's writer runs in the read-only mount all the same. Your checkout is a read-only file system for that
  program only, never for you.
- Rule 4 holds for each change that occurs there in the meantime. The change goes back to the agent, up to
  `review.rounds` times. Then the task waits for you. Nothing is built until the change is gone.
- A task in a repo always gets a worktree of its own when you first delegate it.

A folder that is not a repo has nothing to guard. There, the plan is written and approved the same way, with no mount.

## A plan and several attempts

The plan belongs to the task, not to an agent or a worktree. It is one file, written one time and approved one time.
With [several attempts](attempts.md) (`attempts.by`):

1. The task is not split when it starts. One writer makes the plan (the working agent, or the planner). The plan is
   approved one time.
2. Only then is the task split. Every attempt gets that same plan in its first prompt, and builds it.
3. The worktree and tab where the plan was written become attempt A's. Each other attempt gets its own beside it.
4. From then on, all is as for any task done several times: the check, the pick, and one review of the chosen attempt.
   That review checks the attempt against the task and the plan.

## Related pages

- [Get a plan before the build](../how-to/get-a-plan-first.md)
- [Plan settings](../reference/plan-settings.md)
- [Several attempts](attempts.md)
- [The gate](gate.md)
- [Limits](limits.md)
- [Use a spec tool](../how-to/use-spec-tools.md)
