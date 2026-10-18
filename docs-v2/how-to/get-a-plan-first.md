# Get a plan before the build

Use this page to make an agent write a plan before it changes the repo. You, or the reviewer, then say yes to the plan
or send it back.

**Note:** With `plan.approve` on, nothing in the repo changes until the plan gets its yes. The reason is in
[A plan first](../explanation/plan.md).

## Before you start

- The machine must be able to make a read-only mount for the plan step. On Linux, bubblewrap must be installed. On
  macOS, Herdr must not start inside a sandbox. For the details, see
  [the mount on Linux and on macOS](../reference/plan-settings.md#the-mount-on-linux-and-on-macos).
- To see if plan approval can run on this machine, run `herdr-projects doctor`. It also says why not.

## Ask for a plan on one task

1. To see the plan yourself before the build, delegate the task with `--plan.approve you`:

   ```
   herdr-projects delegate 12 --plan.approve you                                    # show me the plan first
   ```

2. To let a strong model write the plan and the usual agent build it, also give `--plan.agent`:

   ```
   herdr-projects delegate 12 --plan.approve you --plan.agent claude/claude-opus-5-5 # a strong model plans, the usual one builds
   ```

You can also tell the [Coordinator](../explanation/coordinator.md): "show me the plan first", "have Opus plan it".

## Let the reviewer check every plan in a repo

1. Set `plan.approve` to `reviewer` for the repo:

   ```
   herdr-projects set plan.approve reviewer --repo my-app                           # in this repo the reviewer checks every plan
   ```

The reviewer checks each plan against the task's description. If the plan passes, the build starts and nobody is
asked. If the plan still fails after `review.rounds` times, the task waits for your yes.

## Say yes to a plan

With `plan.approve: you`, the task waits in the state **plan ready**, under **needs you**. The row says
`Plan ready · <the plan's first words>`.

1. Select the task on the board.
2. Read the plan under **Plan** in the task panel. To read all of it, open the task panel full screen, or open the
   **`Open plan`** link.
3. To approve the plan as it is, press `Ctrl+L` (the approve key).

   The build starts.

You can also run `herdr-projects approve <id>`.

## Send a plan back with your words

1. Select the task on the board.
2. Press `Space`.
3. Type your words, and send them.

The plan goes back to its writer with your words. The writer writes it again. When the new plan is there, the task is
plan ready again.

You can also run `herdr-projects tell <id> <text>`.

## If the plan step cannot start

If the machine cannot make the read-only mount, the task does not start. It says what is missing.

1. Make the mount possible: on Linux, install bubblewrap; on macOS, start Herdr outside any sandbox.
2. If you cannot do step 1, turn the plan step off for that task:

   ```
   herdr-projects set plan.approve none --task <id>
   ```

**Note:** If you change `plan.approve` or `plan.agent` after the agent starts on the task, the change has no effect
on that run. For the full rules, see [plan settings](../reference/plan-settings.md#when-the-board-reads-them).

## If the board says the repo changed before the yes

The message is `The repo was changed before the plan was approved`, with what changed. The plan is not shown, and
the build does not start.

1. Select the task on the board.
2. To send the problem to the task's agent, press `Ctrl+R` (send it back).

   The agent undoes the change. A yes that you already gave stays. The board tries the start again when the change is
   gone.

## Related pages

- [A plan first](../explanation/plan.md)
- [Plan settings](../reference/plan-settings.md)
- [Run several attempts](run-several-attempts.md)
- [Use a spec tool](use-spec-tools.md)
- [Change a setting](change-a-setting.md)
- [Keys](../reference/keys.md)
