# Settings

**Every choice about how work happens is a setting, and every setting is looked up by one rule: the nearest answer
wins.** Set one for everything, for one repo, or for one task:

```
herdr-projects set merge.by itself                       you: every task
herdr-projects set worker.agent codex --repo my-app      your entry for one repo
herdr-projects set review.rounds 5 --task 12             one task
herdr-projects show 12                                   what task 12 runs on, and where each value comes from
```

On the board, `Ctrl+E` opens the selected task's settings, and on a heading your own. Or just ask the Coordinator. Every setting, with what it
means, is in [the list](#the-list) below.

## Two kinds of setting

- **Work settings**: who works, where, how it is checked, how it lands. They can be set for a task, a repo, or you.
- **Board settings**: the board itself and the Watcher (everything under `board.` and `watcher.`). They are yours
  only. Setting one on a task or for a repo is refused, and the answer says how to set it for yourself.
- **A coordinator's settings**: its program, model, effort, access and your words for it (`coordinator.`), and its
  [autopilot](autopilot.md) (`autopilot.`). They can be set for one coordinator, or for you (every coordinator), and
  `autopilot.*` also for this time only, while autopilot is on. Setting one on a task or for a repo is refused.

One setting is neither: `repo.key`, a fact about one repo.

## The rule

1. **A work setting is looked up in one order, and the nearest answer wins:**
   **this task → your entry for this repo → you → the repo's own file → built in.**
   One function does this for every setting (`crates/core/src/store/settings/lookup.rs`); no setting has a lookup of its
   own.
2. **A board setting is yours only** (above).
3. **A coordinator's setting is looked up as a work setting is, in its own order:**
   **this time → this coordinator → you → built in.** The same function does it.
   - **This coordinator** is the `coordinators` part of your `config.json`, keyed as the coordinators are:
     `{"coordinators": {"main": {…}, "machine": {…}, "projects": {"<repo>": {…}}}}`. `set`, `unset` and `show` take
     `--main-coordinator`, `--machine-coordinator` or `--project-coordinator <repo>`.
   - **This time** is the `autopilot.*` values given with `autopilot on`, kept until autopilot goes off. A
     `coordinator.*` value is never for this time.
   - `autopilot.goal` is for one coordinator only: never yours, never for this time.
   - None of them is ever read from a repo's own file, so code written by agents cannot change them.
   - `coordinator.instructions` add up: yours, then the coordinator's own. A changed `coordinator.*` counts from its next
     fresh chat.
4. **A setting is read when its step starts, and a step under way is never changed**: the agent's settings at its next
   start (whether the task is research only among them), the review's at the next review, the merge's right before the
   merge.
5. **A repo's own fact** (`repo.key`, the short key of its tasks) is set for that repo and nowhere else: **your entry for
   the repo → the repo's own file → built in.** It is refused on a task and for yourself. Every repo and folder has one:
   the board makes it from the repo's name when it first sees the repo, and saves it as your entry for that repo
   (`"repos": { "gtm-skills": { "repo": { "key": "SKL" } } }`). More: [task numbers and repo keys](task-numbers.md).
6. **A task in no repo** skips the two repo places. `workspace.*`, `check.*` and `merge.*` do not apply to it, and `show`
   says so.

## The places

- **You**: `~/.herdr-projects/settings/config.json` ([the home folder](home-folder.md)).
- **Your entry for a repo**: the `repos` section of that same file, keyed by the repo's name on the board.
- **This task**: saved on the task, only the settings that differ. A task an [automation](automations.md) made starts with
  the automation's own settings here; an automation is not a place of its own.
- **The repo's own file**: `.herdr-projects.json`, committed in that repo. It may hold these facts only: `research.only`,
  `chat.only`, `workspace.setup`, `workspace.include`, `workspace.teardown`, `check.command`, `check.timeout` and `repo.key`.
  - Anything else in it is refused, and said on the task (a `Note` in the task panel, and `show <id>`).
  - It is read from the commit the worktree started from. So code written by agents can never loosen a guard: it cannot
    switch on merging by itself, drop the review, or change the rules its own branch is checked by.

Your `config.json`, for example:

```json
{
  "review": { "rounds": 5, "mustFix": "critical" },
  "merge": { "by": "itself" },
  "board": { "agents": 10 },
  "repos": { "my-app": { "worker": { "agent": "codex/gpt-6.1-sol" }, "merge": { "by": "you" } } }
}
```

A repo's `.herdr-projects.json`:

```json
{ "workspace": { "setup": "npm ci", "include": [".env"], "teardown": "docker compose down" },
  "check": { "command": "npm test", "timeout": "20m" } }
```

A mistake in a file edited by hand is reported with the setting's name (`show`, and the task panel's `Note`). For that
one setting the next place down is used, and the board keeps running.

## Values

- **A switch** is `true` or `false`, in the file as real booleans. Typed, `yes`, `no`, `on` and `off` work too. Anything
  else is refused.
- **A choice** is one word from a list. "Off" is always the word `none`.
- **A time** carries a unit: `60s`, `15m`, `2h`.
- **An agent** is `claude`, `codex`, `pi` or another program Herdr can start, with an optional model after a slash
  (`codex/gpt-6.1-sol`). The model travels with the program, so a Claude model is never paired with Codex. A model the
  program does not take is refused, with the ones it does ([models](models.md)).
- **A list of agents** (`worker.fallback`, `reviewer.fallback`) is agents with commas between them, in the order they are
  tried (`codex, claude/claude-opus-5-5`), or `none`. In the file it is a list (`["codex", "pi"]`).
- **`none` is a real answer**, not the same as unset: a task can cancel its repo's setup with
  `set workspace.setup none --task <id>`.
- **Instructions** (`worker.instructions`, `reviewer.instructions`) add up across the places, broadest first, and the
  agent is told that later ones win. `none` in a place drops what the broader places said. Every other setting, lists
  included, is replaced by the nearest answer. Your words come after the board's own rules, which stay fixed: no pushing,
  the closing note, the shape of the reviewer's verdict.
- **An unknown name or a wrong value is refused**, with the nearest right one as a hint.

## Commands

```
herdr-projects set <name> <value>                you
herdr-projects set <name> <value> --repo <name>  your entry for that repo
herdr-projects set <name> <value> --task <id>    one task
herdr-projects set <name> <value> --automation <name>   the tasks one automation makes (automations.md)
herdr-projects set <name> <value> --main-coordinator     one coordinator's own (also --machine-coordinator,
                                                         --project-coordinator <repo>): coordinator.* and autopilot.*
herdr-projects unset <name>  [--repo … | --task … | --automation … | a coordinator's flag]   back to what it would be
herdr-projects show                              your settings, the changed ones marked
herdr-projects show <id>                         every value of one task, and the place it came from
herdr-projects delegate <id> --reviewer.agent claude --merge.by itself
```

`delegate` takes any work setting for that task. A refused option changes nothing on the task.

**On the board**, `Ctrl+E` opens the selected task's settings ([the task panel](task-panel.md)):

- each row has the setting's plain label first and its name last;
- the line on top says what the selected row may be;
- `Enter` changes it, and `Ctrl+U` puts a row back;
- `*` marks the rows set for this task only. Every other row is plain: where its value comes from is `show <id>`'s;
- what cannot apply to the task (fast mode for a program that has none) is not offered.

**Your own settings on the board**: `Ctrl+E` on this machine's heading (or on a status group's) opens the same page for
yours, what every task starts from, work and board settings both; on a repo's heading (grouped by repo) your entry for
that repo. The `?` menu has a line for yours under the task's. A change there is the one `set` (or `set --repo`) makes,
with the same checks, and `Ctrl+U` is `unset`. `*` marks what you set; every other row says where it comes from
(`built in`, `you`, `the repo's file`). A task with no answer of its own follows at once. Another machine's settings
are changed on that machine.

## The list

<!-- made:settings Made by `herdr-projects docs` from crates/core/src/store/settings_list.rs. Do not edit between these two lines. -->
| Setting | Built in | Can be said by | Means |
|---|---|---|---|
| `worker.agent` | `claude` | task, repo entry, you | the working agent's program: one of `claude`, `codex`, `pi`, `agy`, `copilot`, `cursor`, `devin`, `droid`, `grok`, `hermes`, `kilo`, `kimi`, `letta`, `mastracode`, `omp`, `opencode`, `qodercli`, `qwen`, with an optional model after a slash (`codex/gpt-6.1-sol`, `agy/gemini-3-pro`). The model travels with the program, so one program's model is never handed to another; a name the program does not take is refused with the ones it does (`herdr-projects models` lists them, [models](models.md)). A program that cannot do this role, or take a model or an effort, is refused with the reason ([agents](agents.md)) |
| `worker.effort` | `default` | task, repo entry, you | the working agent's effort level: one its program accepts (checked against `worker.agent`), or `default` for the program's own |
| `worker.fast` | `false` | task, repo entry, you | the working agent in its program's fast mode: a faster service tier that uses more of your allowance. Only Codex has one; for another program it does nothing, and `show` says so |
| `worker.access` | `full` | task, repo entry, you | what the working agent may do without asking: `full` (anything), `ask` (nothing: it asks first) or `auto` (its program's own auto mode, which checks each action and does the safe ones without asking, and stops the risky ones). The board turns it into each program's own flags. `ask` is refused for a program that cannot ask (pi); `auto` for one with no auto mode (all but claude, codex, cursor, qodercli, qwen), and for a version older than its first |
| `worker.instructions` | `none` | task, repo entry, you | your own words for the working agent, added after the board's own fixed rules. They add up across the places, broadest first, and the agent is told that later ones win |
| `worker.fallback` | `none` | task, repo entry, you | who takes over when the working agent cannot go on: a list of agents tried in order (`codex, claude/claude-opus-5-5`; a model after a slash, as in `worker.agent`), or `none`. It takes over in the same worktree, with the task, what was done so far and the open findings, when the agent's provider is down (after the board's own tries), the agent is out of allowance or logged out, or its work still fails the review or the check after `review.rounds` rounds. With the list used up the task waits for you. Nobody extra runs until then. More: [the fallback agent](fallback.md) |
| `reviewer.agent` | `codex` | task, repo entry, you | the reviewer's program: one of `claude`, `codex`, `pi`, `agy`, `copilot`, `cursor`, `devin`, `droid`, `grok`, `hermes`, `kilo`, `kimi`, `letta`, `mastracode`, `omp`, `opencode`, `qodercli`, `qwen`, with an optional model after a slash (`codex/gpt-6.1-sol`, `agy/gemini-3-pro`). The model travels with the program, so one program's model is never handed to another; a name the program does not take is refused with the ones it does (`herdr-projects models` lists them, [models](models.md)). A program that cannot do this role, or take a model or an effort, is refused with the reason ([agents](agents.md)). Not installed here: the worker's program reviews, on its own model, and the task says so |
| `reviewer.effort` | `default` | task, repo entry, you | the reviewer's effort level: one its program accepts (checked against `reviewer.agent`), or `default` for the program's own |
| `reviewer.fast` | `false` | task, repo entry, you | the reviewer in its program's fast mode: a faster service tier that uses more of your allowance. Only Codex has one; for another program it does nothing, and `show` says so |
| `reviewer.access` | `full` | task, repo entry, you | what the reviewer may do without asking: `full` (anything), `ask` (nothing: it asks first) or `auto` (its program's own auto mode, which checks each action and does the safe ones without asking, and stops the risky ones). The board turns it into each program's own flags. `ask` is refused for a program that cannot ask (pi); `auto` for one with no auto mode (all but claude, codex, cursor, qodercli, qwen), and for a version older than its first |
| `reviewer.instructions` | `none` | task, repo entry, you | your own words for the reviewer, added after the board's own fixed rules. They add up across the places, broadest first, and the agent is told that later ones win |
| `reviewer.fallback` | `none` | task, repo entry, you | who takes over when the reviewer cannot go on: a list of agents tried in order (`codex, claude/claude-opus-5-5`; a model after a slash, as in `reviewer.agent`), or `none`. It reviews in its place when the reviewer's provider is down (after the board's own tries), the reviewer is out of allowance or logged out, or its program is not installed here; it then reviews the later rounds of that task too. With the list used up the review fails as before. Nobody extra runs until then. More: [the fallback agent](fallback.md) |
| `coordinator.agent` | `claude` | coordinator, you | the Coordinator, the agent you talk to in the chat's program: one of `claude`, `codex`, `pi`, `agy`, `copilot`, `cursor`, `devin`, `droid`, `grok`, `hermes`, `kilo`, `kimi`, `letta`, `mastracode`, `omp`, `opencode`, `qodercli`, `qwen`, with an optional model after a slash (`codex/gpt-6.1-sol`, `agy/gemini-3-pro`). The model travels with the program, so one program's model is never handed to another; a name the program does not take is refused with the ones it does (`herdr-projects models` lists them, [models](models.md)). A program that cannot do this role, or take a model or an effort, is refused with the reason ([agents](agents.md)). `none`: no Coordinator |
| `coordinator.effort` | `default` | coordinator, you | the Coordinator, the agent you talk to in the chat's effort level: one its program accepts (checked against `coordinator.agent`), or `default` for the program's own |
| `coordinator.fast` | `false` | coordinator, you | the Coordinator, the agent you talk to in the chat in its program's fast mode: a faster service tier that uses more of your allowance. Only Codex has one; for another program it does nothing, and `show` says so |
| `coordinator.access` | `full` | coordinator, you | what the Coordinator, the agent you talk to in the chat may do without asking: `full` (anything), `ask` (nothing: it asks first) or `auto` (its program's own auto mode, which checks each action and does the safe ones without asking, and stops the risky ones). The board turns it into each program's own flags. `ask` is refused for a program that cannot ask (pi); `auto` for one with no auto mode (all but claude, codex, cursor, qodercli, qwen), and for a version older than its first |
| `coordinator.instructions` | `none` | coordinator, you | your own words for the Coordinator, the agent you talk to in the chat, added after the board's own fixed rules. They add up across the places, broadest first: yours, then this coordinator's own |
| `autopilot.goal` | `none` | coordinator | what a coordinator on autopilot works toward, in your words. Autopilot cannot be on without one. Said for one coordinator only: never for you, never for this time. More: [autopilot](autopilot.md) |
| `autopilot.every` | `6h` | this time, coordinator, you | autopilot's timer, or `none`: a timer wake happens only if the board changed in its area since its last wake ended (the free look, which costs nothing), or a new day began with room |
| `autopilot.gap` | `10m` | this time, coordinator, you | the least time between the end of one wake's turn, or of a turn a message of yours started, and the next wake; reasons that come meanwhile are kept and merged into one wake |
| `autopilot.wakesPerDay` | `24` | this time, coordinator, you | the most wakes in one day (this machine's time zone), counted when a wake is typed in; reasons after that wait for the next day |
| `autopilot.when` | `done stuck added` | this time, coordinator, you | the board events that wake it, or `none`: `done` a task in its area was merged or done; `stuck` a task it may look after starts to need looking after (it failed, or its agent asks a question); `added` a task was added in its area by anyone but this coordinator |
| `autopilot.hours` | `any` | this time, coordinator, you | when it may wake: `any`, or days and hours in this machine's time zone (`mon-fri 09:00-18:00`, `22:00-06:00`, `sat,sun 10:00-16:00`). Outside them its reasons wait |
| `autopilot.tasksPerDay` | `5` | this time, coordinator, you | the most tasks it adds in one day (this machine's time zone) |
| `autopilot.tasksAtOnce` | `2` | this time, coordinator, you | the most tasks under way that it added or started: from start (or adding, when chained) until merged, done, stopped or archived; waiting for you counts. Under `autopilot.scope all`, your tasks it started count |
| `autopilot.mayChange` | `none` | this time, coordinator, you | which task settings it may give the tasks it adds: exact names (`worker.effort`) and areas (`review.*`), or `none`. Never `merge.*`, `limit.*` or `autopilot.*`; a bare `*` is refused. A setting it is not allowed is not applied, and its answer names it |
| `autopilot.scope` | `own` | this time, coordinator, you | `own`: it looks after only the tasks it added. `all`: it may also start your to-do tasks in its area, on its own machine, and then looks after them. A task of yours it did not start is never its to look after |
| `autopilot.approve` | `no` | this time, coordinator, you | whether it may say yes (a plan, a merge) for the tasks it may look after. `no`: your `merge.by` alone decides. `yes`: it can merge under `merge.by you`, the one setting that overrides `merge.by`. Never for a task held by `autopilot.repoFileWaits`, never `--anyway` |
| `autopilot.switchOthers` | `no` | this time, coordinator, you | whether, on its own, it may turn other coordinators' autopilot on (only with that one's saved goal and settings). Turning off is always allowed; its rules use this to judge whether an unasked off of another is fine |
| `autopilot.releases` | `no` | this time, coordinator, you | whether it may add a release task (a version bump, a tag, a publish). `no`: it says when it thinks one is due. A rule it is told, as a task's purpose cannot be checked by code |
| `autopilot.retries` | `1` | this time, coordinator, you | how many times it may retry one task before that task is left to you (`0`: at its first failure or question). Answering a question uses none |
| `autopilot.startAtOnce` | `yes` | this time, coordinator, you | `yes`: every task it adds is started at once, or chained after one of its own. `no`: it may leave tasks to do |
| `autopilot.needsYouAfter` | `3` | this time, coordinator, you | after this many unanswered wakes in a row (a wake whose turn did not end within 15 minutes), its row says `needs you` and no more wakes are sent until you turn it on again |
| `autopilot.repoFileWaits` | `yes` | this time, coordinator, you | `yes`: a task it added whose change touches the repo's own `.herdr-projects.json` never merges by itself, whatever `merge.by` or `autopilot.approve` say: it waits for your own yes, its row saying why |
| `watcher.agent` | `claude/claude-haiku-5-5` | you only | the Watcher: a small model behind the "doing now" line, the progress guess and the minutes left. `none`: none of the three. A program with no headless one-shot is refused ([agents](agents.md)). More: [the Watcher](watcher.md) |
| `watcher.every` | `60s` | you only | the least time between two looks of the Watcher at one working agent. One hidden model call covers every agent whose look is due, one call runs at a time on the machine, a screen that has not changed costs none, and a busy machine is looked at up to five times less often |
| `research.only` | `false` | task, repo entry, you, repo file | research only: the task's agent changes nothing in the repo and its result is a report, with a private link. Nothing is committed, pushed, opened as a pull request or merged for it, and its row says `[research]`. Read when the agent starts: a change counts from the task's next start, and turning it on is refused once the agent has changed the repo or a pull request is open. `--research` and `--no-research` on `add` and `delegate` are short for it. More: [reports and research-only tasks](research.md) |
| `chat.only` | `false` | task, repo entry, you, repo file | chat only: the task is a conversation (a consultation, a discussion, thinking out loud) that ends only when you say. Its agent changes nothing in the repo, and every stop of its agent waits for you: no check, no review, no push, no pull request, no merge, whatever the review settings say. Yes on the board (or `herdr-projects finish <id>`) ends it as done, with the agent's last message, or its report, as the summary. Its row says `[chat]`. With `research.only` on as well, chat wins. Read when the agent starts. `--chat` and `--no-chat` on `add` and `delegate` are short for it. More: [chat-only tasks](chat.md) |
| `workspace.from` | `main` | task, repo entry, you | where a worktree starts, and where its work lands: `main` (the remote's main branch, fetched first), `here` (the commit your checkout is on) or a branch's name at the remote (`develop`). A named branch is fetched from origin on every start, and one origin does not have is refused; the task keeps up with that branch, its pull request merges into it and GitHub's checks are those of a pull request into it. It needs `merge.pr` on: a local merge of a named branch is refused for now. Read when the worktree is made. More: [how work gets done](how-work-gets-done.md) |
| `workspace.branch` | `work/{n}-{slug}` | task, repo entry, you | the name of a task's branch: `{n}` is the task's number (an attempt's letter included: `7b`), `{slug}` its title in small words with dashes, `{key}` its repo's key and number (`KIC-123`). It needs `{n}` or `{key}`. git must take it as a branch name (`git check-ref-format`); one it would refuse is refused with the nearest one it takes. A name already taken gets `-2`, `-3` and so on after it. Read when the worktree is made. More: [how work gets done](how-work-gets-done.md) |
| `workspace.setup` | `none` | task, repo entry, you, repo file | a command run in the fresh worktree before the agent starts |
| `workspace.include` | `none` | task, repo entry, you, repo file | ignored files a fresh worktree lacks and gets a copy of (`.env`); `.worktreeinclude` in the repo names more |
| `workspace.teardown` | `none` | task, repo entry, you, repo file | a command run in the worktree before it is removed |
| `workspace.cleanup` | `true` | task, repo entry, you | remove a done task's space (so it leaves herdr's sidebar), worktree and branch. More: [when a task is done](how-work-gets-done.md#when-a-task-is-done) |
| `attempts.by` | `1` | task, repo entry, you | how many attempts are made at once, and by whom: a count (`3`), or a list of agents (`claude, codex, pi`; a model after a slash; a name may repeat). A count means that many of `worker.agent`. Each attempt has its own worktree, branch and tab, runs the check, and holds one of `board.agents`' slots; one that finds no free slot waits its turn. At most 5. `1`: the task is done once, as ever. More: [several attempts](attempts.md) |
| `attempts.pick` | `you` | task, repo entry, you | who chooses among the attempts that passed the check. `you`: the task waits as "pick one", the attempts side by side, with the reviewer's comparison when there is a reviewer. `reviewer`: the reviewer compares them, names one and says why, and that one goes on by itself. Only the chosen attempt is reviewed in full, pushed and merged. More: [several attempts](attempts.md) |
| `plan.approve` | `none` | task, repo entry, you | whether the work starts with a plan, and who says yes to it; nothing in the repo is changed before that yes. `none`: no plan. `you`: the agent writes its plan and the task waits as `plan ready` for your yes; a reply sends the plan back with your words, and it is written again. `reviewer`: the reviewer checks the plan against the task's description and the build goes on, or the plan goes back to its writer with the reviewer's points. Read when the agent starts and again when the plan is written. More: [a plan first](plan.md) |
| `plan.agent` | `none` | task, repo entry, you | who writes the plan, where there is one (`plan.approve`). `none`: the working agent, in its program's own plan mode where it has one (Claude Code). A program, with an optional model after a slash (`claude/claude-opus-5-5`): that one writes the plan into the task's folder, in a tab of its own (`Planner`), and the working agent builds from it. More: [a plan first](plan.md) |
| `check.command` | `none` | task, repo entry, you, repo file | the test command: the work only counts as done when it passes |
| `check.again` | `none` | task, repo entry, you, repo file | a shorter check run instead of `check.command` on a later round of a task (sent back by a review, a failed check or GitHub's checks), once the full check has passed on its branch, and only while GitHub runs its checks before the merge (`merge.checks` is `github`, the pull request is open and a workflow of the branch runs for it). It is told the commit the full check passed on, in `HERDR_PROJECTS_CHECKED`, to run only the tests the change since then reaches. `none`: the full check every time. More: [shorter checks on fix rounds](shorter-checks.md) |
| `check.timeout` | `15m` | task, repo entry, you, repo file | how long the check may run before it is stopped and counts as failed |
| `review.when` | `repos` | task, repo entry, you | what gets a review when finished: `repos` (work in a repo), `always` (work outside one too) or `never` |
| `review.mustFix` | `major` | task, repo entry, you | a finding that bad or worse fails the review. More: [the gate](gate.md) |
| `review.rounds` | `3` | task, repo entry, you | how many times work goes back to the agent by itself, for any one reason: findings, a failed check, a conflict with main, pull request feedback. `0`: nothing goes back by itself, and a failed review waits for you |
| `review.timeout` | `20m` | task, repo entry, you | how long one review may take, and one comparison of attempts: a time (`45m`, `1h`) or `none`. Read when the review starts. Reached: the reviewer is stopped and the review fails (`review timed out after 20m (review.timeout)`), and what happens next is as for any failed review. Not from the repo's own file, so the code under review cannot change it. Unlike `limit.time`, which stops any run and waits for you. More: [the gate](gate.md) |
| `review.afterChat` | `60s` | task, repo entry, you | how long a chat with the agent of a task whose work was reviewed (under review, waiting for your yes, done) must be quiet before the code it changed is reviewed: no new message from you and the agent at rest for this long, or you left the chat (another task on the board, another space in herdr). Then one review runs on the final state. A chat that changed no code reviews nothing: the task goes back to where it was, with the same review. More: [how work gets done](how-work-gets-done.md) |
| `review.evidence` | `true` | task, repo entry, you | the reviewer brings evidence: screenshots, a recording, a link, test output |
| `limit.time` | `none` | task, repo entry, you | the longest one run of an agent may take, the working agent's or the reviewer's: a time (`2h`, `45m`) or `none`. Read when a run starts: set while one is under way, it counts from the next. Reached: the agent is stopped, its work is kept as it is, and the task waits for you under "needs you" (`stopped at its 2h limit`). Nothing is merged. More: [limits](limits.md) |
| `limit.tokens` | `none` | task, repo entry, you | the most tokens a task may use, all its agents, attempts and reviews together, what is read again from the provider's cache included: a count (`2m`, `500k`) or `none`. Counted at the end of each turn, so one turn can pass it. Reached: as `limit.time` (`stopped at 2m tokens`). For a program whose tokens cannot be counted it does nothing, and `show <id>` says so. More: [limits](limits.md) |
| `merge.by` | `you` | task, repo entry, you | `you`: finished work waits for your yes. `itself`: it merges the moment it may: its review passed, its checks pass, nothing conflicts. Read right before the merge, so a change counts for tasks under way. More: [the gate](gate.md#merging-by-itself) |
| `merge.method` | `squash` | task, repo entry, you | how approved work lands |
| `merge.pr` | `true` | task, repo entry, you | push the branch and open a draft pull request after the check passes. Turned off, no new one is opened: a task that already has its pull request still pushes to it after each check that passes (main merged in after the review too) and merges through it |
| `merge.pushTimeout` | `2m` | task, repo entry, you, repo file | how long the push of a task's branch may run, the repo's pre-push hook included: a time (`10m`) or `none`. Reached: the push is stopped and the finish fails (`git push did not end within 2m (merge.pushTimeout)`). Raise it for a repo whose pre-push hook runs its tests: `set merge.pushTimeout 10m --repo <name>`. More: [how work gets done](how-work-gets-done.md#the-finish-line) |
| `merge.checks` | `github` | task, repo entry, you | whether a pull request waits for its checks on GitHub before it merges, by itself or on your yes. `github`: still running, it waits for them, at most `check.timeout`, then it waits for you with that reason; failed, nothing merges and the failure goes back to the agent with each failing check's name and the end of its log, as a failed check does, within `review.rounds`, then it waits for you (`approve <id> --anyway` merges it all the same); passed, and nothing expected still out, it merges: each workflow of the branch that runs for the pull request (its events, branches and paths, as GitHub reads them) must have its run on the commit through, every job and matrix member of it, each check the branch requires must be listed, and a check gone from the list is still waited for; with nothing listed at all, only a repo where no outside CI reports merges. A fine-grained token (which cannot have the Checks permission) is enough: the checks are then read from Actions and Commit statuses, and what it cannot read (another app's check runs) is said on the task, not asked as a login. `none`: GitHub's checks are not looked at. More: [how work gets done](how-work-gets-done.md) |
| `merge.closes` | `none` | task only | the GitHub issues of the task's own repo its work closes, by number (`2203, 2204`), or `none`. One task's own: set it when the task is added (`add … --merge.closes 2203`) or later (`set merge.closes 2203 --task <id>`). Its pull request gets a `Closes #2203` line for each, so GitHub closes them when it merges into the default branch (one the agent opened itself gets the lines added, and the board's own lines of an issue no longer named are taken off before it merges); a merge with no pull request closes them with `gh issue close` once it has landed, with a comment naming the task and the merge commit. Only on a merge that landed; a close that fails never fails the merge, and the task's line says what was closed or why not. More: [how work gets done](how-work-gets-done.md#closing-github-issues) |
| `merge.sync` | `always` | task, repo entry, you | when main is merged into a task's branch. `always`: before its review, whenever main moves while it waits for your yes (each merge checked again and pushed to its pull request) and after your yes. `approve`: before its review and after your yes only: while it waits, a test merge in memory (nothing in its branch or pull request changes) only finds out whether main conflicts, and a conflict goes to its agent as a real one does. Fewer merge commits, CI runs and bot reviews on a pull request in a busy repo. More: [how work gets done](how-work-gets-done.md#staying-current-with-main) |
| `repo.key` | `none` | repo entry, repo file | the short key of this repo's tasks, as `SKL` for gtm-skills: 2 to 5 letters or digits, the first a letter, kept in capitals. Every repo and folder has one: the board makes it from the name the moment it first sees the repo (`SKL` for gtm-skills, `HOME` for the home folder, `ROOT` for `/`) and saves it as your entry for the repo, so `none` lasts only until then. A task's tag on the board reads `HSW-134`, and `HSW-134` (in any letter case) names the task wherever its id `herdr-projects/134` does. Set it to change it: the old key still names the tasks. A key another repo already has is refused; `unset` goes back to the repo's file's key, or to the one made from the name. More: [task numbers and keys](task-numbers.md) |
| `board.agents` | `4` | you only | how many agents work at once |
| `board.diskFree` | `5` | you only | how many GB a disk the board writes to (the repo's, the worktrees', the board's data, the temporary files) keeps free, never more than half of that disk (a `/tmp` of 3.4 GB in all keeps 1.7 GB): under it, a task about to start (its worktree, its setup, its agent) first sweeps what ended tasks left, and still under it waits in the queue (`waiting for disk space, Q#1`) with a plain line (`Disk nearly full on vm-str: 3.1 GB free …`) instead of failing half-way through a build; it starts by itself once there is room. `0`: never waits. More: [disk space](disk-space.md) |
| `board.oldBuilds` | `7` | you only | after how many days without a change a done or archived task's kept worktree loses its build output (`target/`, `node_modules/`, `.next/`: only what git ignores) in the daily sweep; the next build makes it again. `0`: never. More: [disk space](disk-space.md) |
| `board.machines` | `auto` | you only | other machines: `auto` (herdr's saved machines), names (`exedev@vm-a, vm-b`) or `none` |
| `board.open` | `tab` | you only | what the board key opens. `tab`: the board in a herdr tab of its own, which stays when you go into an agent; pressed on the board itself, the key takes you back to where you came from. `popup`: the board as herdr's popup, a framed window in the middle of herdr's screen, over whatever is there (in a window under 110 columns or 30 rows it takes the whole pane: a margin would cost half the board); the key again or `Esc` closes it, as its hint line says (`esc close`), it closes when you go into an agent, and it is never left behind on a tab. More: [the board](board.md) |
| `board.notify` | `none` | you only | when a task needs you, is ready to approve, or failed: `bell` rings the terminal bell, a command is run with `HERDR_PROJECTS_ID`, `HERDR_PROJECTS_TITLE` and `HERDR_PROJECTS_EVENT` (`needs you`, `ready to approve` or `failed`) in its environment, `none` does neither |
| `board.keys` | `none` | you only | your own chord for some of the board's keys, when the one it comes with does not reach it (a window manager or a terminal that takes `Alt`): `name=chord` pairs, each name a key's from the tables in [keys](keys.md#all-the-keys), such as `chatMessage=ctrl+g, chatScrollUp=ctrl+up, chatScrollDown=ctrl+down`, or `none`. The key then answers to your chord only, and the `?` menu, the hint line and the board's messages name it so. A chord another key of the board has, or the key that opens the board (or herdr's prefix, when that key starts with it), is refused with the key it would clash with: move that one too, in the same value. A plain character only for a key that is one already (`talk`, `menu`, `sayYes`, `sayNo`): any other is typed into the text box. The text box's own keys cannot be moved. Yours only: never per repo, and never from a repo's file. More: [your own keys](keys.md#your-own-keys) |
| `board.keyStyle` | `auto` | you only | how the board writes its keys, in the hint line, the `?` menu and its messages: as on the keyboard you type on. `mac`: the Option key's chords are written `opt+…`, not `alt+…`; `ctrl` stays `ctrl`, as a Mac's terminal passes Control on and never Cmd. `pc`: `alt`. `auto`: by the system the board runs on, so a board on a Linux machine you reach over SSH from a Mac says `alt`: set `mac` there. The keys themselves stay the same. More: [key names on a Mac](keys.md#key-names-on-a-mac) |
| `board.webUi` | `auto` | you only | the board inside the herdr web UI. `auto`: when the web UI plugin (`devswha.herdr-web-ui`) is installed, a small front door of the board's own runs in front of it and adds a Boards section to its sidebar; without that plugin nothing runs. `none`: never. More: [the board in the web UI](web-ui.md) |
| `voice.calls` | `on` | you only | voice calls at all ([voice calls](voice.md)). `on`: a call rings with the call key or the web board's Call button, once an ElevenLabs key is there. `off`: there are none: the call key only says so, the web board has no Call button and its page may not reach ElevenLabs, the front door refuses a call and the voice agent's tools, the call helper does not run (whatever `voice.helper` says), and nothing contacts ElevenLabs. More: [turning calls off](voice.md#turning-calls-off) |
| `voice.language` | `auto` | you only | the language of a voice call ([voice calls](voice.md)). `auto`: it starts in English and follows you into any language ElevenLabs' agents speak, as you speak it; one of their language codes (`en`, `de`, `es`, `fr`, `ja`, `pt-br` …): that one only, and the voice waits for you to speak first |
| `voice.voice` | `none` | you only | the ElevenLabs voice a call speaks with: a voice id from your ElevenLabs account that agents can use (a voice with live moderation cannot be), or `none` for the built-in one: Warren (`7QN34D2r3hCNwbOYIeK0`) from ElevenLabs' Voice Library, added to your account when it is not there (a key with Voices: Write), and until then Will in an account made before March 2026 ([voice calls](voice.md#setting-it-up)). When ElevenLabs refuses a voice, the call says so and how to pick another |
| `voice.llm` | `none` | you only | the language model the call's voice agent thinks with: one of the LLMs ElevenLabs lists for agents, or `none` for the built-in one, DeepSeek V4.1 Flash (`deepseek-v41-flash`): small and quick, as it only talks and relays. Speech stays ElevenLabs' `eleven_v4_turbo`. When ElevenLabs refuses a model, the call says so and how to pick another |
| `voice.helper` | `auto` | you only | on the computer you sit at: a small helper that holds a call started from the terminal board (this computer's or another machine's), with this computer's microphone and speaker and no browser. `auto`: it runs while herdr has saved machines; `none`: never, and a call from the terminal board is not picked up here. More: [voice calls](voice.md#from-the-terminal-board) |
| `board.theme` | `herdr` | you only | the board's colours. `herdr`: herdr's own theme, read from this machine's `config.toml` and resolved as herdr does, so with no theme set there it is herdr's default (Catppuccin); `dark` or `light` picks that theme's dark or light one by hand; the name of one of herdr's themes (`nord`, `gruvbox`, … and `terminal` for the terminal's own colours) is that theme whatever the config says, for a herdr opened from another machine whose config names another one. More: [colours](colours.md) |
<!-- /made:settings -->

## Other machines

A task's work settings travel with it. The machine you are on works out what you set for it (you, and your entry for its
repo) and sends that along with whatever starts its agent. Board settings stay with each machine
([other machines](other-machines.md)).

## What is fixed

These are not settings:

- a task in a repo always works in a git worktree of its own, never in your checkout;
- the board always keeps the size of its own pane;
- evidence links are shared privately on your tailnet when Tailscale is installed, and on 127.0.0.1 when it is not;
- the how-to for agents and the trust mark for a task's repo are written;
- a conflict with main goes back to the task's agent by itself (within `review.rounds`);
- a passed review still stands after a clean catch-up with main when the check passes;
- only your own comments on a pull request (the logged-in GitHub user's) wake its agent.

## Coming from the old names

The first run of this version converts your `config.json` and every task's own settings by itself, once, and keeps the
old file beside the new one as `config.old.json`. A task under way keeps the choices it had.

Data lives in `~/.herdr-projects/data` (`HERDR_PROJECTS_DATA_DIR` overrides it): one folder per task, with `task.json`,
`inbox.jsonl` and `artifacts/`. Everything the board keeps: [the home folder](home-folder.md).
