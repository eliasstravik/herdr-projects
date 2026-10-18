# How automations work

This page tells what an automation is, who fires it, and why its rules are as they are. For the steps, see
[make an automation](../how-to/make-an-automation.md). For the facts, see [automations](../reference/automations.md).

## A saved task plus a trigger

**An automation is a saved task plus a trigger.** Each time that the trigger fires, the board adds an ordinary task.
Then it puts an agent on that task at once.

An automation does nothing more. It does not try to reach a goal by itself. It does not wake anyone: the board sends
nothing to the Coordinator.

The trigger is a schedule or an event on the board. There is no webhook trigger. To watch something outside the board,
you use a schedule, for example an automation that looks at GitHub every ten minutes.

After the firing, the new task is a task like all other tasks. Its own worktree, the check, the review, the gate and
the merge all follow its settings ([how work gets done](how-work-gets-done.md)).

## Why missed ticks are not caught up

A schedule skips missed ticks and never catches them up. If the machine or the keeper was off during a tick, that tick
does not fire later. Thus, after a week off, there is no flood of tasks. A tick that the board finds more than five
minutes late counts as missed. One line in the firings tells you so, one time for the whole gap. Then the schedule
continues from now.

## Why an event fires once, and only after the wait starts

Only events after the automation started to wait count. The wait starts when you make the automation, when you last
turned it on, or when you gave it this trigger. A state that was already true fires nothing. Also, no event after that
time is missed, even if nothing looked for some time. Each event fires one time.

## Why the event block is data

The new task gets a block under the prompt that tells what happened. The title in that block is the own words of the
other task. Thus it is data about what happened, never an instruction to the agent.

The rules of a task that an automation made tell it so. Its description is the prompt of the automation: your own
standing words. These words count as an instruction from you ([roles](roles.md)).

## Why there are guards

- **One at a time for each automation.** A firing does not start a second task while the task of the previous firing
  still has an agent, the check or a reviewer on it. A task that only waits for you does not stop the next firing.
- **No loops.** A task that an automation made remembers which automation made it. Each task that it adds remembers
  this too. Events of these tasks never fire that same automation. A task fired by the task of a different automation
  carries both automations. Thus two automations cannot fire each other with no end.
- **A runaway stops itself.** An automation makes a maximum of 10 tasks in an hour. The next firing turns it off. This
  stops an automation that fires too often.
- **The agent limit stays.** `board.agents` applies to the tasks of automations too. A new task waits for a free slot.

## Why its settings are the settings of a task

An automation holds only the work settings that a single task can hold. It saves only the values that are different.
At a firing, they become the own settings of the new task. The board finds all other values by the one rule, as for a
task that you typed ([settings](settings.md)). Thus automations have no place of their own in that order, and no
setting exists only for automations.

## Who fires them

The keeper and an open board fire automations. They use the same code, once a second, one at a time on the machine. A
lock in the data folder gives the turn. If one of them finds the lock in use, it skips. Thus automations work when the
board is closed.

An automation belongs to the machine where you made it. Nothing moves between machines. The board shows the
automations of each machine in the part of the list of that machine.

## Related pages

- [Make an automation](../how-to/make-an-automation.md)
- [Automations reference](../reference/automations.md)
- [Automations on the board](../reference/automations-board.md)
- [How work gets done](how-work-gets-done.md)
- [Settings](settings.md)
- [Other machines](other-machines.md)
- [The Coordinator](coordinator.md)
