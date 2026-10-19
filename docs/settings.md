# Settings

**Every choice about how work happens is a setting, and every setting is looked up by one rule: the nearest answer
wins.** Set one for everything, for one repo, or for one task:

```
herdr-projects set merge.by itself                       you: every task
herdr-projects set worker.agent codex --repo my-app      your entry for one repo
herdr-projects set review.rounds 5 --task 12             one task
herdr-projects show 12                                   what task 12 runs on, and where each value comes from
```

On the board, `Ctrl+E` opens the selected task's settings. Or just ask the Coordinator. Every setting, with what it
means, is in [the list](#the-list) below.

## Two kinds of setting

- **Work settings**: who works, where, how it is checked, how it lands. They can be set for a task, a repo, or you.
- **Board settings**: the board itself, the Coordinator, the Watcher (everything under `board.`, `coordinator.` and
  `watcher.`). They are yours only. Setting one on a task or for a repo is refused, and the answer says how to set it for
  yourself.

One setting is neither: `repo.key`, a fact about one repo.

## The rule

1. **A work setting is looked up in one order, and the nearest answer wins:**
   **this task → your entry for this repo → you → the repo's own file → built in.**
   One function does this for every setting (`crates/core/src/store/settings/lookup.rs`); no setting has a lookup of its
   own.
2. **A board setting is yours only** (above).
3. **A setting is read when its step starts, and a step under way is never changed**: the agent's settings at its next
   start (whether the task is research only among them), the review's at the next review, the merge's right before the
   merge.
4. **A repo's own fact** (`repo.key`, the short key of its tasks) is set for that repo and nowhere else: **your entry for
   the repo → the repo's own file → built in.** It is refused on a task and for yourself. Every repo and folder has one:
   the board makes it from the repo's name when it first sees the repo, and saves it as your entry for that repo
   (`"repos": { "gtm-skills": { "repo": { "key": "SKL" } } }`). More: [task numbers and repo keys](task-numbers.md).
5. **A task in no repo** skips the two repo places. `workspace.*`, `check.*` and `merge.*` do not apply to it, and `show`
   says so.

## The places

- **You**: `~/.herdr-projects/settings/config.json` ([the home folder](home-folder.md)).
- **Your entry for a repo**: the `repos` section of that same file, keyed by the repo's name on the board.
- **This task**: saved on the task, only the settings that differ. A task an [automation](automations.md) made starts with
  the automation's own settings here; an automation is not a place of its own.
- **The repo's own file**: `.herdr-projects.json`, committed in that repo. It may hold seven facts only: `research.only`,
  `workspace.setup`, `workspace.include`, `workspace.teardown`, `check.command`, `check.timeout` and `repo.key`.
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
herdr-projects unset <name>  [--repo … | --task … | --automation …]   back to what it would be
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

## The list

<!-- made:settings Made by `herdr-projects docs` from crates/core/src/store/settings_list.rs. Do not edit between these two lines. -->
| Setting | Built in | Can be said by | Means |
|---|---|---|---|
| `worker.agent` | `claude` | task, repo entry, you | the working agent's program: one of `claude`, `codex`, `pi`, `agy`, `copilot`, `cursor`, `freebuff`, `codebuddy`, `cline`, `gemini`, `kiro`, `devin`, `droid`, `grok`, `hermes`, `kilo`, `kimi`, `letta`, `mastracode`, `omp`, `opencode`, `qodercli`, `qwen`, with an optional model after a slash (`codex/gpt-6.1-sol`, `agy/gemini-3-pro`). The model travels with the program, so one program's model is never handed to another; a name the program does not take is refused with the ones it does (`herdr-projects models` lists them, [models](models.md)). A program that cannot do this role, or take a model or an effort, is refused with the reason ([agents](agents.md)) |
| `worker.effort` | `default` | task, repo entry, you | the working agent's effort level: one its program accepts (checked against `worker.agent`), or `default` for the program's own |
| `worker.fast` | `false` | task, repo entry, you | the working agent in its program's fast mode: a faster service tier that uses more of your allowance. Only Codex has one; for another program it does nothing, and `show` says so |
| `worker.access` | `full` | task, repo entry, you | what the working agent may do without asking: `full` or `ask`. The board turns it into each program's own flags. `ask` is refused for a program that cannot ask (pi) |
| `worker.instructions` | `none` | task, repo entry, you | your own words for the working agent, added after the board's own fixed rules. They add up across the places, broadest first, and the agent is told that later ones win |
| `worker.fallback` | `none` | task, repo entry, you | who takes over when the working agent cannot go on: a list of agents tried in order (`codex, claude/claude-opus-5-5`; a model after a slash, as in `worker.agent`), or `none`. It takes over in the same worktree, with the task, what was done so far and the open findings, when the agent's provider is down (after the board's own tries), the agent is out of allowance or logged out, or its work still fails the review or the check after `review.rounds` rounds. With the list used up the task waits for you. Nobody extra runs until then. More: [the fallback agent](fallback.md) |
| `reviewer.agent` | `codex` | task, repo entry, you | the reviewer's program: one of `claude`, `codex`, `pi`, `agy`, `copilot`, `cursor`, `freebuff`, `codebuddy`, `cline`, `gemini`, `kiro`, `devin`, `droid`, `grok`, `hermes`, `kilo`, `kimi`, `letta`, `mastracode`, `omp`, `opencode`, `qodercli`, `qwen`, with an optional model after a slash (`codex/gpt-6.1-sol`, `agy/gemini-3-pro`). The model travels with the program, so one program's model is never handed to another; a name the program does not take is refused with the ones it does (`herdr-projects models` lists them, [models](models.md)). A program that cannot do this role, or take a model or an effort, is refused with the reason ([agents](agents.md)). Not installed here: the worker's program reviews, on its own model, and the task says so |
| `reviewer.effort` | `default` | task, repo entry, you | the reviewer's effort level: one its program accepts (checked against `reviewer.agent`), or `default` for the program's own |
| `reviewer.fast` | `true` | task, repo entry, you | the reviewer in its program's fast mode: a faster service tier that uses more of your allowance. Only Codex has one; for another program it does nothing, and `show` says so |
| `reviewer.access` | `full` | task, repo entry, you | what the reviewer may do without asking: `full` or `ask`. The board turns it into each program's own flags. `ask` is refused for a program that cannot ask (pi) |
| `reviewer.instructions` | `none` | task, repo entry, you | your own words for the reviewer, added after the board's own fixed rules. They add up across the places, broadest first, and the agent is told that later ones win |
| `reviewer.fallback` | `none` | task, repo entry, you | who takes over when the reviewer cannot go on: a list of agents tried in order (`codex, claude/claude-opus-5-5`; a model after a slash, as in `reviewer.agent`), or `none`. It reviews in its place when the reviewer's provider is down (after the board's own tries), the reviewer is out of allowance or logged out, or its program is not installed here; it then reviews the later rounds of that task too. With the list used up the review fails as before. Nobody extra runs until then. More: [the fallback agent](fallback.md) |
| `coordinator.agent` | `claude` | you only | the Coordinator, the agent you talk to in the chat's program: one of `claude`, `codex`, `pi`, `agy`, `copilot`, `cursor`, `freebuff`, `codebuddy`, `cline`, `gemini`, `kiro`, `devin`, `droid`, `grok`, `hermes`, `kilo`, `kimi`, `letta`, `mastracode`, `omp`, `opencode`, `qodercli`, `qwen`, with an optional model after a slash (`codex/gpt-6.1-sol`, `agy/gemini-3-pro`). The model travels with the program, so one program's model is never handed to another; a name the program does not take is refused with the ones it does (`herdr-projects models` lists them, [models](models.md)). A program that cannot do this role, or take a model or an effort, is refused with the reason ([agents](agents.md)). `none`: no Coordinator |
| `coordinator.effort` | `default` | you only | the Coordinator, the agent you talk to in the chat's effort level: one its program accepts (checked against `coordinator.agent`), or `default` for the program's own |
| `coordinator.fast` | `false` | you only | the Coordinator, the agent you talk to in the chat in its program's fast mode: a faster service tier that uses more of your allowance. Only Codex has one; for another program it does nothing, and `show` says so |
| `coordinator.access` | `full` | you only | what the Coordinator, the agent you talk to in the chat may do without asking: `full` or `ask`. The board turns it into each program's own flags. `ask` is refused for a program that cannot ask (pi) |
| `coordinator.instructions` | `none` | you only | your own words for the Coordinator, the agent you talk to in the chat, added after the board's own fixed rules |
| `watcher.agent` | `none` | you only | the Watcher: a small model behind the "doing now" line, the progress guess and the minutes left. `none`: none of the three. Off by default: the configured coding CLI pool runs interactively and is never started as a headless one-shot. An optional helper outside that pool can be selected explicitly ([agents](agents.md)). More: [the Watcher](watcher.md) |
| `watcher.every` | `60s` | you only | the least time between two looks of the Watcher at one working agent. One hidden model call covers every agent whose look is due, one call runs at a time on the machine, a screen that has not changed costs none, and a busy machine is looked at up to five times less often |
| `research.only` | `false` | task, repo entry, you, repo file | research only: the task's agent changes nothing in the repo and its result is a report, with a private link. Nothing is committed, pushed, opened as a pull request or merged for it, and its row says `[research]`. Read when the agent starts: a change counts from the task's next start, and turning it on is refused once the agent has changed the repo or a pull request is open. `--research` and `--no-research` on `add` and `delegate` are short for it. More: [reports and research-only tasks](research.md) |
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
| `review.by` | `agent` | task, repo entry, you | Wer Ergebnisse prüft: agent startet einen eigenen Reviewer; coordinator gibt das Ergebnis an die bestehende sichtbare Orchestrator-Sitzung zurück. |
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
| `merge.pr` | `true` | task, repo entry, you | push the branch and open a draft pull request after the check passes |
| `merge.checks` | `github` | task, repo entry, you | whether a pull request waits for its checks on GitHub before it merges, by itself or on your yes. `github`: still running, it waits for them, at most `check.timeout`, then it waits for you with that reason; failed, nothing merges and the failure goes back to the agent with each failing check's name and the end of its log, as a failed check does, within `review.rounds`, then it waits for you (`approve <id> --anyway` merges it all the same); passed, and nothing expected still out, it merges: each workflow of the branch that runs for the pull request (its events, branches and paths, as GitHub reads them) must have its run on the commit through, every job and matrix member of it, each check the branch requires must be listed, and a check gone from the list is still waited for; with nothing listed at all, only a repo where no outside CI reports merges. A fine-grained token (which cannot have the Checks permission) is enough: the checks are then read from Actions and Commit statuses, and what it cannot read (another app's check runs) is said on the task, not asked as a login. `none`: GitHub's checks are not looked at. More: [how work gets done](how-work-gets-done.md) |
| `repo.key` | `none` | repo entry, repo file | the short key of this repo's tasks, as `SKL` for gtm-skills: 2 to 5 letters or digits, the first a letter, kept in capitals. Every repo and folder has one: the board makes it from the name the moment it first sees the repo (`SKL` for gtm-skills, `HOME` for the home folder, `ROOT` for `/`) and saves it as your entry for the repo, so `none` lasts only until then. A task's tag on the board reads `HSW-134`, and `HSW-134` (in any letter case) names the task wherever its id `herdr-projects/134` does. Set it to change it: the old key still names the tasks. A key another repo already has is refused; `unset` goes back to the repo's file's key, or to the one made from the name. More: [task numbers and keys](task-numbers.md) |
| `board.agents` | `4` | you only | how many agents work at once |
| `board.machines` | `auto` | you only | other machines: `auto` (herdr's saved machines), names (`exedev@vm-a, vm-b`) or `none` |
| `board.open` | `tab` | you only | what the board key opens. `tab`: the board in a herdr tab of its own, which stays when you go into an agent; pressed on the board itself, the key takes you back to where you came from. `popup`: the board as herdr's popup, a framed window in the middle of herdr's screen, over whatever is there (in a window under 110 columns or 30 rows it takes the whole pane: a margin would cost half the board); the key again or `Esc` closes it, as its hint line says (`esc close`), it closes when you go into an agent, and it is never left behind on a tab. More: [the board](board.md) |
| `board.notify` | `none` | you only | when a task needs you, is ready to approve, or failed: `bell` rings the terminal bell, a command is run with `HERDR_PROJECTS_ID`, `HERDR_PROJECTS_TITLE` and `HERDR_PROJECTS_EVENT` (`needs you`, `ready to approve` or `failed`) in its environment, `none` does neither |
| `board.keys` | `none` | you only | your own chord for some of the board's keys, when the one it comes with does not reach it (a window manager or a terminal that takes `Alt`): `name=chord` pairs, each name a key's from the tables in [keys](keys.md#all-the-keys), such as `chatMessage=ctrl+g, chatScrollUp=ctrl+up, chatScrollDown=ctrl+down`, or `none`. The key then answers to your chord only, and the `?` menu, the hint line and the board's messages name it so. A chord another key of the board has, or the key that opens the board (or herdr's prefix, when that key starts with it), is refused with the key it would clash with: move that one too, in the same value. A plain character only for a key that is one already (`talk`, `menu`, `sayYes`, `sayNo`): any other is typed into the text box. The text box's own keys cannot be moved. Yours only: never per repo, and never from a repo's file. More: [your own keys](keys.md#your-own-keys) |
| `board.webUi` | `auto` | you only | the board inside the herdr web UI. `auto`: when the web UI plugin (`devswha.herdr-web-ui`) is installed, a small front door of the board's own runs in front of it and adds a Boards section to its sidebar; without that plugin nothing runs. `none`: never. More: [the board in the web UI](web-ui.md) |
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
