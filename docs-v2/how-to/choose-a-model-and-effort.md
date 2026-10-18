# Choose a model and an effort

Use this page to choose the model and the effort of an agent. The names, the levels and the checks are in
[models and effort](../reference/models.md).

## Find the names

1. To see every program, some names for each, and its effort levels, type this command:

   ```
   herdr-projects models
   ```

2. To see every name that one program takes, in the form that a setting uses, type the command with the program:

   ```
   herdr-projects models codex
   ```

## Set a model and an effort

1. Write the model after the program, with a slash. If you do not give a model, the program uses its own default.
2. Set the setting of the role in the place that you want:

   ```
   herdr-projects set worker.agent codex/gpt-6.1-sol          you, for every task
   herdr-projects set worker.effort high
   herdr-projects set reviewer.agent claude/opus --repo my-app  one repo
   herdr-projects delegate 12 --worker.agent pi/anthropic/claude-opus-5-5 --worker.effort medium   one task
   herdr-projects set worker.fallback "codex, claude/sonnet"
   herdr-projects coordinator new --agent claude/sonnet --effort low   the next Coordinator only
   ```

If the name or the level is wrong, the board refuses it when you set it. The answer shows the names or levels that
work. When several places set a value, the nearest place wins ([settings](../reference/settings.md)).

## Ask the Coordinator

You can also ask the Coordinator in your own words. Examples: "do this one with Opus on high effort", "let Codex review
it".

- The Coordinator finds the names with `herdr-projects models`. It sets them in the same way as the commands above.
- It writes only a name from that list. For example, "sol 6.1" becomes `gpt-6.1-sol`. If it cannot find the name, it
  asks you.
- If you name no model, the Coordinator gives no model. The program then uses its own default.

## If the board cannot read the list of models

The board refuses the name if the program is installed but not logged in, is offline, or its list command fails.

Do one of these two steps:

- Run the list command of the program in a terminal, and repair the problem that it shows.
- Set the agent without a model.

## If the program refuses the model while the agent works

If the program also refuses its default model, or the board gave the program no model, the task waits for you. The
task shows the reason and a name to try. The task panel shows the next step.

1. Do one of these two steps:
   - Choose a model for the task:

     ```
     herdr-projects set worker.agent codex/gpt-6.1-sol --task <id>
     ```

   - Repair the default model of the program. For Codex, this is `model` in `~/.codex/config.toml`.
2. Press `Ctrl+L` (retry). The board starts the program again on the model that is set then.

## Related pages

- [Models and effort](../reference/models.md)
- [Settings](../reference/settings.md)
- [Change a setting](change-a-setting.md)
- [Set a fallback agent](set-a-fallback-agent.md)
- [Agent programs](../reference/agent-programs.md)
