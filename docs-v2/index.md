# herdr-projects documentation

herdr-projects is a work board in Herdr. You use it to give tasks to agents, follow them and merge their work.
This page lists all the pages, in five groups. Each group is for a different need.

| If you want to | Go to |
| --- | --- |
| Learn the board from the start | [Tutorials](#tutorials) |
| Do one specific job | [How-to guides](#how-to-guides) |
| Find a key, a setting, a command or a part of the screen | [Reference](#reference) |
| Understand how a part works and why | [Explanation](#explanation) |
| Change the code of herdr-projects | [Contributing](#contributing) |

Note: [How these pages are written](style.md) gives the writing rules. [Coverage](coverage.md) shows where each part
of the old pages in `docs/` is now.

## Tutorials

Do these lessons in this order.

1. [Your first board](tutorials/first-board.md): install the plugin and open the board.
2. [Your first task](tutorials/first-task.md): add a task, give it to an agent and merge its work.
3. [Your first look at the web board](tutorials/web-board.md): open the board in the Herdr web UI.

## How-to guides

### Install and set up

- [Install and update herdr-projects](how-to/install-and-update.md)
- [Remove herdr-projects](how-to/remove.md)
- [Move from an old install](how-to/move-from-an-old-install.md)
- [Move the home folder](how-to/move-the-home-folder.md)
- [Set up another machine](how-to/set-up-another-machine.md)
- [Set up the web board](how-to/set-up-the-web-ui.md)
- [Choose the board's theme](how-to/colours-choose-a-theme.md)

### Give work to agents

- [Delegate a task and merge its work](how-to/delegate-and-merge.md)
- [Merge work by itself](how-to/turn-on-auto-merge.md)
- [Get a plan before the build](how-to/get-a-plan-first.md)
- [Run several attempts on one task](how-to/run-several-attempts.md)
- [Chain tasks with `--after`](how-to/chain-tasks.md)
- [Make a research task](how-to/make-a-research-task.md)
- [Use a spec tool (OpenSpec)](how-to/use-spec-tools.md)
- [Run long work in one repo](how-to/run-long-work-in-one-repo.md)
- [Send tasks to another repo](how-to/send-tasks-to-another-repo.md)
- [Talk to a coordinator](how-to/talk-to-a-coordinator.md)
- [Make an automation](how-to/make-an-automation.md)

### Change how the board works

- [Change a setting](how-to/change-a-setting.md)
- [Choose a model and an effort](how-to/choose-a-model-and-effort.md)
- [Set a fallback agent](how-to/set-a-fallback-agent.md)
- [Set limits on time and tokens](how-to/set-limits.md)
- [Add your own keys](how-to/add-your-own-keys.md)
- [Change a repo key](how-to/task-numbers-change-a-repo-key.md)
- [Set a shorter check for fix rounds](how-to/shorter-checks-set-up.md)
- [Make a check report its progress](how-to/check-progress-report.md)

### Use the board

- [Find a task](how-to/find-a-task.md)
- [Fold list headings](how-to/fold-list-headings.md)

### Fix a problem

- [Troubleshoot the board](how-to/troubleshoot.md)
- [Fix an agent that could not start](how-to/could-not-start-fix.md)
- [Fix a task that seems stuck](how-to/troubleshooting-stuck-task.md)
- [Fix a task that is behind main](how-to/troubleshooting-behind-main.md)

## Reference

### The board screen

- [The board: the header and the list](reference/board-header-and-list.md)
- [The rows of the board](reference/board-rows.md)
- [Marks and colours on a row](reference/board-marks-and-colours.md)
- [The board in a narrow window](reference/board-narrow-window.md)
- [Automations in the list](reference/board-automations.md)
- [The chat of the board](reference/board-chat.md)
- [The messages in the chat](reference/board-chat-messages.md)
- [The board key, the board tab and the popup](reference/board-tab-and-popup.md)
- [The task panel](reference/task-panel.md)
- [List headings](reference/list-headings.md)
- [Task numbers](reference/task-numbers.md)
- [Repo keys](reference/task-numbers-repo-keys.md)
- [Titles](reference/titles.md)
- [Colours](reference/colours.md)
- [The colour of the text box](reference/colours-text-box.md)
- [The mouse](reference/mouse.md)

### Keys, settings and commands

- [Keys](reference/keys.md)
- [The hint line](reference/keys-hint-line.md)
- [Settings](reference/settings.md)
- [Commands](reference/commands.md)
- [Roles](reference/roles.md)
- [Models and effort](reference/models.md)
- [Glossary](reference/glossary.md)

### Agents and tasks

- [Agent programs](reference/agent-programs.md)
- [The fallback settings](reference/fallback-settings.md)
- [Rules of the gate](reference/gate-rules.md)
- [Plan settings](reference/plan-settings.md)
- [Attempts settings](reference/attempts-settings.md)
- [`--after` options](reference/after-options.md)
- [Limits](reference/limits.md)
- [Research tasks and reports](reference/research-tasks.md)
- [Automations](reference/automations.md)
- [Automations on the board](reference/automations-board.md)

### Herdr, files and the web board

- [Herdr's sidebar](reference/sidebar.md)
- [A task's tabs in Herdr](reference/tabs.md)
- [The home folder](reference/home-folder.md)
- [Install details](reference/install-details.md)
- [What the takeover of an old install does](reference/old-installs-takeover.md)
- [The health check and the logs](reference/troubleshooting-checks-and-logs.md)
- [The web board](reference/web-ui.md)
- [The parts of the web board](reference/web-ui-parts.md)
- [The web board's chat box](reference/web-chat-box.md)
- [The terminal board in the web UI's terminal view](reference/web-ui-terminal-view.md)

## Explanation

### How work gets done

- [How work gets done](explanation/how-work-gets-done.md)
- [The gate](explanation/gate.md)
- [Staying current with main](explanation/how-work-gets-done-main.md)
- [GitHub's checks](explanation/how-work-gets-done-github-checks.md)
- [When a task is done](explanation/how-work-gets-done-clean-up.md)
- [Chatting with a finished task](explanation/how-work-gets-done-chat.md)
- [Behind main](explanation/troubleshooting-behind-main.md)
- [A plan first](explanation/plan.md)
- [Several attempts, and one kept](explanation/attempts.md)
- [One task after another](explanation/after.md)
- [How limits work](explanation/limits.md)
- [Shorter checks on fix rounds](explanation/shorter-checks.md)
- [The progress of the check](explanation/check-progress.md)

### Agents and roles

- [How the roles work](explanation/roles.md)
- [How the board uses agent programs](explanation/agent-programs.md)
- [Handing text to an agent](explanation/handing-over.md)
- [An agent that waits for you is not finished](explanation/waiting-for-you.md)
- [When an agent could not start](explanation/could-not-start.md)
- [A turn cut off by the provider](explanation/provider-errors.md)
- [The fallback agent](explanation/fallback.md)
- [The Coordinator](explanation/coordinator.md)
- [Machine and project coordinators](explanation/machine-and-project-coordinators.md)
- [The Watcher](explanation/watcher.md)
- [How automations work](explanation/automations.md)

### The board and Herdr

- [Why the board looks as it does](explanation/board-design.md)
- [The size of the board](explanation/board-size.md)
- [What a key shows at once](explanation/under-way.md)
- [How settings work](explanation/settings.md)
- [How the task panel is made](explanation/task-panel.md)
- [Why the board uses these colours](explanation/colours.md)
- [Why the board takes the whole mouse](explanation/mouse.md)
- [Why titles are short](explanation/titles.md)
- [How the board writes Herdr's sidebar](explanation/sidebar.md)
- [The home folder: how it works and why](explanation/home-folder.md)
- [Other machines and several boards](explanation/other-machines.md)
- [Machine order](explanation/machine-order.md)
- [How the web board works](explanation/web-ui.md)
- [Why the web board's chat box is built this way](explanation/web-chat-box.md)

## Contributing

- [Run the tests](contributing/run-the-tests.md)
- [How the tests work](contributing/testing.md)
- [Light tests](contributing/testing-light-tests.md)
- [The build cache and the warm start](contributing/testing-build-cache.md)
- [Crates](contributing/crates.md)
- [Linking with mold](contributing/linking.md)
- [The code of the agent programs](contributing/agents-code.md)
- [Add a model list for a new program](contributing/models-new-program.md)
- [The code of the retries and the fallback](contributing/fallback.md)
- [Limits code](contributing/limits-code.md)
- [Testing the web board](contributing/web-ui.md)
- [What was checked on Herdr 0.9.3](contributing/herdr-0.9.3-facts.md)
  - [Agents and their start](contributing/checked-on-herdr-agents.md)
  - [Plugin panes and the keyboard](contributing/checked-on-herdr-panes.md)
  - [The popup](contributing/checked-on-herdr-popup.md)
  - [The sidebar, the config and the screen](contributing/checked-on-herdr-sidebar.md)
  - [Terminal sizes, tabs and the Coordinator](contributing/checked-on-herdr-tabs.md)
  - [The turn end and the conversation files](contributing/checked-on-herdr-turn-end.md)
