# How settings work

This page tells how the board finds the value of a setting, when it reads the value, and why the repo's own file can
hold only a few settings.

## The nearest answer wins

Every setting obeys one rule: the nearest answer wins. For a work setting, the board looks in this order:

**this task → your entry for this repo → you → the repo's own file → built in.**

The first place that has an answer gives the value. One function does this lookup for every setting
(`crates/core/src/store/settings/lookup.rs`). No setting has a lookup of its own. Thus every setting obeys the same
rule, and you can always find where a value comes from with `herdr-projects show <id>`.

A board setting is for you only, because it controls the board, the Coordinator or the Watcher, not one task.
`repo.key` is a fact about one repo. Thus the board looks for it only in your entry for the repo, then in the repo's own
file, then in the built-in value.

All the places are in [settings](../reference/settings.md#the-places).

## When the board reads a setting

The board reads a setting when its step starts. It never changes a step that is in progress:

- It reads the settings of the agent when the agent starts next. This includes whether the task is research only.
- It reads the settings of the review when the next review starts.
- It reads the settings of the merge immediately before the merge.

Thus a change has an effect on the next step, not on the step that runs now.

## Why the repo's own file is limited

The repo's own file, `.herdr-projects.json`, can hold only seven settings: `research.only`, `workspace.setup`,
`workspace.include`, `workspace.teardown`, `check.command`, `check.timeout` and `repo.key`.

The board reads this file from the commit that the worktree started from. It does not read the file from the work of
the agent. Thus code that an agent writes can never make a guard weaker. It cannot turn on the merge by itself, remove
the review, or change the rules that check its own branch. For the same reason, `review.timeout` cannot come from the
repo's own file: the code under review must not change it.

## Instructions add up, other settings replace

`worker.instructions` and `reviewer.instructions` add up across the places, broadest first. The board tells the agent
that the later instructions win. If a place says `none`, the board removes the instructions of the broader places.

All other settings, lists too, take the value of the nearest place only. `none` is a real answer, not the same as
unset. Thus a task can cancel the setup of its repo with `set workspace.setup none --task <id>`.

## A mistake in a file

If a file that you edit by hand has a wrong value, the board does not stop. It reports the mistake with the name of
the setting, in `show` and in the `Note` of the task panel. For that setting only, it uses the next place down.

## Settings on other machines

The work settings of a task go with the task. The machine that you are on finds your settings for the task and sends
them with the command that starts the agent. Thus a task on another machine runs with the same choices. Board settings
stay on each machine.

## Related pages

- [Settings](../reference/settings.md)
- [Change a setting](../how-to/change-a-setting.md)
- [Other machines](other-machines.md)
- [The gate](gate.md)
