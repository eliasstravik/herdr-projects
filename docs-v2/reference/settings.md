# Settings

This page lists every setting, the places where you can set it, and the values that it takes. To learn how the board
finds the value of a setting, read [how settings work](../explanation/settings.md).

Every choice about how work occurs is a setting. You can set a setting for everything, for one repo, or for one task:

```
herdr-projects set merge.by itself                       you: every task
herdr-projects set worker.agent codex --repo my-app      your entry for one repo
herdr-projects set review.rounds 5 --task 12             one task
herdr-projects show 12                                   what task 12 runs on, and where each value comes from
```

On the board, `Ctrl+E` opens the settings of the selected task. You can also ask the Coordinator. Each setting and its
meaning is in [the list](#the-list).

## Two kinds of setting

- **Work settings** control who works, where, how the work is checked, and how it lands. You can set them for a task,
  for a repo, or for you.
- **Board settings** control the board itself, the Coordinator and the Watcher. They are all the settings under
  `board.`, `coordinator.` and `watcher.`. They are for you only. The board refuses a board setting for a task or for a
  repo. The answer tells you how to set it for yourself.

One setting is neither kind: `repo.key`, a fact about one repo.

## The order of the places

| Kind of setting | Order, nearest first |
|---|---|
| A work setting | this task → your entry for this repo → you → the repo's own file → built in |
| A board setting | you → built in |
| `repo.key` | your entry for the repo → the repo's own file → built in |

The nearest answer wins. The rules for each kind:

- The board refuses `repo.key` on a task and for you.
- Every repo and folder has a `repo.key`. When the board first sees a repo, it makes the key from the name of the repo.
  It saves the key as your entry for that repo: `"repos": { "gtm-skills": { "repo": { "key": "SKL" } } }`. More
  information: [task numbers and repo keys](task-numbers.md).
- A task in no repo skips the two repo places. `workspace.*`, `check.*` and `merge.*` do not apply to it, and `show`
  tells you this.

## The places

| Place | Where it is |
|---|---|
| You | `~/.herdr-projects/settings/config.json` ([the home folder](home-folder.md)) |
| Your entry for a repo | The `repos` section of that same file, with the name of the repo on the board as its key |
| This task | Saved on the task. Only the settings that are different are saved. |
| The repo's own file | `.herdr-projects.json`, committed in that repo |

- A task that an [automation](automations.md) made starts with the settings of the automation in the place "this
  task". An automation is not a place of its own.
- The repo's own file can hold only seven settings: `research.only`, `workspace.setup`, `workspace.include`,
  `workspace.teardown`, `check.command`, `check.timeout` and `repo.key`.
- The board refuses all other settings in the repo's own file. It tells you so on the task: a `Note` in the task panel,
  and in `show <id>`.
- The board reads the repo's own file from the commit that the worktree started from.

An example of your `config.json`:

```json
{
  "review": { "rounds": 5, "mustFix": "critical" },
  "merge": { "by": "itself" },
  "board": { "agents": 10 },
  "repos": { "my-app": { "worker": { "agent": "codex/gpt-6.1-sol" }, "merge": { "by": "you" } } }
}
```

An example of the `.herdr-projects.json` file of a repo:

```json
{ "workspace": { "setup": "npm ci", "include": [".env"], "teardown": "docker compose down" },
  "check": { "command": "npm test", "timeout": "20m" } }
```

If a file that you edit by hand has a mistake, the board reports it with the name of the setting. It shows the report
in `show` and in the `Note` of the task panel. For that setting, the board uses the next place down. The board
continues to run.

## Values

| Kind of value | Rules |
|---|---|
| A switch | `true` or `false`, as real booleans in the file. When you type it, `yes`, `no`, `on` and `off` also work. The board refuses all other values. |
| A choice | One word from a list. "Off" is always the word `none`. |
| A time | A number with a unit: `60s`, `15m`, `2h`. |
| An agent | `claude`, `codex`, `pi` or a different program that Herdr can start. You can add a model after a slash (`codex/gpt-6.1-sol`). |
| A list of agents | Agents with commas between them, in the order that the board tries them (`codex, claude/claude-opus-5-5`), or `none`. In the file, it is a list (`["codex", "pi"]`). `worker.fallback` and `reviewer.fallback` use this kind. |
| Instructions | Your own words. `worker.instructions` and `reviewer.instructions` use this kind. |

- The model goes with the program. Thus the board never gives a Claude model to Codex. The board refuses a model that
  the program does not take, and tells you the models that it takes ([models](models.md)).
- **`none` is a real answer.** It is not the same as unset. For example, a task can cancel the setup of its repo with
  `set workspace.setup none --task <id>`.
- **Instructions add up** across the places, broadest first. The board tells the agent that the later instructions win.
  `none` in a place removes the instructions of the broader places. All other settings, lists too, take the nearest
  answer only.
- Your instructions come after the fixed rules of the board. These rules do not change: no push, the closing note, and
  the form of the verdict of the reviewer.
- **The board refuses an unknown name or a wrong value.** It gives the nearest correct one as a hint.

## Commands

```
herdr-projects set <name> <value>                you
herdr-projects set <name> <value> --repo <name>  your entry for that repo
herdr-projects set <name> <value> --task <id>    one task
herdr-projects set <name> <value> --automation <name>   the tasks one automation makes (automations.md)
herdr-projects unset <name>  [--repo … | --task … | --automation …]   back to what it would be
herdr-projects show                              your settings, the changed ones marked
herdr-projects show <id>                         every value of one task, and the place it came from
herdr-projects delegate <id> --reviewer.agent claude --merge.by itself
```

`delegate` takes any work setting for that task. If the board refuses an option, nothing on the task changes. For the
automations, refer to [automations](automations.md).

On the board, `Ctrl+E` opens the settings of the selected task
([the settings page](task-panel.md#the-settings-page)):

- Each row shows the plain label of the setting first and its name last.
- The line on top tells what the selected row can be.
- `Enter` changes the row. `Ctrl+U` puts the row back.
- `*` marks the rows that are set for this task only. All other rows are plain. `show <id>` tells where their values
  come from.
- The page does not offer a setting that cannot apply to the task. An example is fast mode for a program that has no
  fast mode.

## The list

Note: This table is a snapshot of the table that the code makes in [`docs/settings.md`](../../docs/settings.md). If the
two are different, `docs/settings.md` is correct. The links in the table go to the pages in `docs-v2/` that hold the same topics.

| Setting | Built in | Can be said by | Means |
|---|---|---|---|
| `worker.agent` | `claude` | task, repo entry, you | the working agent's program: one of `claude`, `codex`, `pi`, `agy`, `copilot`, `cursor`, `devin`, `droid`, `grok`, `hermes`, `kilo`, `kimi`, `letta`, `mastracode`, `omp`, `opencode`, `qodercli`, `qwen`, with an optional model after a slash (`codex/gpt-6.1-sol`, `agy/gemini-3-pro`). The model travels with the program, so one program's model is never handed to another; a name the program does not take is refused with the ones it does (`herdr-projects models` lists them, [models](models.md)). A program that cannot do this role, or take a model or an effort, is refused with the reason ([agents](agent-programs.md)) |
| `worker.effort` | `default` | task, repo entry, you | the working agent's effort level: one its program accepts (checked against `worker.agent`), or `default` for the program's own |
| `worker.fast` | `false` | task, repo entry, you | the working agent in its program's fast mode: a faster service tier that uses more of your allowance. Only Codex has one; for another program it does nothing, and `show` says so |
| `worker.access` | `full` | task, repo entry, you | what the working agent may do without asking: `full` or `ask`. The board turns it into each program's own flags. `ask` is refused for a program that cannot ask (pi) |
| `worker.instructions` | `none` | task, repo entry, you | your own words for the working agent, added after the board's own fixed rules. They add up across the places, broadest first, and the agent is told that later ones win |
| `worker.fallback` | `none` | task, repo entry, you | who takes over when the working agent cannot go on: a list of agents tried in order (`codex, claude/claude-opus-5-5`; a model after a slash, as in `worker.agent`), or `none`. It takes over in the same worktree, with the task, what was done so far and the open findings, when the agent's provider is down (after the board's own tries), the agent is out of allowance or logged out, or its work still fails the review or the check after `review.rounds` rounds. With the list used up the task waits for you. Nobody extra runs until then. More: [the fallback agent](fallback-settings.md) |
| `reviewer.agent` | `codex` | task, repo entry, you | the reviewer's program: one of `claude`, `codex`, `pi`, `agy`, `copilot`, `cursor`, `devin`, `droid`, `grok`, `hermes`, `kilo`, `kimi`, `letta`, `mastracode`, `omp`, `opencode`, `qodercli`, `qwen`, with an optional model after a slash (`codex/gpt-6.1-sol`, `agy/gemini-3-pro`). The model travels with the program, so one program's model is never handed to another; a name the program does not take is refused with the ones it does (`herdr-projects models` lists them, [models](models.md)). A program that cannot do this role, or take a model or an effort, is refused with the reason ([agents](agent-programs.md)). Not installed here: the worker's program reviews, on its own model, and the task says so |
| `reviewer.effort` | `default` | task, repo entry, you | the reviewer's effort level: one its program accepts (checked against `reviewer.agent`), or `default` for the program's own |
| `reviewer.fast` | `true` | task, repo entry, you | the reviewer in its program's fast mode: a faster service tier that uses more of your allowance. Only Codex has one; for another program it does nothing, and `show` says so |
| `reviewer.access` | `full` | task, repo entry, you | what the reviewer may do without asking: `full` or `ask`. The board turns it into each program's own flags. `ask` is refused for a program that cannot ask (pi) |
| `reviewer.instructions` | `none` | task, repo entry, you | your own words for the reviewer, added after the board's own fixed rules. They add up across the places, broadest first, and the agent is told that later ones win |
| `reviewer.fallback` | `none` | task, repo entry, you | who takes over when the reviewer cannot go on: a list of agents tried in order (`codex, claude/claude-opus-5-5`; a model after a slash, as in `reviewer.agent`), or `none`. It reviews in its place when the reviewer's provider is down (after the board's own tries), the reviewer is out of allowance or logged out, or its program is not installed here; it then reviews the later rounds of that task too. With the list used up the review fails as before. Nobody extra runs until then. More: [the fallback agent](fallback-settings.md) |
| `coordinator.agent` | `claude` | you only | the Coordinator, the agent you talk to in the chat's program: one of `claude`, `codex`, `pi`, `agy`, `copilot`, `cursor`, `devin`, `droid`, `grok`, `hermes`, `kilo`, `kimi`, `letta`, `mastracode`, `omp`, `opencode`, `qodercli`, `qwen`, with an optional model after a slash (`codex/gpt-6.1-sol`, `agy/gemini-3-pro`). The model travels with the program, so one program's model is never handed to another; a name the program does not take is refused with the ones it does (`herdr-projects models` lists them, [models](models.md)). A program that cannot do this role, or take a model or an effort, is refused with the reason ([agents](agent-programs.md)). `none`: no Coordinator |
| `coordinator.effort` | `default` | you only | the Coordinator, the agent you talk to in the chat's effort level: one its program accepts (checked against `coordinator.agent`), or `default` for the program's own |
| `coordinator.fast` | `false` | you only | the Coordinator, the agent you talk to in the chat in its program's fast mode: a faster service tier that uses more of your allowance. Only Codex has one; for another program it does nothing, and `show` says so |
| `coordinator.access` | `full` | you only | what the Coordinator, the agent you talk to in the chat may do without asking: `full` or `ask`. The board turns it into each program's own flags. `ask` is refused for a program that cannot ask (pi) |
| `coordinator.instructions` | `none` | you only | your own words for the Coordinator, the agent you talk to in the chat, added after the board's own fixed rules |
| `watcher.agent` | `claude/claude-haiku-5-5` | you only | the Watcher: a small model behind the "doing now" line, the progress guess and the minutes left. `none`: none of the three. A program with no headless one-shot is refused ([agents](agent-programs.md)). More: [the Watcher](../explanation/watcher.md) |
| `watcher.every` | `60s` | you only | the least time between two looks of the Watcher at one working agent. One hidden model call covers every agent whose look is due, one call runs at a time on the machine, a screen that has not changed costs none, and a busy machine is looked at up to five times less often |
| `research.only` | `false` | task, repo entry, you, repo file | research only: the task's agent changes nothing in the repo and its result is a report, with a private link. Nothing is committed, pushed, opened as a pull request or merged for it, and its row says `[research]`. Read when the agent starts: a change counts from the task's next start, and turning it on is refused once the agent has changed the repo or a pull request is open. `--research` and `--no-research` on `add` and `delegate` are short for it. More: [reports and research-only tasks](research-tasks.md) |
| `workspace.from` | `main` | task, repo entry, you | where a worktree starts, and where its work lands: `main` (the remote's main branch, fetched first), `here` (the commit your checkout is on) or a branch's name at the remote (`develop`). A named branch is fetched from origin on every start, and one origin does not have is refused; the task keeps up with that branch, its pull request merges into it and GitHub's checks are those of a pull request into it. It needs `merge.pr` on: a local merge of a named branch is refused for now. Read when the worktree is made. More: [how work gets done](../explanation/how-work-gets-done.md) |
| `workspace.branch` | `work/{n}-{slug}` | task, repo entry, you | the name of a task's branch: `{n}` is the task's number (an attempt's letter included: `7b`), `{slug}` its title in small words with dashes, `{key}` its repo's key and number (`KIC-123`). It needs `{n}` or `{key}`. git must take it as a branch name (`git check-ref-format`); one it would refuse is refused with the nearest one it takes. A name already taken gets `-2`, `-3` and so on after it. Read when the worktree is made. More: [how work gets done](../explanation/how-work-gets-done.md) |
| `workspace.setup` | `none` | task, repo entry, you, repo file | a command run in the fresh worktree before the agent starts |
| `workspace.include` | `none` | task, repo entry, you, repo file | ignored files a fresh worktree lacks and gets a copy of (`.env`); `.worktreeinclude` in the repo names more |
| `workspace.teardown` | `none` | task, repo entry, you, repo file | a command run in the worktree before it is removed |
| `workspace.cleanup` | `true` | task, repo entry, you | remove a done task's space (so it leaves herdr's sidebar), worktree and branch. More: [when a task is done](../explanation/how-work-gets-done-clean-up.md) |
| `attempts.by` | `1` | task, repo entry, you | how many attempts are made at once, and by whom: a count (`3`), or a list of agents (`claude, codex, pi`; a model after a slash; a name may repeat). A count means that many of `worker.agent`. Each attempt has its own worktree, branch and tab, runs the check, and holds one of `board.agents`' slots; one that finds no free slot waits its turn. At most 5. `1`: the task is done once, as ever. More: [several attempts](attempts-settings.md) |
| `attempts.pick` | `you` | task, repo entry, you | who chooses among the attempts that passed the check. `you`: the task waits as "pick one", the attempts side by side, with the reviewer's comparison when there is a reviewer. `reviewer`: the reviewer compares them, names one and says why, and that one goes on by itself. Only the chosen attempt is reviewed in full, pushed and merged. More: [several attempts](attempts-settings.md) |
| `plan.approve` | `none` | task, repo entry, you | whether the work starts with a plan, and who says yes to it; nothing in the repo is changed before that yes. `none`: no plan. `you`: the agent writes its plan and the task waits as `plan ready` for your yes; a reply sends the plan back with your words, and it is written again. `reviewer`: the reviewer checks the plan against the task's description and the build goes on, or the plan goes back to its writer with the reviewer's points. Read when the agent starts and again when the plan is written. More: [a plan first](plan-settings.md) |
| `plan.agent` | `none` | task, repo entry, you | who writes the plan, where there is one (`plan.approve`). `none`: the working agent, in its program's own plan mode where it has one (Claude Code). A program, with an optional model after a slash (`claude/claude-opus-5-5`): that one writes the plan into the task's folder, in a tab of its own (`Planner`), and the working agent builds from it. More: [a plan first](plan-settings.md) |
| `check.command` | `none` | task, repo entry, you, repo file | the test command: the work only counts as done when it passes |
| `check.again` | `none` | task, repo entry, you, repo file | a shorter check run instead of `check.command` on a later round of a task (sent back by a review, a failed check or GitHub's checks), once the full check has passed on its branch, and only while GitHub runs its checks before the merge (`merge.checks` is `github`, the pull request is open and a workflow of the branch runs for it). It is told the commit the full check passed on, in `HERDR_PROJECTS_CHECKED`, to run only the tests the change since then reaches. `none`: the full check every time. More: [shorter checks on fix rounds](../explanation/shorter-checks.md) |
| `check.timeout` | `15m` | task, repo entry, you, repo file | how long the check may run before it is stopped and counts as failed |
| `review.when` | `repos` | task, repo entry, you | what gets a review when finished: `repos` (work in a repo), `always` (work outside one too) or `never` |
| `review.mustFix` | `major` | task, repo entry, you | a finding that bad or worse fails the review. More: [the gate](gate-rules.md) |
| `review.rounds` | `3` | task, repo entry, you | how many times work goes back to the agent by itself, for any one reason: findings, a failed check, a conflict with main, pull request feedback. `0`: nothing goes back by itself, and a failed review waits for you |
| `review.timeout` | `20m` | task, repo entry, you | how long one review may take, and one comparison of attempts: a time (`45m`, `1h`) or `none`. Read when the review starts. Reached: the reviewer is stopped and the review fails (`review timed out after 20m (review.timeout)`), and what happens next is as for any failed review. Not from the repo's own file, so the code under review cannot change it. Unlike `limit.time`, which stops any run and waits for you. More: [the gate](gate-rules.md) |
| `review.afterChat` | `60s` | task, repo entry, you | how long a chat with the agent of a task whose work was reviewed (under review, waiting for your yes, done) must be quiet before the code it changed is reviewed: no new message from you and the agent at rest for this long, or you left the chat (another task on the board, another space in herdr). Then one review runs on the final state. A chat that changed no code reviews nothing: the task goes back to where it was, with the same review. More: [how work gets done](../explanation/how-work-gets-done.md) |
| `review.evidence` | `true` | task, repo entry, you | the reviewer brings evidence: screenshots, a recording, a link, test output |
| `limit.time` | `none` | task, repo entry, you | the longest one run of an agent may take, the working agent's or the reviewer's: a time (`2h`, `45m`) or `none`. Read when a run starts: set while one is under way, it counts from the next. Reached: the agent is stopped, its work is kept as it is, and the task waits for you under "needs you" (`stopped at its 2h limit`). Nothing is merged. More: [limits](limits.md) |
| `limit.tokens` | `none` | task, repo entry, you | the most tokens a task may use, all its agents, attempts and reviews together, what is read again from the provider's cache included: a count (`2m`, `500k`) or `none`. Counted at the end of each turn, so one turn can pass it. Reached: as `limit.time` (`stopped at 2m tokens`). For a program whose tokens cannot be counted it does nothing, and `show <id>` says so. More: [limits](limits.md) |
| `merge.by` | `you` | task, repo entry, you | `you`: finished work waits for your yes. `itself`: it merges the moment it may: its review passed, its checks pass, nothing conflicts. Read right before the merge, so a change counts for tasks under way. More: [the gate](../how-to/turn-on-auto-merge.md) |
| `merge.method` | `squash` | task, repo entry, you | how approved work lands |
| `merge.pr` | `true` | task, repo entry, you | push the branch and open a draft pull request after the check passes |
| `merge.checks` | `github` | task, repo entry, you | whether a pull request waits for its checks on GitHub before it merges, by itself or on your yes. `github`: still running, it waits for them, at most `check.timeout`, then it waits for you with that reason; failed, nothing merges and the failure goes back to the agent with each failing check's name and the end of its log, as a failed check does, within `review.rounds`, then it waits for you (`approve <id> --anyway` merges it all the same); passed, and nothing expected still out, it merges: each workflow of the branch that runs for the pull request (its events, branches and paths, as GitHub reads them) must have its run on the commit through, every job and matrix member of it, each check the branch requires must be listed, and a check gone from the list is still waited for; with nothing listed at all, only a repo where no outside CI reports merges. A fine-grained token (which cannot have the Checks permission) is enough: the checks are then read from Actions and Commit statuses, and what it cannot read (another app's check runs) is said on the task, not asked as a login. `none`: GitHub's checks are not looked at. More: [how work gets done](../explanation/how-work-gets-done.md) |
| `repo.key` | `none` | repo entry, repo file | the short key of this repo's tasks, as `SKL` for gtm-skills: 2 to 5 letters or digits, the first a letter, kept in capitals. Every repo and folder has one: the board makes it from the name the moment it first sees the repo (`SKL` for gtm-skills, `HOME` for the home folder, `ROOT` for `/`) and saves it as your entry for the repo, so `none` lasts only until then. A task's tag on the board reads `HSW-134`, and `HSW-134` (in any letter case) names the task wherever its id `herdr-projects/134` does. Set it to change it: the old key still names the tasks. A key another repo already has is refused; `unset` goes back to the repo's file's key, or to the one made from the name. More: [task numbers and keys](task-numbers.md) |
| `board.agents` | `4` | you only | how many agents work at once |
| `board.machines` | `auto` | you only | other machines: `auto` (herdr's saved machines), names (`exedev@vm-a, vm-b`) or `none` |
| `board.open` | `tab` | you only | what the board key opens. `tab`: the board in a herdr tab of its own, which stays when you go into an agent; pressed on the board itself, the key takes you back to where you came from. `popup`: the board as herdr's popup, a framed window in the middle of herdr's screen, over whatever is there (in a window under 110 columns or 30 rows it takes the whole pane: a margin would cost half the board); the key again or `Esc` closes it, as its hint line says (`esc close`), it closes when you go into an agent, and it is never left behind on a tab. More: [the board](board-header-and-list.md) |
| `board.notify` | `none` | you only | when a task needs you, is ready to approve, or failed: `bell` rings the terminal bell, a command is run with `HERDR_PROJECTS_ID`, `HERDR_PROJECTS_TITLE` and `HERDR_PROJECTS_EVENT` (`needs you`, `ready to approve` or `failed`) in its environment, `none` does neither |
| `board.keys` | `none` | you only | your own chord for some of the board's keys, when the one it comes with does not reach it (a window manager or a terminal that takes `Alt`): `name=chord` pairs, each name a key's from the tables in [keys](keys.md#all-the-keys), such as `chatMessage=ctrl+o, chatScrollUp=ctrl+up, chatScrollDown=ctrl+down`, or `none`. The key then answers to your chord only, and the `?` menu, the hint line and the board's messages name it so. A chord another key of the board has, or the key that opens the board (or herdr's prefix, when that key starts with it), is refused with the key it would clash with: move that one too, in the same value. A plain character only for a key that is one already (`talk`, `menu`, `sayYes`, `sayNo`): any other is typed into the text box. The text box's own keys cannot be moved. Yours only: never per repo, and never from a repo's file. More: [your own keys](keys.md#your-own-keys) |
| `board.webUi` | `auto` | you only | the board inside the herdr web UI. `auto`: when the web UI plugin (`devswha.herdr-web-ui`) is installed, a small front door of the board's own runs in front of it and adds a Boards section to its sidebar; without that plugin nothing runs. `none`: never. More: [the board in the web UI](web-ui.md) |
| `board.theme` | `herdr` | you only | the board's colours. `herdr`: herdr's own theme, read from this machine's `config.toml` and resolved as herdr does, so with no theme set there it is herdr's default (Catppuccin); `dark` or `light` picks that theme's dark or light one by hand; the name of one of herdr's themes (`nord`, `gruvbox`, … and `terminal` for the terminal's own colours) is that theme whatever the config says, for a herdr opened from another machine whose config names another one. More: [colours](colours.md) |

## Other machines

The work settings of a task go with the task. The machine that you are on finds your settings for the task: you, and
your entry for its repo. It sends them with the command that starts the agent. Each machine keeps its own board
settings ([other machines](../explanation/other-machines.md)).

## What is fixed

These items are not settings:

- A task in a repo always works in a git worktree of its own, never in your checkout.
- The board always keeps the size of its own pane.
- The board shares evidence links privately on your tailnet when Tailscale is installed. When Tailscale is not
  installed, it shares them on 127.0.0.1.
- The board always writes the how-to for agents and the trust mark for the repo of a task.
- A conflict with main goes back to the agent of the task automatically (within `review.rounds`).
- A passed review stays valid after a clean catch-up with main, if the check passes.
- Only your own comments on a pull request wake its agent. These are the comments of the GitHub user who is logged in.

## Settings from the old names

The first run of this version converts your `config.json` and the settings of each task automatically, one time. It
keeps the old file beside the new file as `config.old.json`. A task that is in progress keeps the choices that it had.

The data is in `~/.herdr-projects/data`. `HERDR_PROJECTS_DATA_DIR` changes this folder. There is one folder for each
task, with `task.json`, `inbox.jsonl` and `artifacts/`. For all the data that the board keeps, refer to
[the home folder](home-folder.md).

## Related pages

- [How settings work](../explanation/settings.md)
- [Change a setting](../how-to/change-a-setting.md)
- [The task panel](task-panel.md)
- [Models](models.md)
- [Commands](commands.md)
- [Task numbers](task-numbers.md)
- [The home folder](home-folder.md)
