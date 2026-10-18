# Glossary

This page gives the meaning of each term that these pages use. Each term has one meaning. The links go to the
pages that tell more about the term.

## A

- **agent program**: A coding agent program that Herdr can start and that has its own Herdr integration, for example `claude` or `codex`. ([agent-programs](../reference/agent-programs.md))
- **Also noticed**: A short list at the end of the closing note or report of an agent, with work that it found but did not do. ([roles](../reference/roles.md), [research-tasks](../reference/research-tasks.md))
- **area**: A group of test files with one name, for example `board-panel` for `tests/board_panel.rs` and `tests/board_panel_*.rs`. ([run-the-tests](../contributing/run-the-tests.md))
- **attempt**: One of several agents that do the same task at the same time, each with its own worktree, branch and tab. ([attempts](../explanation/attempts.md))
- **automation**: A saved task plus a trigger; each time that the trigger fires, the board adds a task and puts an agent on it. ([automations](../explanation/automations.md))
## B

- **batch**: One step of big work, done as its own task with its own check, review and merge. ([run-long-work-in-one-repo](../how-to/run-long-work-in-one-repo.md))
- **behind main**: The state of a task whose branch the board could not bring up to date with main, because the fetch failed. ([how-work-gets-done-main](../explanation/how-work-gets-done-main.md), [troubleshooting-behind-main](../explanation/troubleshooting-behind-main.md))
- **best of three**: Three attempts on one task by one agent. ([attempts](../explanation/attempts.md))
- **board**: The one list of tasks and the one chat that herdr-projects shows in a Herdr tab or popup. ([board-header-and-list](../reference/board-header-and-list.md))
- **board key**: The Herdr key that opens the board, goes to the running board, or takes you back to the pane you came from. ([install-details](../reference/install-details.md), [board-tab-and-popup](../reference/board-tab-and-popup.md), [keys](../reference/keys.md), [checked-on-herdr-popup](../contributing/checked-on-herdr-popup.md))
- **board setting**: A setting under `board.`, `coordinator.` or `watcher.` that controls the board itself; it is for you only. ([settings](../reference/settings.md))
- **Boards section**: The section of the web UI sidebar, under Agents, with one entry for each machine that has a board and one for All machines. ([web-ui](../reference/web-ui.md))
- **board tab**: The Herdr tab named `board` that the board key opens in the current space. ([board-tab-and-popup](../reference/board-tab-and-popup.md))
- **build cache**: The one sccache on a machine that keeps the compiled libraries for all worktrees, in `~/.cache/herdr-projects`. ([testing-build-cache](../contributing/testing-build-cache.md))
- **built in**: The value that a setting has when no place gives a value. ([settings](../reference/settings.md))
## C

- **cannot go on**: The state of an agent whose provider is down after five tries, that is out of allowance or logged out, or whose work still fails after its rounds. ([fallback](../explanation/fallback.md))
- **caret**: The `▾` (open) or `▸` (folded) mark of a list heading. ([list-headings](../reference/list-headings.md))
- **chat**: The box at the bottom of the board that talks to the Coordinator or to the agent of the selected task. ([board-chat](../reference/board-chat.md))
- **chat box**: The box at the foot of the web board that you type in; it copies the chat input of the web UI. ([web-chat-box](../reference/web-chat-box.md))
- **choice**: A setting value that is one word from a list; "off" is always `none`. ([settings](../reference/settings.md))
- **chord**: A key with its modifier, for example `ctrl+o`. ([add-your-own-keys](../how-to/add-your-own-keys.md))
- **clean-up**: The removal of the space, worktree and branch of a task when the task is done. ([how-work-gets-done-clean-up](../explanation/how-work-gets-done-clean-up.md))
- **closing note**: The file `closing-note.md` that an agent writes in the task's own folder, which the board uses as a fallback sign that a turn ended. ([how-work-gets-done](../explanation/how-work-gets-done.md), [agent-programs](../reference/agent-programs.md))
- **condition**: The `--if` pattern that limits an event trigger to some tasks. ([automations](../reference/automations.md))
- **condition (`--after`)**: A list of tasks that must be done before a task can start; by itself it never starts the task. ([after](../explanation/after.md))
- **conversation panel**: The panel over the chat box that shows the newest message and, when opened, the rest of the conversation. ([web-chat-box](../reference/web-chat-box.md))
- **Coordinator**: The agent in the chat of the board that changes what you type into tasks, in one conversation that stays open. ([roles](../reference/roles.md), [coordinator](../explanation/coordinator.md))
- **could not start**: The state of a task whose agent did not start; the row says `Could not start: …` with the reason. ([could-not-start](../explanation/could-not-start.md))
- **council**: Attempts on one task by different agents, with a judge that picks. ([attempts](../explanation/attempts.md))
- **crate**: One of the five parts of the code (sw-core, sw-flow, sw-board, sw-web, herdr-projects) that the compiler builds separately; a crate uses only the crates below it. ([crates](../contributing/crates.md))
## D

- **delegate**: To put an agent on a task, so that the agent works on it. ([how-work-gets-done](../explanation/how-work-gets-done.md))
- **done**: What the board calls done: work merged, a research task finished with its report, or a task marked done. ([after-options](../reference/after-options.md))
- **draft**: The text in the chat box that is not sent yet and keeps who it is for. ([board-chat](../reference/board-chat.md), [keys](../reference/keys.md))
- **draft lock**: The rule that keeps a draft with the receiver that the switch named when you typed the first word. ([web-chat-box](../reference/web-chat-box.md))
## E

- **effort**: How much an agent thinks, `default` or a level such as `low`, `medium` or `high`. ([models](../reference/models.md))
- **extras**: The marked blocks in the Herdr config and the how-to files in the skills folders of the agent programs; `herdr-projects remove-extras` removes them. ([remove](../how-to/remove.md))
## F

- **fact**: A label and its value on a row of the task panel. ([task-panel](../reference/task-panel.md))
- **fake Herdr**: The stand-in for Herdr in the tests, whose whole world is one JSON record. ([testing-light-tests](../contributing/testing-light-tests.md))
- **fallback agent**: An agent on the `worker.fallback` or `reviewer.fallback` list that takes over when the agent on a task cannot go on. ([fallback](../explanation/fallback.md))
- **fast mode**: The faster service tier of Codex, which uses more of your allowance; it has no effect on other programs. ([models](../reference/models.md))
- **finding**: One fault that the reviewer lists, with a severity, a subject, a place, a proof and a `preexisting` flag. ([gate-rules](../reference/gate-rules.md))
- **finish line**: The steps that the board does when the turn of an agent ends: commit, catch up with main, check, pull request, review and approval. ([how-work-gets-done](../explanation/how-work-gets-done.md))
- **firing**: One time that an automation fires, which makes a task or is skipped. ([automations](../reference/automations.md))
- **first run**: Any run of a new version (the board, a command, or the hook that Herdr calls), which does the first-run setup, the takeover and the one-time move. ([move-from-an-old-install](../how-to/move-from-an-old-install.md))
- **first-run screen**: The screen that the board shows the first time it opens inside Herdr, which offers a key and sidebar rows for `config.toml`. ([install-details](../reference/install-details.md))
- **fix round**: A later round of a task that starts when a review, a failed check or the checks of GitHub send the work back. ([shorter-checks](../explanation/shorter-checks.md))
- **floor**: The minimum WCAG 2 contrast ratio that a grey must have against its background, normally 4.5 to 1 and never less than 3 to 1. ([colours](../reference/colours.md))
- **fold**: To hide the tasks under a list heading; the heading then shows `▸` and a count. ([list-headings](../reference/list-headings.md))
- **followed sidebar**: The Herdr sidebar that the board uses for the order of the machines. ([machine-order](../explanation/machine-order.md))
- **free slot**: A place for one more agent to work, of which there are at most `board.agents`. ([board-rows](../reference/board-rows.md))
- **front door**: A small server of the board that stands in front of the web UI, sends its requests on, and adds the Boards section. ([old-installs-takeover](../reference/old-installs-takeover.md), [web-ui](../explanation/web-ui.md))
- **full access**: The default mode in which an agent program runs any command as you without asking first. ([how-work-gets-done](../explanation/how-work-gets-done.md))
## G

- **gate**: The fixed rules with which the board decides from the findings of the reviewer if work passes. ([gate](../explanation/gate.md))
- **green**: The state of work whose check passed, whose review passed the gate, whose branch is up to date with main and whose pull request has no failed check or requested change. ([gate-rules](../reference/gate-rules.md))
- **greyed-out words**: Words in the theme's grey `overlay0`, such as a task number, a time or a label. ([colours](../reference/colours.md))
- **grip**: The handle on the top edge of the chat box card that makes the box taller. ([web-chat-box](../reference/web-chat-box.md))
- **group**: The rows under one heading of the list: `needs you`, `ready to approve`, `working`, `to do`, `done` or `automations`. ([board-header-and-list](../reference/board-header-and-list.md))
- **guard**: The step of the check (`herdr-projects docs --guard`) that checks the rules of the repo: file sizes, the comment at the top, chords only in the keys' table, no made table out of date. ([automations](../reference/automations.md), [testing](../contributing/testing.md))
## H

- **hand-over**: The step in which the next agent on the fallback list takes over a task in the same worktree, branch and Herdr pane. ([fallback](../explanation/fallback.md))
- **header**: The top line of the board that shows whose tasks show and how many tasks are in each group. ([board-header-and-list](../reference/board-header-and-list.md))
- **headless one-shot**: A command of an agent program that answers one time and ends; the Watcher needs one. ([agent-programs](../reference/agent-programs.md))
- **headline**: The bold title of a part of the task panel, on a thin rule. ([task-panel](../reference/task-panel.md))
- **health check**: The command `herdr-projects doctor`, which examines the usual problems and tells what to do for each. ([troubleshooting-checks-and-logs](../reference/troubleshooting-checks-and-logs.md))
- **helper**: A sub-agent or background agent to which an agent gives part of its work; its finish does not end the turn of the agent. ([how-work-gets-done](../explanation/how-work-gets-done.md))
- **helper (subagent)**: An agent that the agent on a task starts with the own tools of its program, for example a Claude Code subagent. ([agent-programs](../explanation/agent-programs.md))
- **Herdr integration**: The part of Herdr that tells whether an agent program works or waits; you install it with `herdr integration install <name>`. ([agent-programs](../reference/agent-programs.md))
- **Herdr's dot**: The state icon in Herdr's sidebar, which is Herdr's own reading of the pane. ([sidebar](../reference/sidebar.md))
- **Herdr web UI**: A separate Herdr plugin that shows Herdr in a browser; the web board runs only if it is installed. ([web-ui](../explanation/web-ui.md))
- **hint line**: The bottom line of the board, which shows a maximum of four hints for the keys that matter now. ([board-chat](../reference/board-chat.md), [keys-hint-line](../reference/keys-hint-line.md))
- **home folder**: The one folder, `~/.herdr-projects/`, that holds everything that the board keeps for you, the same on every machine. ([home-folder](../reference/home-folder.md))
- **home space**: The Herdr space labelled `~`, which holds the `Coordinator` tab. ([coordinator](../explanation/coordinator.md), [checked-on-herdr-tabs](../contributing/checked-on-herdr-tabs.md))
- **how-to file**: The `herdr-projects` file in the skills folder of each agent program, which tells the agent how to use the board. ([home-folder](../reference/home-folder.md))
## I

- **instructions**: Your own words for an agent, which add up across the places and come after the fixed rules of the board. ([settings](../reference/settings.md))
## K

- **keeper**: The background process of the board that looks after the tasks of every session and updates itself to a new program file. ([install-details](../reference/install-details.md), [how-work-gets-done-clean-up](../explanation/how-work-gets-done-clean-up.md), [troubleshooting-checks-and-logs](../reference/troubleshooting-checks-and-logs.md))
- **kept build**: The build of the main line that a new worktree starts from, in `~/.cache/herdr-projects/warm/<repo>/`. ([testing-build-cache](../contributing/testing-build-cache.md))
- **kept frame**: A whole frame of the board, kept in `tests/frames/`, that a test compares with the board as it draws now. ([testing-light-tests](../contributing/testing-light-tests.md))
## L

- **launcher**: The file `~/.herdr-projects/data/bin/herdr-projects` that runs the program; agents and hooks get it by its full path. ([install-details](../reference/install-details.md))
- **leftover**: A worktree or a warm build that no task needs any more, which the health check names. ([troubleshooting-checks-and-logs](../reference/troubleshooting-checks-and-logs.md))
- **legacy folders**: The `-legacy` folders (for example `~/.herdr-projects-legacy`) where the first run sets aside herdr-projects 0.2. ([old-installs-takeover](../reference/old-installs-takeover.md))
- **light test**: A test that calls the code in its own process, starts no program except git, and never sleeps. ([testing-light-tests](../contributing/testing-light-tests.md))
- **limit**: A cap on what a task can cost (`limit.time`, `limit.tokens`); at a limit the agent stops, the work stays and the task waits for you. ([limits](../explanation/limits.md))
- **limit's mark**: The mark (`limitStop`) on a stop that the board made at a limit; it goes when you stop the task yourself. ([limits](../reference/limits.md))
- **link step**: The change that points the shared link of the web UI at the front door (the Yes answer, or `herdr-projects web-ui link`). ([web-ui](../explanation/web-ui.md))
- **list heading**: A line in the task list that names a machine, a repo or a status, with the tasks of that group under it. ([list-headings](../reference/list-headings.md))
- **list of agents**: Agents with commas between them, in the order that the board tries them, or `none`. ([settings](../reference/settings.md))
- **live task**: A task under `needs you`, `ready to approve` or `working`, which has a row of two lines or more. ([board-rows](../reference/board-rows.md))
- **Local**: The first entry of the Herdr machines sidebar, the machine where that Herdr window runs. ([machine-order](../explanation/machine-order.md))
- **login shell**: The shell that SSH starts and that nobody types into; on another machine, it must find `herdr-projects`. ([set-up-another-machine](../how-to/set-up-another-machine.md))
- **look**: One call of the Watcher, which covers up to 8 working agents whose screen changed. ([watcher](../explanation/watcher.md))
## M

- **machine coordinator**: A coordinator for one machine as a whole (its repos, folders, tasks and agents), opened with `Space` on the heading of the machine. ([machine-and-project-coordinators](../explanation/machine-and-project-coordinators.md))
- **machine heading**: A list heading with the name of a machine, which shows when the board shows every machine. ([list-headings](../reference/list-headings.md))
- **machines sidebar**: The **machines** panel on the left of the Herdr window, whose order the board follows. ([machine-order](../explanation/machine-order.md))
- **made table**: A table in `docs/keys.md` or `docs/settings.md` that the check writes from the code; you must not edit it by hand. ([testing](../contributing/testing.md))
- **main**: The branch on which a task will land: the main of the remote, the named branch it started from, or the branch of your checkout. ([how-work-gets-done-main](../explanation/how-work-gets-done-main.md))
- **mark**: The one-column character at the start of a row that shows the state of a task. ([board-marks-and-colours](../reference/board-marks-and-colours.md))
- **marked blocks**: The blocks that the board writes into the Herdr `config.toml`, each between two marker lines. ([install-details](../reference/install-details.md))
- **`?` menu**: The menu that lists every key of the board, in which you can also pick a `Ctrl` or `Alt` line. ([keys](../reference/keys.md))
- **merge by itself**: The mode `merge.by: itself`, in which green work merges without your yes. ([gate](../explanation/gate.md))
- **missed tick**: A time of a schedule that passed while the machine or the keeper was off, or that the board found more than five minutes late; it does not fire. ([automations](../reference/automations.md))
- **model**: The name of a model of a program, written after the program with a slash, for example `codex/gpt-6.1-sol`. ([models](../reference/models.md))
- **mold**: The linker that the builds of this repo use on Linux when it is installed. ([linking](../contributing/linking.md))
- **More…**: The menu of the web board that holds the actions of the `?` menu that are not buttons already. ([web-ui](../reference/web-ui.md))
## N

- **nearest answer**: The value from the first place in the lookup order that has a value for the setting. ([settings](../explanation/settings.md))
- **Not started**: The state of a task whose text stays unsent in the box of its agent 40 seconds after the last enter. ([handing-over](../explanation/handing-over.md))
## O

- **one-time move**: The move of the board's files from the older `~/.local/state`, `~/.config` and `~/.local/share` places into `~/.herdr-projects/`. ([home-folder](../explanation/home-folder.md))
- **open task**: On the web board, a task shown whole, with every part of the task panel. ([web-ui](../reference/web-ui.md))
## P

- **pass**: The result of a review with no finding of the task's own at or above `review.mustFix`; it means "can merge". ([gate-rules](../reference/gate-rules.md))
- **PC**: A machine that the web UI knows: this machine, or a machine added in its Settings → Remote PCs. ([web-ui](../reference/web-ui.md))
- **picker**: Who chooses among the attempts that passed the check (`attempts.pick`): you or the reviewer. ([attempts-settings](../reference/attempts-settings.md))
- **pick one**: The state of a task that waits for you to choose one of its attempts; it stands under needs you. ([attempts-settings](../reference/attempts-settings.md))
- **placement**: The way that Herdr shows a plugin pane: `overlay`, `split`, `tab`, `zoomed` or `popup`. ([checked-on-herdr-popup](../contributing/checked-on-herdr-popup.md))
- **plan file**: The one file `plan.md` in the task's own folder on the board that holds the plan; it is never in the repo. ([plan-settings](../reference/plan-settings.md))
- **planner**: An agent of its own (`plan.agent: <program>`) that writes the plan in a tab named `Planner`, before the working agent builds. ([plan-settings](../reference/plan-settings.md))
- **plan ready**: The state of a task whose plan waits for your yes; it stands under needs you. ([plan-settings](../reference/plan-settings.md))
- **plan step**: The stage of a task in which an agent writes a plan and nothing in the repo can change. ([plan](../explanation/plan.md))
- **popup**: The board in the own framed popup window of Herdr, which `board.open popup` selects. ([board-tab-and-popup](../reference/board-tab-and-popup.md))
- **popup (of Herdr)**: The one plugin pane for a whole session that Herdr draws over the panes of the tab that it opened over. ([checked-on-herdr-popup](../contributing/checked-on-herdr-popup.md))
- **pre-existing finding**: A finding about a fault that was there before the task; it never sends the work back. ([gate-rules](../reference/gate-rules.md))
- **program**: The agent program that the board starts for a role, for example `claude`, `codex` or `pi`. ([models](../reference/models.md))
- **progress file**: The file that `HERDR_PROJECTS_PROGRESS` names, in which a check writes how far it has gone. ([check-progress-report](../how-to/check-progress-report.md))
- **project coordinator**: A coordinator for one repo that runs in your checkout of the repo, opened with `Space` on the heading of the repo. ([send-tasks-to-another-repo](../how-to/send-tasks-to-another-repo.md), [run-long-work-in-one-repo](../how-to/run-long-work-in-one-repo.md), [machine-and-project-coordinators](../explanation/machine-and-project-coordinators.md))
- **provider error**: An error from the provider of the model that cuts off a turn; it either stops by itself or waiting cannot fix it. ([provider-errors](../explanation/provider-errors.md))
## Q

- **question**: The box that the board shows before a merge or the removal of a task, answered with `y` or `n`. ([keys](../reference/keys.md))
- **queued**: The state of a task that waits for a free slot of `board.agents`, or that is tabbed with `--after` and waits for other tasks; it is pink and has the `◌` mark. ([colours](../reference/colours.md))
- **queued after**: The state of a task that you tabbed with `--after` while the tasks it waits for are not all done; it starts by itself when they are. ([after-options](../reference/after-options.md))
- **queue place**: The `Q#` number of a task that waits for a free slot; `Q#1` gets the next slot. ([board-rows](../reference/board-rows.md))
- **`quick` profile**: The build profile of the check, with no optimisations and no debug data for the code of the program. ([testing](../contributing/testing.md))
## R

- **read-only mount**: The mount that the board starts the plan's writer in, where the task's worktree and its git folder cannot be written. ([plan](../explanation/plan.md))
- **real-program test**: A test whose name starts with `real_programs_` and that starts the real program, because the program itself is what it tests. ([testing-light-tests](../contributing/testing-light-tests.md))
- **remote view**: A `herdr terminal attach` of the pane of the board, which sets that pane to its own size. ([board-narrow-window](../reference/board-narrow-window.md))
- **repo for new tasks**: The repo that the header names, where a new task goes: the repo that you opened the board in. ([send-tasks-to-another-repo](../how-to/send-tasks-to-another-repo.md))
- **repo key**: The short key of 2 to 5 capital letters or digits that names a repo in task tags (`HSW`). ([task-numbers-repo-keys](../reference/task-numbers-repo-keys.md))
- **report**: A document that an agent writes for you to read, kept in the task's own folder among the reports of the board and never committed. ([research-tasks](../reference/research-tasks.md))
- **repo's own file**: The file `.herdr-projects.json`, committed in a repo, that can hold only seven settings. ([settings](../reference/settings.md))
- **research-only task**: A task with `research.only` set to `true`, which changes nothing in the repo and whose result is a report. ([research-tasks](../reference/research-tasks.md))
- **retry**: The action (the "yes" key, or `herdr-projects retry <id>`) that starts the program of the agent again and gives it the same text. ([handing-over](../explanation/handing-over.md))
- **reviewer**: The agent that attacks the finished work, to find what is wrong. ([roles](../reference/roles.md))
- **review round**: The number of a review of a task, shown as `R#2` and in the tab name `Reviewer #2`. ([tabs](../reference/tabs.md))
- **review tag**: The `R#` tag on a row that shows the review that a task is on or last had, and by its colour how it went. ([board-marks-and-colours](../reference/board-marks-and-colours.md))
- **role**: The job of an agent that the board starts: working agent, reviewer, Watcher or Coordinator. ([roles](../reference/roles.md))
- **round**: One time that a failed review or check goes back to the agent; `review.rounds` sets the limit. ([gate-rules](../reference/gate-rules.md))
- **run**: One stretch of an agent's work, from when it is set going until that turn ends; for the reviewer, one review. ([limits](../reference/limits.md))
- **runaway**: An automation that makes more than 10 tasks in an hour, which turns itself off. ([automations](../reference/automations.md))
## S

- **scripted agent**: A stand-in agent in the tests that plays the steps that the test gives its role. ([testing-light-tests](../contributing/testing-light-tests.md))
- **selection**: The one selected row, marked by a bar in the gutter (`▌`), that the keys act on. ([board-header-and-list](../reference/board-header-and-list.md))
- **Send + start**: The chip beside the round Send that sends the words and puts an agent on what they add, as `Tab` does. ([web-chat-box](../reference/web-chat-box.md))
- **set-aside**: The `set-aside/` folder that holds the copy of a file that a move did not keep in use. ([home-folder](../reference/home-folder.md))
- **settings page**: The full-screen page of the settings of one task, opened with `Ctrl+E`. ([task-panel](../reference/task-panel.md))
- **severity**: The level of a finding: `critical`, `major`, `minor` or `trivial`. ([gate-rules](../reference/gate-rules.md))
- **shared link**: The private `tailscale serve` link of the web UI, which the link step points at the front door. ([web-ui](../explanation/web-ui.md))
- **share of the cores**: The cores of the machine divided by `board.agents`, rounded up, that the build and the tests of one task use. ([testing](../contributing/testing.md))
- **shorter check**: A check that the board runs on a fix round in place of the full check, named in `check.again`; in this repo it runs only the test files that the change since a commit reaches. ([shorter-checks](../explanation/shorter-checks.md))
- **sidebar entry**: The three rows that the board writes in Herdr's sidebar for one live agent. ([sidebar](../reference/sidebar.md))
- **sidebar rows**: The config blocks of the plugin that make Herdr's sidebar show all three rows of an entry. ([sidebar](../reference/sidebar.md))
- **space**: The Herdr workspace that a task with an agent gets, with one tab for each helper. ([tabs](../reference/tabs.md))
- **spec tool**: A tool such as OpenSpec that writes a proposal and specs before the build; it works with the board through settings. ([use-spec-tools](../how-to/use-spec-tools.md))
- **stand-in**: A key that the board makes for a repo when another repo already has or had the key that the repo gives. ([task-numbers-repo-keys](../reference/task-numbers-repo-keys.md))
- **standing rules**: The rules of the Coordinator, which the board writes as `AGENTS.md` in the folder where the Coordinator starts. ([agent-programs](../reference/agent-programs.md), [coordinator](../explanation/coordinator.md))
- **status colour**: The colour that shows the state of a task on its mark, its group heading, its bar and its agent's text box. ([colours](../reference/colours.md))
- **step in**: To act on a task yourself, with a reply, `Ctrl+R`, or text in the terminal of the agent. ([fallback](../explanation/fallback.md))
- **step of the board**: Work that the board itself does on a task (start, set up, run the check, push, merge, or wait), shown with `◇`. ([board-marks-and-colours](../reference/board-marks-and-colours.md))
- **stepping in**: Your own action on a task (a reply, a turn in its terminal, delegating it again) that starts the finish line again. ([how-work-gets-done](../explanation/how-work-gets-done.md))
- **Stopped without a summary**: The state of a task whose turn the board cannot confirm as ended without a hook. ([how-work-gets-done](../explanation/how-work-gets-done.md))
- **sw**: The name of the library of herdr-projects; the tests reach every module as `sw::<module>`. ([crates](../contributing/crates.md))
- **switch**: The button over the chat box that names who gets your words, and flips between the Coordinator and the agent of the selected task. ([settings](../reference/settings.md), [web-chat-box](../reference/web-chat-box.md))
## T

- **tab**: A Herdr tab in the space of a task, for one helper (`Agent`, `Reviewer #1`, `Planner`, `Plan review`, `Picker`). ([tabs](../reference/tabs.md))
- **table of programs**: The table in the board that tells how each program lists its models and effort levels. ([models-new-program](../contributing/models-new-program.md))
- **tag**: The short name of a task on a row, the repo key and the number joined by a dash (`HSW-134`). ([board-rows](../reference/board-rows.md), [task-numbers](../reference/task-numbers.md))
- **takeover**: What the first run does to herdr-superwork (moved and carried on) and to herdr-projects 0.2 (set aside). ([old-installs-takeover](../reference/old-installs-takeover.md))
- **task id**: The full name of a task, the repo name and the number (`herdr-projects/134`), which never changes. ([task-numbers](../reference/task-numbers.md))
- **task panel**: The part of the board beside the list that shows all the data about the selected task. ([task-panel](../reference/task-panel.md))
- **`terminal` theme**: The theme that paints with the sixteen colours of the terminal. ([colours](../reference/colours.md))
- **terminal view**: The view of the web UI that shows a pane as a terminal; the terminal board can show there. ([web-ui-terminal-view](../reference/web-ui-terminal-view.md))
- **text box**: The box under the list where you type a message to the Coordinator or to an agent; its frame has the colour of the recipient. ([colours-text-box](../reference/colours-text-box.md), [keys](../reference/keys.md))
- **the check (`npm run check`)**: The one command that builds the program, writes the made tables, checks the rules of the repo and runs the tests. ([testing](../contributing/testing.md))
- **title**: The short name of a task, at most 60 characters (cells). ([titles](../reference/titles.md))
- **tracking doc**: A doc in the repo, or a pull request or issue, that says the goal of big work and what is left; each task's description names it. ([run-long-work-in-one-repo](../how-to/run-long-work-in-one-repo.md))
- **trigger**: The schedule or the board event that makes an automation fire. ([automations](../reference/automations.md))
- **try (retry)**: One of the five times that the board tells an agent to continue after a provider error, after 1, 2, 4 and 8 minutes. ([provider-errors](../explanation/provider-errors.md))
- **turn-end hook**: A hook that the plugin gives an agent at launch, which runs `herdr-projects turn-ended` at the end of each turn. ([how-work-gets-done](../explanation/how-work-gets-done.md))
## U

- **under-way words**: The words such as `Starting…` that a row shows at once after a key acts on a task, until the work ends. ([under-way](../explanation/under-way.md))
## W

- **waiting**: The state of an agent that ran `herdr-projects report <id> --state blocked` and stopped; it is not finished. ([waiting-for-you](../explanation/waiting-for-you.md))
- **warm start**: The copy of the last kept build of the main line into a new worktree, so that cargo builds only what changed. ([testing-build-cache](../contributing/testing-build-cache.md))
- **Watcher**: The small hidden model call that writes the "doing now" line and the progress guess of each task. ([roles](../reference/roles.md), [watcher](../explanation/watcher.md))
- **watching device**: A device that the web UI paired to watch; it can look at the board but cannot change anything. ([web-ui](../reference/web-ui.md))
- **web board**: The board as a part of the Herdr web UI, with buttons and touch in place of keys. ([web-ui](../reference/web-ui.md))
- **whole check**: The check that runs all the tests (`npm run check -- --all`). ([testing](../contributing/testing.md))
- **whole task**: The full-screen view of one task, opened with `Alt+T`. ([task-panel](../reference/task-panel.md))
- **working agent**: The agent that does the work of a task (settings `worker.`). ([roles](../reference/roles.md))
- **work setting**: A setting that controls who works, where, how the work is checked and how it lands; you can set it for a task, a repo or you. ([settings](../reference/settings.md))
- **worktree space**: A Herdr space with its own git worktree and branch, in which the agent of a task works. ([how-work-gets-done](../explanation/how-work-gets-done.md))
- **world**: What `world()` gives a test: a board folder of its own, a git repo, a fake Herdr and a home folder of its own. ([testing-light-tests](../contributing/testing-light-tests.md))
## Y

- **your entry for a repo**: The `repos` section of your `config.json`, with the settings that you give for one repo. ([settings](../reference/settings.md))

## Related pages

- [How these pages are written](../style.md)
- [All pages](../index.md)
