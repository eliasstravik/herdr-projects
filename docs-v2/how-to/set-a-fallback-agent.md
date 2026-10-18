# Set a fallback agent

This page tells how to make another agent take over when an agent cannot go on (out of allowance, logged out, its
provider down). For how the hand-over works, read [the fallback agent](../explanation/fallback.md).

**Note:** Without a fallback list, nothing changes: the built-in value is `none`. A fallback list costs nothing while
all goes well, because no extra agent runs until a hand-over.

## Set a fallback for the working agent

1. Choose the agents, in the order that the board must try them. Write a model after a slash
   (`claude/claude-opus-5-5`).
2. Set the list for each task of yours:

   ```
   herdr-projects set worker.fallback "codex, claude/claude-opus-5-5"   for every task of yours
   ```

3. If you want a different list for one repo, set it with `--repo`:

   ```
   herdr-projects set worker.fallback codex --repo my-app               for one repo
   ```

4. If one task must not use a fallback, set `none` with `--task`:

   ```
   herdr-projects set worker.fallback none --task 12                    not for this one
   ```

**Note:** If a name in the list is wrong, the board refuses the full list and changes nothing. It gives the nearest
correct name.

## Set a fallback for one delegation

1. Give the lists to `delegate`:

   ```
   herdr-projects delegate 12 --worker.fallback pi --reviewer.fallback claude
   ```

`add --delegate` takes the same flags.

## Set a fallback for the reviewer

1. Set `reviewer.fallback` in the same way as `worker.fallback`. For example:
   `herdr-projects set reviewer.fallback claude`.

## Set a fallback on a task that is under way

The board reads the setting at the time of the hand-over.

1. Set the list on the task: `herdr-projects set worker.fallback codex --task 12`.

The list applies from the next time that the agent of the task cannot go on.

## Remove a fallback

1. Remove the value with `unset`: `herdr-projects unset worker.fallback`.

## See which list a task uses

1. Run `herdr-projects show <id>`. It lists each setting, with where its value came from.
2. Or open the settings page of the board with `Ctrl+E`. Find the rows `Agent's fallback` and `Reviewer's fallback`.

## See who took over

1. Look at the row of the task. While the new agent starts, it says the hand-over in plain words, for example
   `codex took over from claude: out of allowance until 17:00`.
2. Open the task panel. Find `Took over` under `Status`.
3. To see each hand-over, run `herdr-projects details <id>`.

## Related pages

- [The fallback agent](../explanation/fallback.md)
- [The fallback settings](../reference/fallback-settings.md)
- [Agent programs](../reference/agent-programs.md)
- [Change a setting](change-a-setting.md)
- [A turn cut off by the provider](../explanation/provider-errors.md)
