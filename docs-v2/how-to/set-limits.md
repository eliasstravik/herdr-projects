# Set limits on time and tokens

Use this page to put a limit on what a task can cost, and to continue a task that stopped at its limit.

**Note:** When a task gets to a limit, the board stops its agent and keeps its work. The task waits for you. Nothing is
merged. Both limits are `none` until you set them, so a board that sets neither works as before.

## Set a limit

1. Choose the scope, and run the command for it:

   ```
   herdr-projects set limit.time 2h                      # for every task of yours
   herdr-projects set limit.tokens 2m --repo my-app      # for one repo
   herdr-projects set limit.time 45m --task my-app/12    # for one task, also while it is under way
   ```

2. Optional: to set limits when you delegate or add a task, give them as flags:

   ```
   herdr-projects delegate 12 --limit.time 2h --limit.tokens 5m
   herdr-projects add "Tidy the docs" --limit.time 1h     # with or without --delegate
   ```

3. Optional: to set a limit for the tasks of an automation, give `--automation`:

   ```
   herdr-projects set limit.tokens 1m --automation "Nightly look"
   ```

You can also tell the Coordinator: "never more than two hours on this one", "at most two million tokens a task".

**Note:** A request reads the whole conversation again, so the number of tokens grows fast. Fifty steps at 100k of
context are five million tokens, and a review is easily half a million.
`show <id>` and the task panel say what a task has used.

**Note:** If you write a value that the board does not accept, the board changes nothing. It names the nearest correct
value: for `2 hours`, it says "Did you mean 2h?".

## Remove a limit

1. Run `unset` for the setting and the scope:

   ```
   herdr-projects unset limit.time --task my-app/12
   ```

## Continue a task that stopped at a limit

The task is under **needs you**. Its line says, for example, `stopped at its 2h limit` or `stopped at 2m tokens`. Its
task panel has a part "Stopped at a limit".

Choose one of these:

| You want | Do |
|---|---|
| it to continue | `Ctrl+R` on the task, or `herdr-projects retry <id>`, or reply to its agent with your own words |
| to give it more | set a higher limit (or `none`): `herdr-projects set limit.time 4h --task <id>` |
| to take the work as it is | `Ctrl+L`, or `herdr-projects finish <id>` |

On the web board, "Send back" is the button for continue, and "Yes" is the button for "as it is".

- **Continue:** the task has its limit one more time. The agent continues from where it stopped. Its run starts at
  zero. The tokens count on from where they were.
- **A higher limit:** the task continues by itself within a couple of seconds, and nobody is asked. The count goes on:
  a run stopped at 2h and given 4h has two hours left.
- **As it is:** the work of the stopped agent goes through the check and the review, like any finished turn. Then
  `merge.by` decides how it lands.

What occurs in each case, in full, is in [limits](../reference/limits.md#carrying-on).

## Stop a task at a limit yourself

1. Press `Ctrl+X`, or run `herdr-projects stop <id>`.

   The task loses the limit's mark. It is then simply stopped.

## Change limits on the web board

1. On the task, click **Settings**.

   The task's settings page opens. It shows every work setting, the two limits among them.

2. Change the row of the limit.

## Related pages

- [Limits](../reference/limits.md)
- [How limits work](../explanation/limits.md)
- [Change a setting](change-a-setting.md)
- [Run several attempts](run-several-attempts.md)
- [Settings](../reference/settings.md)
