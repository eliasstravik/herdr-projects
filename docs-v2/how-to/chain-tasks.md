# Chain tasks with `--after`

Use this page to make a task wait until other tasks are done. Use it for a chain of work: change `gtm-skills` first,
then take that into `gtm-agent`, then into the copies downstream. Each step is its own task, with its own review and
merge.

**Note:** `--after` is only a condition. The task cannot start before the tasks that it names are done. To start it,
you still press `Tab` (`delegate`), as for any task.

## Add a task that waits

1. To add the task with the condition only, run `add` with `--after`:

   ```sh
   herdr-projects add "Take the new skill in" --repo gtm-agent --after gtm-skills/12             # only the condition
   ```

   The task stays under **to do**. It never starts by itself.

2. To add the task and queue it after the other task, also give `--delegate`:

   ```sh
   herdr-projects add "Take the new skill in" --repo gtm-agent --after gtm-skills/12 --delegate  # queued after it
   ```

   The task is queued. It starts by itself when all the tasks that it waits for are done.

## Delegate a task that waits

1. To delegate a task that waits for two tasks, give a list after `--after`:

   ```sh
   herdr-projects delegate gtm-agent/4 --after gtm-skills/12,gtm-skills/13
   ```

## Tell the Coordinator

You can also tell the Coordinator: "then", "once that is merged", "after HSW-12", "after ELA-3 on vm-str". These words
become an `--after` on the task that it adds or delegates. The Coordinator adds `--delegate` (it queues the task) only
when you asked it to start the task, or sent the message with `Tab`.

## Start a task that has a condition only

1. Select the task under **to do**.
2. Press `Tab`.

If the tasks that it waits for are not all done, the task is queued. If they are all done, it starts at once.

## Change what a task waits for

- To set what a task waits for, run `herdr-projects after <id> <id>[,<id>…]`.
- To see what a task waits for, run `herdr-projects after <id>`.
- To remove the wait, run `herdr-projects after <id> --clear`. A task that was waiting starts at once.
- To remove the wait while you delegate, run `herdr-projects delegate <id> --after none`.

You can also see and change the wait on the `To wait for` line of the task panel.

## Start a queued task early

1. Run `herdr-projects delegate <id> --now`.

   The task starts. Its agent is told that the tasks were not all done, and what there is of them.

On the board (and the web board), you can also press `Tab` on the queued task. The board asks first:
`Start HSW-4 "…" now?`.

## Stop a queued task

1. Run `herdr-projects stop <id>`, or press the stop key on the board.

The task goes back to **to do**. It keeps what it waits for as its condition for the next `Tab`.

## Wait for a task on another machine

The other machine must be one that the board knows (`board.machines`, or Herdr's own list of machines). It must have
this version of herdr-projects. It does not need an open board.

1. Name the task with its machine first: `vm-str:ELA-3`, `vm-str:ela/3` or `vm-str#3`.
2. Give that name to `--after`:

   ```sh
   herdr-projects delegate notably/3 --after vm-str:ELA-3
   herdr-projects add "Use the new API" --after vm-str:ELA-3,HSW-12 --delegate
   ```

The other machine must answer at that time. If it does not, the board refuses the wait and changes nothing.

## If a waiting task needs you

If a task that it waits for fails, is stopped, is removed, or is archived without being done, the queued task does not
start. It waits for you under **needs you**, with the reason on its row.

1. Read the reason on the row, for example `gtm-skills/12 was archived without being done`.
2. Get the other task going again.

   When the other task is done, the wait continues by itself.

## Related pages

- [One task after another](../explanation/after.md)
- [`--after` options](../reference/after-options.md)
- [Run long work in one repo](run-long-work-in-one-repo.md)
- [Other machines](../explanation/other-machines.md)
- [Commands](../reference/commands.md)
