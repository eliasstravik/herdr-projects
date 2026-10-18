# The fallback settings

This page gives the facts about `worker.fallback` and `reviewer.fallback`, and where the board shows a hand-over. For how
a hand-over works, read [the fallback agent](../explanation/fallback.md).

## The two settings

| Setting | What it does | Built-in value |
|---|---|---|
| `worker.fallback` | The agents that take over a task when its working agent cannot go on. | `none` |
| `reviewer.fallback` | The agents that review in place of the reviewer when the reviewer cannot. | `none` |

## The value

- The value is a list of agents. The board tries them in the order of the list.
- Write each agent as for `worker.agent`, and put commas between them: `codex, claude/claude-opus-5-5`. A model goes
  after a slash.
- The value can also be `none`. With `none`, nothing changes.
- A wrong name refuses the full list. Then the board changes nothing, and gives the nearest correct name.
- If a list names only the agent that is already on the task, the board tells you that the list does nothing.

## Examples

```
herdr-projects set worker.fallback "codex, claude/claude-opus-5-5"   for every task of yours
herdr-projects set worker.fallback codex --repo my-app               for one repo
herdr-projects set worker.fallback none --task 12                    not for this one
herdr-projects delegate 12 --worker.fallback pi --reviewer.fallback claude
```

## Where the value comes from

Both settings are work settings ([settings](settings.md)):

- The board looks for the nearest value first: the task → your entry for the repo → you → the built-in value.
- `set` sets them, and `unset` removes them.
- `delegate` and `add --delegate` take them as `--worker.fallback <list>` and `--reviewer.fallback <list>`.
- [Automations](automations.md) keep them.
- `show` and `show <id>` list them, with where each value came from.
- The settings page of the board (`Ctrl+E`) shows them as `Agent's fallback` and `Reviewer's fallback`.

**A repo's own file cannot set them.** `.herdr-projects.json` holds its seven facts and no more
([why](../explanation/fallback.md#why-a-repo-file-cannot-set-the-fallback)).

## Where you see a hand-over

| Place | What it shows |
|---|---|
| The row | The hand-over in plain words while the new agent starts, until the Watcher writes its first line about it: `codex took over from claude: out of allowance until 17:00`. The program column names the agent that is on the task now. |
| The task panel | Each hand-over under `Status`, as `Took over`. Beside the list it shows the latest hand-over. In full screen it shows all of them, each with its age. `On it` and the `Agent` row show the agent that is on the task now. A `Takes over` row under each of the two shows its list. After a reviewer took over, each review tells who made it. |
| The history | `herdr-projects details <id>` lists each hand-over (who, from whom, why, when) beside each review. It also tells who is on the task now. |
| Herdr | After a hand-over, the tab of the agent is `Agent (codex)`, and the tab of a reviewer is `Reviewer #2 (claude)`. The sidebar names the agent: `agent codex · working`, `reviewer claude · under review`. |

## The words of the reason

The board writes the reason from the words of the provider:

| Reason | When |
|---|---|
| `out of allowance until 17:00` | The provider tells when the allowance comes back (`resets 5pm`, `try again at 5:00 PM`, `try again in 2 hours`). |
| `out of allowance` | The provider does not tell when it comes back. |
| `logged out` | The program is logged out. |
| `gpt-6.1 can't be used with your ChatGPT sign-in` | The sign-in cannot use the model. |
| `claude can't use the model claude-x` | The program cannot use the model. |
| `its provider is overloaded, still after 5 tries` | The provider was down for all five tries. |
| `its work still failed review after 3 rounds` | The review still failed after the rounds. |
| `the check still failed after 4 tries` | The check still failed after the rounds. |
| Any other words | The board writes the reason as the provider wrote it. |

## Related pages

- [Set a fallback agent](../how-to/set-a-fallback-agent.md)
- [The fallback agent](../explanation/fallback.md)
- [Settings](settings.md)
- [Agent programs](agent-programs.md)
- [The task panel](task-panel.md)
