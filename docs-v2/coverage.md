# Coverage: from docs/ to docs-v2/

This page shows where each part of the old pages in `docs/` is now in `docs-v2/`. Use it to find a topic that
you know from `docs/`. Each row is one heading of an old page. "intro" is the text before the first heading.

## [`docs/after.md`](../docs/after.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [how-to/chain-tasks.md](how-to/chain-tasks.md) § Add a task that waits<br>[how-to/chain-tasks.md](how-to/chain-tasks.md) § Delegate a task that waits<br>[how-to/chain-tasks.md](how-to/chain-tasks.md) § Tell the Coordinator<br>[reference/after-options.md](reference/after-options.md) § The two kinds of wait<br>[reference/after-options.md](reference/after-options.md) § What counts as done<br>[explanation/after.md](explanation/after.md) § A condition, not a start |
| One task after another: `--after` | [explanation/after.md](explanation/after.md) § One task after another (title)<br>[how-to/chain-tasks.md](how-to/chain-tasks.md) § Chain tasks with `--after` (title) |
| Not tabbed: a condition only | [explanation/after.md](explanation/after.md) § A condition, not a start<br>[reference/after-options.md](reference/after-options.md) § A condition only (not tabbed)<br>[how-to/chain-tasks.md](how-to/chain-tasks.md) § Start a task that has a condition only |
| Tabbed: queued after them | [reference/after-options.md](reference/after-options.md) § Queued after other tasks (tabbed)<br>[reference/after-options.md](reference/after-options.md) § The two kinds of wait<br>[reference/after-options.md](reference/after-options.md) § Refused waits<br>[explanation/after.md](explanation/after.md) § While the task waits |
| When the wait is over | [explanation/after.md](explanation/after.md) § When the wait is over<br>[reference/after-options.md](reference/after-options.md) § After the wait |
| When it cannot go on by itself | [explanation/after.md](explanation/after.md) § When the task cannot continue by itself<br>[how-to/chain-tasks.md](how-to/chain-tasks.md) § If a waiting task needs you<br>[reference/after-options.md](reference/after-options.md) § After the wait |
| Changing it, and starting early | [reference/after-options.md](reference/after-options.md) § Flags and commands<br>[how-to/chain-tasks.md](how-to/chain-tasks.md) § Change what a task waits for<br>[how-to/chain-tasks.md](how-to/chain-tasks.md) § Start a queued task early<br>[how-to/chain-tasks.md](how-to/chain-tasks.md) § Stop a queued task |
| Waiting for a task on another machine | [explanation/after.md](explanation/after.md) § Waiting for a task on another machine<br>[how-to/chain-tasks.md](how-to/chain-tasks.md) § Wait for a task on another machine<br>[reference/after-options.md](reference/after-options.md) § Names of tasks<br>[reference/after-options.md](reference/after-options.md) § Refused waits<br>[reference/after-options.md](reference/after-options.md) § Timing |

## [`docs/agents.md`](../docs/agents.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [reference/agent-programs.md](reference/agent-programs.md) § Which programs a role can use<br>[reference/agent-programs.md](reference/agent-programs.md) § The program names |
| Agent programs | [reference/agent-programs.md](reference/agent-programs.md) § Agent programs |
| Before a program's first task | [explanation/agent-programs.md](explanation/agent-programs.md) § Why you start a program one time yourself<br>[how-to/could-not-start-fix.md](how-to/could-not-start-fix.md) § Before the first task of a program |
| How a program is started | [reference/agent-programs.md](reference/agent-programs.md) § How the board starts a program<br>[explanation/agent-programs.md](explanation/agent-programs.md) § How the board starts a program<br>[contributing/agents-code.md](contributing/agents-code.md) § The table of programs |
| Helpers (subagents) | [explanation/agent-programs.md](explanation/agent-programs.md) § Helpers (subagents)<br>[contributing/agents-code.md](contributing/agents-code.md) § Helpers (subagents) |
| What the board knows about each program | [reference/agent-programs.md](reference/agent-programs.md) § What the other programs do not get<br>[explanation/agent-programs.md](explanation/agent-programs.md) § What the board knows about each program |
| What is refused, and why | [reference/agent-programs.md](reference/agent-programs.md) § What the board refuses when you set it<br>[reference/agent-programs.md](reference/agent-programs.md) § What the board says when the program starts<br>[explanation/agent-programs.md](explanation/agent-programs.md) § Why the board refuses some programs<br>[explanation/agent-programs.md](explanation/agent-programs.md) § Why some problems show only at start<br>[how-to/could-not-start-fix.md](how-to/could-not-start-fix.md) § `herdr did not see kilo come up in its pane …`<br>[how-to/could-not-start-fix.md](how-to/could-not-start-fix.md) § `omp needs herdr's own integration, …` |
| Not supported | [reference/agent-programs.md](reference/agent-programs.md) § Programs that the board does not support<br>[explanation/agent-programs.md](explanation/agent-programs.md) § Programs that Herdr starts without an integration<br>[contributing/agents-code.md](contributing/agents-code.md) § The table of programs |
| The programs | [reference/agent-programs.md](reference/agent-programs.md) § The table of programs |

## [`docs/attempts.md`](../docs/attempts.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [how-to/run-several-attempts.md](how-to/run-several-attempts.md) § Start several attempts<br>[how-to/run-several-attempts.md](how-to/run-several-attempts.md) § Pick the attempt to keep<br>[explanation/attempts.md](explanation/attempts.md) § One idea for best of three and for a council |
| Several attempts, and one kept | [explanation/attempts.md](explanation/attempts.md) § Several attempts, and one kept (title) |
| # best of three, by the usual agent; you pick (code comment) | [how-to/run-several-attempts.md](how-to/run-several-attempts.md) § Start several attempts |
| # a council of claude, codex and pi, with a named judge (code comment) | [how-to/run-several-attempts.md](how-to/run-several-attempts.md) § Start several attempts |
| # two models for every task in one repo (code comment) | [how-to/run-several-attempts.md](how-to/run-several-attempts.md) § Start several attempts |
| # which attempt to keep, when the task waits for you (code comment) | [how-to/run-several-attempts.md](how-to/run-several-attempts.md) § Pick the attempt to keep |
| The two settings | [reference/attempts-settings.md](reference/attempts-settings.md) § The two settings<br>[how-to/run-several-attempts.md](how-to/run-several-attempts.md) § Before you start |
| What happens | [explanation/attempts.md](explanation/attempts.md) § What happens<br>[explanation/attempts.md](explanation/attempts.md) § Why only one review<br>[explanation/attempts.md](explanation/attempts.md) § Why some worktrees stay<br>[explanation/attempts.md](explanation/attempts.md) § No race, no mix<br>[reference/attempts-settings.md](reference/attempts-settings.md) § Names of attempts<br>[reference/attempts-settings.md](reference/attempts-settings.md) § When an attempt is out<br>[reference/attempts-settings.md](reference/attempts-settings.md) § The pick<br>[how-to/run-several-attempts.md](how-to/run-several-attempts.md) § If no attempt passed |
| On the board | [reference/attempts-settings.md](reference/attempts-settings.md) § What the board shows<br>[reference/attempts-settings.md](reference/attempts-settings.md) § Keys and commands<br>[how-to/run-several-attempts.md](how-to/run-several-attempts.md) § Pick the attempt to keep<br>[how-to/run-several-attempts.md](how-to/run-several-attempts.md) § Talk to the attempts, or stop them |
| Research-only tasks | [reference/attempts-settings.md](reference/attempts-settings.md) § Research-only tasks |
| Beside the other settings | [explanation/attempts.md](explanation/attempts.md) § Attempts beside the other settings |

## [`docs/automations.md`](../docs/automations.md)

| Part of the old page | Where it is now |
| --- | --- |
| Automations | [explanation/automations.md](explanation/automations.md) § How automations work (title)<br>[reference/automations.md](reference/automations.md) § Automations (title) |
| intro | [explanation/automations.md](explanation/automations.md) § A saved task plus a trigger<br>[how-to/make-an-automation.md](how-to/make-an-automation.md) § Make an automation with the Coordinator<br>[how-to/make-an-automation.md](how-to/make-an-automation.md) § Make an automation on a schedule<br>[how-to/make-an-automation.md](how-to/make-an-automation.md) § Make an automation on a board event<br>[reference/automations.md](reference/automations.md) § The task that a firing makes |
| Triggers: a schedule | [reference/automations.md](reference/automations.md) § Triggers: a schedule<br>[explanation/automations.md](explanation/automations.md) § Why missed ticks are not caught up |
| Triggers: something that happened on the board | [reference/automations.md](reference/automations.md) § Triggers: something that happened on the board<br>[explanation/automations.md](explanation/automations.md) § Why an event fires once, and only after the wait starts<br>[explanation/automations.md](explanation/automations.md) § Why the event block is data |
| Only for some tasks: `--if` | [reference/automations.md](reference/automations.md) § Only for some tasks: `--if`<br>[how-to/make-an-automation.md](how-to/make-an-automation.md) § Make an automation on a board event<br>[how-to/make-an-automation.md](how-to/make-an-automation.md) § Change an automation |
| Commands | [reference/automations.md](reference/automations.md) § Commands<br>[how-to/make-an-automation.md](how-to/make-an-automation.md) § Change an automation<br>[how-to/make-an-automation.md](how-to/make-an-automation.md) § Turn an automation on or off<br>[how-to/make-an-automation.md](how-to/make-an-automation.md) § Fire an automation now<br>[how-to/make-an-automation.md](how-to/make-an-automation.md) § See what an automation did<br>[how-to/make-an-automation.md](how-to/make-an-automation.md) § Remove an automation |
| Its settings: the same as any task's | [reference/automations.md](reference/automations.md) § Its settings<br>[how-to/make-an-automation.md](how-to/make-an-automation.md) § Give the automation its own settings<br>[explanation/automations.md](explanation/automations.md) § Why its settings are the settings of a task |
| The guards | [reference/automations.md](reference/automations.md) § The guards<br>[explanation/automations.md](explanation/automations.md) § Why there are guards<br>[explanation/automations.md](explanation/automations.md) § Why the event block is data |
| Who fires them | [explanation/automations.md](explanation/automations.md) § Who fires them |
| On the board | [reference/automations-board.md](reference/automations-board.md) § Where automations show in the list<br>[reference/automations-board.md](reference/automations-board.md) § The row of an automation<br>[reference/automations-board.md](reference/automations-board.md) § Keys on the row<br>[reference/automations-board.md](reference/automations-board.md) § One automation, whole<br>[reference/automations-board.md](reference/automations-board.md) § Its settings page<br>[reference/automations-board.md](reference/automations-board.md) § A task that an automation made<br>[reference/automations-board.md](reference/automations-board.md) § Automations of other machines<br>[reference/automations-board.md](reference/automations-board.md) § The web board<br>[how-to/make-an-automation.md](how-to/make-an-automation.md) § Give the automation its own settings |
| Where it is kept | [reference/automations.md](reference/automations.md) § Where it is kept |

## [`docs/board-size.md`](../docs/board-size.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [explanation/board-size.md](explanation/board-size.md) § In short |
| The board's size | [explanation/board-size.md](explanation/board-size.md) § The size of the board |
| What does not change it: agents starting and ending | [explanation/board-size.md](explanation/board-size.md) § What does not change the size: agents that start and end |
| What does change it | [explanation/board-size.md](explanation/board-size.md) § What changes the size |
| A size that lasts a moment is not drawn | [explanation/board-size.md](explanation/board-size.md) § A size that lasts a moment is not drawn |
| A second Herdr window cannot be disconnected | [explanation/board-size.md](explanation/board-size.md) § A second Herdr window cannot be disconnected |
| A view somebody is looking through keeps the terminal | [explanation/board-size.md](explanation/board-size.md) § A view that somebody looks through keeps the terminal |
| The board loading its new version | [explanation/board-size.md](explanation/board-size.md) § The board loads its new version |
| On the real board, as it happened | [explanation/board-size.md](explanation/board-size.md) § On the real board, as it happened |

## [`docs/board.md`](../docs/board.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [reference/board-header-and-list.md](reference/board-header-and-list.md) § The board at a glance |
| The board: the list and the chat | [reference/board-header-and-list.md](reference/board-header-and-list.md) § The board: the header and the list |
| The header | [reference/board-header-and-list.md](reference/board-header-and-list.md) § The header |
| The list: five groups | [reference/board-header-and-list.md](reference/board-header-and-list.md) § The five groups<br>[reference/board-header-and-list.md](reference/board-header-and-list.md) § Moving through the list<br>[explanation/board-design.md](explanation/board-design.md) § The order of the groups |
| The list says what it hides | [reference/board-header-and-list.md](reference/board-header-and-list.md) § The list says what it hides |
| A list taller than the window | [reference/board-header-and-list.md](reference/board-header-and-list.md) § A list taller than the window<br>[explanation/board-design.md](explanation/board-design.md) § The order of the groups |
| Marks: where a task stands | [reference/board-marks-and-colours.md](reference/board-marks-and-colours.md) § The marks<br>[explanation/board-design.md](explanation/board-design.md) § Marks and not only colours |
| A task that needs you | [reference/board-marks-and-colours.md](reference/board-marks-and-colours.md) § A task that needs you<br>[explanation/board-design.md](explanation/board-design.md) § Marks and not only colours |
| A row | [reference/board-rows.md](reference/board-rows.md) § The parts of a row |
| Spacing | [reference/board-rows.md](reference/board-rows.md) § Spacing<br>[explanation/board-design.md](explanation/board-design.md) § Spacing |
| Live rows have two lines | [reference/board-rows.md](reference/board-rows.md) § Live rows have two lines<br>[reference/board-header-and-list.md](reference/board-header-and-list.md) § Moving through the list<br>[explanation/board-design.md](explanation/board-design.md) § Two lines for a live task |
| Colours on a row | [reference/board-marks-and-colours.md](reference/board-marks-and-colours.md) § The colours of a row<br>[reference/board-marks-and-colours.md](reference/board-marks-and-colours.md) § The blue<br>[explanation/board-design.md](explanation/board-design.md) § One meaning for each colour |
| The review tag, the pull request and the time | [reference/board-marks-and-colours.md](reference/board-marks-and-colours.md) § The review tag<br>[reference/board-marks-and-colours.md](reference/board-marks-and-colours.md) § The pull request<br>[reference/board-marks-and-colours.md](reference/board-marks-and-colours.md) § The time<br>[explanation/board-design.md](explanation/board-design.md) § Quiet tags on done tasks |
| A task that waits for a free slot | [reference/board-rows.md](reference/board-rows.md) § A task that waits for a free slot<br>[explanation/board-design.md](explanation/board-design.md) § Tasks that wait for a slot |
| While the board runs the check | [reference/board-rows.md](reference/board-rows.md) § While the board runs the check<br>[explanation/board-design.md](explanation/board-design.md) § One meaning for each colour |
| When the window is narrow | [reference/board-narrow-window.md](reference/board-narrow-window.md) § Rows hide the same parts together<br>[reference/board-narrow-window.md](reference/board-narrow-window.md) § The order in which parts give way<br>[explanation/board-design.md](explanation/board-design.md) § The narrow window |
| Narrow screens | [reference/board-narrow-window.md](reference/board-narrow-window.md) § Narrow screens<br>[reference/board-narrow-window.md](reference/board-narrow-window.md) § Long rows wrap<br>[reference/board-narrow-window.md](reference/board-narrow-window.md) § Very narrow screens<br>[explanation/board-design.md](explanation/board-design.md) § Three lines on a narrow screen<br>[explanation/board-design.md](explanation/board-design.md) § Long rows wrap<br>[explanation/board-design.md](explanation/board-design.md) § Why 80 columns |
| The selected row | [reference/board-header-and-list.md](reference/board-header-and-list.md) § The selected row<br>[reference/board-header-and-list.md](reference/board-header-and-list.md) § Where the selection goes after an action<br>[explanation/board-design.md](explanation/board-design.md) § The selection |
| Automations in the list | [reference/board-automations.md](reference/board-automations.md) § Where the automations are<br>[reference/board-automations.md](reference/board-automations.md) § The parts of a row<br>[reference/board-automations.md](reference/board-automations.md) § What gives way when the room is short<br>[reference/board-automations.md](reference/board-automations.md) § What is not on the row<br>[reference/board-automations.md](reference/board-automations.md) § Keys and other rules<br>[explanation/board-design.md](explanation/board-design.md) § Automation rows |
| The chat | [reference/board-chat.md](reference/board-chat.md) § Who the chat talks to |
| What the chat is made of | [reference/board-chat.md](reference/board-chat.md) § The parts of the chat |
| Only the words of the conversation | [reference/board-chat-messages.md](reference/board-chat-messages.md) § Only the words of the conversation<br>[explanation/board-design.md](explanation/board-design.md) § The chat |
| While a task is under review, the chat is its reviewer's | [reference/board-chat-messages.md](reference/board-chat-messages.md) § While a task is under review |
| The box holds one message: the newest, whoever said it | [reference/board-chat-messages.md](reference/board-chat-messages.md) § The box holds one message |
| Scrolling back through the conversation | [reference/board-chat-messages.md](reference/board-chat-messages.md) § Scrolling back through the conversation |
| Hiding the message | [reference/board-chat.md](reference/board-chat.md) § Hiding the message<br>[reference/board-chat.md](reference/board-chat.md) § When the chat folds by itself |
| Typed words keep who they are for | [reference/board-chat.md](reference/board-chat.md) § Typed words keep who they are for<br>[explanation/board-design.md](explanation/board-design.md) § The chat |
| Going into an agent | [reference/board-tab-and-popup.md](reference/board-tab-and-popup.md) § Going into an agent |
| The board key and the board's tab | [reference/board-tab-and-popup.md](reference/board-tab-and-popup.md) § The board key and the board tab<br>[reference/board-tab-and-popup.md](reference/board-tab-and-popup.md) § The name of the tab |
| The board as a popup | [reference/board-tab-and-popup.md](reference/board-tab-and-popup.md) § The board as a popup<br>[reference/board-tab-and-popup.md](reference/board-tab-and-popup.md) § Words in the popup stay with their repo<br>[reference/board-tab-and-popup.md](reference/board-tab-and-popup.md) § A board in a tab and a popup at the same time<br>[reference/board-tab-and-popup.md](reference/board-tab-and-popup.md) § When Herdr does not open the popup<br>[reference/board-tab-and-popup.md](reference/board-tab-and-popup.md) § The old meaning of popup |
| What still differs from Herdr's own menu | [reference/board-tab-and-popup.md](reference/board-tab-and-popup.md) § How the popup differs from the own menu of Herdr<br>[explanation/board-design.md](explanation/board-design.md) § The board tab and the popup |
| A board that looks cut off or half empty | [reference/board-narrow-window.md](reference/board-narrow-window.md) § A board that looks cut off or half empty<br>[explanation/board-design.md](explanation/board-design.md) § The board that looks cut off |

## [`docs/check-progress.md`](../docs/check-progress.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [explanation/check-progress.md](explanation/check-progress.md) § What the board shows, and where |
| What the task shows | [explanation/check-progress.md](explanation/check-progress.md) § What the task shows |
| The percent | [explanation/check-progress.md](explanation/check-progress.md) § The percent |
| The minutes left | [explanation/check-progress.md](explanation/check-progress.md) § The minutes left |
| When it stands still | [explanation/check-progress.md](explanation/check-progress.md) § When the check stands still |
| The colour | [explanation/check-progress.md](explanation/check-progress.md) § The colour |
| How a check reports its progress | [explanation/check-progress.md](explanation/check-progress.md) § How a check gives its progress<br>[how-to/check-progress-report.md](how-to/check-progress-report.md) § Before you start, § Write the progress, § What to write, § An example, § What the board does with the file |

## [`docs/checked-on-herdr.md`](../docs/checked-on-herdr.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [contributing/herdr-0.9.3-facts.md](contributing/herdr-0.9.3-facts.md) § (intro) |
| The facts (first table) | [contributing/herdr-0.9.3-facts.md](contributing/herdr-0.9.3-facts.md) § The facts (index)<br>[contributing/checked-on-herdr-popup.md](contributing/checked-on-herdr-popup.md) § Popup panes, § The popup for the board<br>[contributing/checked-on-herdr-panes.md](contributing/checked-on-herdr-panes.md) § Placements other than the popup, § The board key in a tab, and the keyboard<br>[contributing/checked-on-herdr-agents.md](contributing/checked-on-herdr-agents.md) § Events and status, § Hooks and processes, § Prompts, § Agent programs and versions |
| The facts (second table, "Checked on 2026-10-09 …") | [contributing/checked-on-herdr-turn-end.md](contributing/checked-on-herdr-turn-end.md) § (intro), § Claude Code, § Codex, § pi, § All three programs<br>[contributing/checked-on-herdr-sidebar.md](contributing/checked-on-herdr-sidebar.md) § The sidebar and metadata, § The config and the keys, § The screen<br>[contributing/checked-on-herdr-tabs.md](contributing/checked-on-herdr-tabs.md) § Terminal sizes, § Tabs and spaces, § The Coordinator and the keeper |
| Missing in herdr | [contributing/herdr-0.9.3-facts.md](contributing/herdr-0.9.3-facts.md) § Missing in Herdr, § Settled by this work |

## [`docs/colours.md`](../docs/colours.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro (Colours, summary table) | [reference/colours.md](reference/colours.md) § What each colour means<br>[explanation/colours.md](explanation/colours.md) § Colour is never the only clue |
| The board is in Herdr's theme | [reference/colours.md](reference/colours.md) § Where the theme comes from<br>[reference/colours.md](reference/colours.md) § The `board.theme` setting<br>[explanation/colours.md](explanation/colours.md) § The board follows Herdr's theme<br>[how-to/colours-choose-a-theme.md](how-to/colours-choose-a-theme.md) § Select a theme by hand |
| When the board cannot know the theme | [reference/colours.md](reference/colours.md) § When the board cannot know the theme<br>[explanation/colours.md](explanation/colours.md) § When the two cannot match<br>[how-to/colours-choose-a-theme.md](how-to/colours-choose-a-theme.md) § Make the board match Herdr with `herdr --remote` |
| A status is in Herdr's own colour | [reference/colours.md](reference/colours.md) § Status colours<br>[explanation/colours.md](explanation/colours.md) § A status uses Herdr's own colour |
| A queued task is pink | [reference/colours.md](reference/colours.md) § The queued pink<br>[explanation/colours.md](explanation/colours.md) § Why queued is pink |
| An automation is purple | [reference/colours.md](reference/colours.md) § The automation purple<br>[explanation/colours.md](explanation/colours.md) § Why automations are purple |
| The greys are quiet but always readable | [reference/colours.md](reference/colours.md) § The greys<br>[explanation/colours.md](explanation/colours.md) § Why the greys have a floor |
| In the `terminal` theme | [reference/colours.md](reference/colours.md) § The `terminal` theme<br>[explanation/colours.md](explanation/colours.md) § Why the `terminal` theme does not use the terminal's greys |
| On the web Board | [reference/colours.md](reference/colours.md) § The web board |
| The box says who you type to | [reference/colours-text-box.md](reference/colours-text-box.md) (whole page)<br>[explanation/colours.md](explanation/colours.md) § Why the box has the colour of the recipient<br>[explanation/colours.md](explanation/colours.md) § Why done is teal on the web board |

## [`docs/coordinator.md`](../docs/coordinator.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [explanation/coordinator.md](explanation/coordinator.md) § One long conversation<br>[how-to/talk-to-a-coordinator.md](how-to/talk-to-a-coordinator.md) § Send a message to the Coordinator from the board<br>[how-to/talk-to-a-coordinator.md](how-to/talk-to-a-coordinator.md) § Send a message from a shell<br>[how-to/talk-to-a-coordinator.md](how-to/talk-to-a-coordinator.md) § Restart, replace or stop the Coordinator |
| The Coordinator | [explanation/coordinator.md](explanation/coordinator.md) § The Coordinator (title) |
| Your message, and where you were | [explanation/coordinator.md](explanation/coordinator.md) § Your message, and where you were<br>[how-to/talk-to-a-coordinator.md](how-to/talk-to-a-coordinator.md) § Send a message from a shell |
| `Tab`: "and start it right away" | [explanation/coordinator.md](explanation/coordinator.md) § `Tab`: "and start it right away" |
| What it says it did | [explanation/coordinator.md](explanation/coordinator.md) § What it says it did |
| Going into it | [how-to/talk-to-a-coordinator.md](how-to/talk-to-a-coordinator.md) § Go into the Coordinator in Herdr<br>[explanation/coordinator.md](explanation/coordinator.md) § Why you go into it |
| What it is | [explanation/coordinator.md](explanation/coordinator.md) § What it is |
| Its standing rules are a file | [explanation/coordinator.md](explanation/coordinator.md) § Its standing rules are a file |
| Where it lives in Herdr | [explanation/coordinator.md](explanation/coordinator.md) § Where it lives in Herdr |
| Moving over from the old layout | [explanation/coordinator.md](explanation/coordinator.md) § Moving over from the old layout |

## [`docs/could-not-start.md`](../docs/could-not-start.md)

| Part of the old page | Where it is now |
| --- | --- |
| When an agent could not start | [explanation/could-not-start.md](explanation/could-not-start.md) § When an agent could not start |
| intro | [explanation/could-not-start.md](explanation/could-not-start.md) § The row tells the reason<br>[how-to/could-not-start-fix.md](how-to/could-not-start-fix.md) § one section per `Could not start: …` message |
| Where to look further | [how-to/could-not-start-fix.md](how-to/could-not-start-fix.md) § Look further |
| Long rules and long messages | [explanation/could-not-start.md](explanation/could-not-start.md) § Long rules and long messages (§ Long rules go in a file, § The board waits for a shell that is not at its prompt yet, § Very long messages go in a file) |

## [`docs/crates.md`](../docs/crates.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [contributing/crates.md](contributing/crates.md) § The five crates<br>[contributing/crates.md](contributing/crates.md) § Why the code has several crates |
| Where a new module goes | [contributing/crates.md](contributing/crates.md) § Where a new module goes |
| A crate uses only the crates below it | [contributing/crates.md](contributing/crates.md) § A crate uses only the crates below it (§ Code that was moved down, § Code that is handed in) |
| A new worktree | [contributing/crates.md](contributing/crates.md) § A new worktree |
| What it saves | [contributing/crates.md](contributing/crates.md) § What it saves |

## [`docs/fallback.md`](../docs/fallback.md)

| Part of the old page | Where it is now |
| --- | --- |
| The fallback agent: another agent takes over | [explanation/fallback.md](explanation/fallback.md) § The fallback agent |
| intro | [explanation/fallback.md](explanation/fallback.md) § Why there is a fallback agent<br>[reference/fallback-settings.md](reference/fallback-settings.md) § The two settings<br>[reference/fallback-settings.md](reference/fallback-settings.md) § The value<br>[reference/fallback-settings.md](reference/fallback-settings.md) § Examples<br>[how-to/set-a-fallback-agent.md](how-to/set-a-fallback-agent.md) § Set a fallback for the working agent<br>[how-to/set-a-fallback-agent.md](how-to/set-a-fallback-agent.md) § Set a fallback for one delegation |
| When a task is handed over | [explanation/fallback.md](explanation/fallback.md) § When a task is handed over<br>[explanation/fallback.md](explanation/fallback.md) § What is not handed over |
| What a hand-over is | [explanation/fallback.md](explanation/fallback.md) § What a hand-over is |
| Who is next | [explanation/fallback.md](explanation/fallback.md) § Who is next |
| Stepping in, and changing the list | [explanation/fallback.md](explanation/fallback.md) § When you step in, and when the list changes<br>[how-to/set-a-fallback-agent.md](how-to/set-a-fallback-agent.md) § Set a fallback on a task that is under way |
| The reviewer | [explanation/fallback.md](explanation/fallback.md) § The reviewer<br>[how-to/set-a-fallback-agent.md](how-to/set-a-fallback-agent.md) § Set a fallback for the reviewer |
| What you see | [reference/fallback-settings.md](reference/fallback-settings.md) § Where you see a hand-over<br>[reference/fallback-settings.md](reference/fallback-settings.md) § The words of the reason<br>[how-to/set-a-fallback-agent.md](how-to/set-a-fallback-agent.md) § See who took over |
| A setting like every other | [reference/fallback-settings.md](reference/fallback-settings.md) § Where the value comes from<br>[reference/fallback-settings.md](reference/fallback-settings.md) § The value<br>[explanation/fallback.md](explanation/fallback.md) § Why a repo file cannot set the fallback<br>[how-to/set-a-fallback-agent.md](how-to/set-a-fallback-agent.md) § Remove a fallback<br>[how-to/set-a-fallback-agent.md](how-to/set-a-fallback-agent.md) § See which list a task uses |
| Beside the other settings | [explanation/fallback.md](explanation/fallback.md) § The fallback and the other settings (§ Several attempts, § Limits on time and tokens) |
| For contributors | [contributing/fallback.md](contributing/fallback.md) § The hand-over |

## [`docs/gate.md`](../docs/gate.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [explanation/gate.md](explanation/gate.md) § The reviewer describes, the board decides |
| What the reviewer gives | [reference/gate-rules.md](reference/gate-rules.md) § What the reviewer gives; Severities |
| The board's rules | [reference/gate-rules.md](reference/gate-rules.md) § The rules of the board; Settings<br>[explanation/gate.md](explanation/gate.md) § Why the findings of the reviewer decide; Why a pre-existing finding never sends the work back; Why later rounds are narrower |
| Merging by itself | [explanation/gate.md](explanation/gate.md) § Merging by itself<br>[how-to/turn-on-auto-merge.md](how-to/turn-on-auto-merge.md) § Turn it on |
| What "green" means | [reference/gate-rules.md](reference/gate-rules.md) § What "green" means<br>[how-to/turn-on-auto-merge.md](how-to/turn-on-auto-merge.md) § Before you start |
| Only the reviewed commit merges | [explanation/gate.md](explanation/gate.md) § Only the reviewed commit merges |
| Main moved since the review | [explanation/gate.md](explanation/gate.md) § Main moved since the review |
| One at a time | [explanation/gate.md](explanation/gate.md) § One at a time |
| What still stops and asks you | [reference/gate-rules.md](reference/gate-rules.md) § What still stops and asks you<br>[explanation/gate.md](explanation/gate.md) § Stepping in, and the words of the board<br>[how-to/turn-on-auto-merge.md](how-to/turn-on-auto-merge.md) § Let a task go on after you stepped in |
| It counts from the moment you change it | [explanation/gate.md](explanation/gate.md) § It counts from the moment that you change it<br>[how-to/turn-on-auto-merge.md](how-to/turn-on-auto-merge.md) § Before you start; Turn it off |
| A repo cannot switch it on for itself | [explanation/gate.md](explanation/gate.md) § A repo cannot turn it on for itself<br>[how-to/turn-on-auto-merge.md](how-to/turn-on-auto-merge.md) § Turn it on for one repo |
| When the agent stops with an error or a question | [reference/gate-rules.md](reference/gate-rules.md) § When the agent stops with an error or a question |

## [`docs/get-started.md`](../docs/get-started.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [tutorials/first-board.md](tutorials/first-board.md) § Before you start<br>[how-to/install-and-update.md](how-to/install-and-update.md) § What you need |
| Get started | [tutorials/first-board.md](tutorials/first-board.md) § Your first board |
| Option A: do it yourself | [tutorials/first-board.md](tutorials/first-board.md) § Step 1: Install the plugin<br>[tutorials/first-board.md](tutorials/first-board.md) § Step 2: Open the board for the first time<br>[tutorials/first-board.md](tutorials/first-board.md) § Step 3: Put the command on your PATH |
| Option B: let an agent do it | [how-to/install-and-update.md](how-to/install-and-update.md) § Let an agent install it |
| Start it | [tutorials/first-board.md](tutorials/first-board.md) § Step 4: Open the board with its key<br>[tutorials/first-board.md](tutorials/first-board.md) § Step 5: Look at the board<br>[tutorials/first-board.md](tutorials/first-board.md) § What you learned<br>[tutorials/first-board.md](tutorials/first-board.md) § Next lesson |

## [`docs/handing-over.md`](../docs/handing-over.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [explanation/handing-over.md](explanation/handing-over.md) § What the board does |
| What you see | [explanation/handing-over.md](explanation/handing-over.md) § What you see |
| A task shown as not started | [explanation/handing-over.md](explanation/handing-over.md) § A task shown as not started; The reviewer and the Coordinator<br>[how-to/delegate-and-merge.md](how-to/delegate-and-merge.md) § If the task does not go on |
| Why this exists | [explanation/handing-over.md](explanation/handing-over.md) § Why this exists |
| How the board makes sure | [explanation/handing-over.md](explanation/handing-over.md) § How the board makes sure |
| A message typed while the agent is mid-turn | [explanation/handing-over.md](explanation/handing-over.md) § A message typed while the agent is in a turn; A message that the agent takes during a turn |
| The waits | [explanation/handing-over.md](explanation/handing-over.md) § The waits |

## [`docs/home-folder.md`](../docs/home-folder.md)

| Part of the old page | Where it is now |
| --- | --- |
| The home folder | [reference/home-folder.md](reference/home-folder.md) § The home folder |
| intro | [reference/home-folder.md](reference/home-folder.md) § Contents<br>[explanation/home-folder.md](explanation/home-folder.md) § One folder for everything |
| Putting it somewhere else | [reference/home-folder.md](reference/home-folder.md) § Variables that move the folder<br>[how-to/move-the-home-folder.md](how-to/move-the-home-folder.md) § Move the whole folder<br>[how-to/move-the-home-folder.md](how-to/move-the-home-folder.md) § Move only one part |
| Nothing of the board's goes into the folder an agent works in | [reference/home-folder.md](reference/home-folder.md) § Files that an agent writes for the board<br>[explanation/home-folder.md](explanation/home-folder.md) § Nothing of the board goes into the folder where an agent works |
| Leftovers from older versions | [reference/home-folder.md](reference/home-folder.md) § Leftovers from older versions<br>[how-to/move-from-an-old-install.md](how-to/move-from-an-old-install.md) § Remove old folders from your repos |
| What stays outside it | [reference/home-folder.md](reference/home-folder.md) § What stays outside the home folder<br>[explanation/home-folder.md](explanation/home-folder.md) § Why some files stay outside the folder<br>[how-to/remove.md](how-to/remove.md) § What stays outside the home folder |
| Coming from herdr-superwork or herdr-projects 0.2 | [reference/home-folder.md](reference/home-folder.md) § Old paths after a move<br>[reference/old-installs-takeover.md](reference/old-installs-takeover.md) § herdr-superwork: moved into `~/.herdr-projects`, carried on |
| Coming from an older version: the one-time move | [how-to/move-from-an-old-install.md](how-to/move-from-an-old-install.md) § Move from a version with files in `~/.local/state`, `~/.config` and `~/.local/share`<br>[explanation/home-folder.md](explanation/home-folder.md) § How the one-time move keeps your data safe |
| Why it is safe | [explanation/home-folder.md](explanation/home-folder.md) § How the one-time move keeps your data safe (and its six sub-sections) |
| The Coordinator's conversation | [explanation/home-folder.md](explanation/home-folder.md) § The conversation of the Coordinator after a move |
| Undoing it | [how-to/move-from-an-old-install.md](how-to/move-from-an-old-install.md) § Undo the move from `~/.local/state`, `~/.config` and `~/.local/share` |

## [`docs/how-work-gets-done.md`](../docs/how-work-gets-done.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [explanation/how-work-gets-done.md](explanation/how-work-gets-done.md) § The path of a task |
| Delegating | [explanation/how-work-gets-done.md](explanation/how-work-gets-done.md) § Delegating (In a git repo; The name of the branch; A task started from a named branch; Outside git; How many agents work at once)<br>[how-to/delegate-and-merge.md](how-to/delegate-and-merge.md) § Delegate a task; Start from another place; Choose the name of the branch |
| Agents run with full access by default | [explanation/how-work-gets-done.md](explanation/how-work-gets-done.md) § Agents run with full access by default<br>[how-to/delegate-and-merge.md](how-to/delegate-and-merge.md) § Make the agent ask before it acts |
| How the board knows a turn ended | [explanation/how-work-gets-done.md](explanation/how-work-gets-done.md) § How the board knows that a turn ended |
| Without a hook | [explanation/how-work-gets-done.md](explanation/how-work-gets-done.md) § Without a hook<br>[how-to/delegate-and-merge.md](how-to/delegate-and-merge.md) § If the task does not go on |
| The finish line | [explanation/how-work-gets-done.md](explanation/how-work-gets-done.md) § The finish line; The pull request; The review; Helpers of an agent<br>[how-to/delegate-and-merge.md](how-to/delegate-and-merge.md) § Read the review<br>[tutorials/first-task.md](tutorials/first-task.md) § Step 5: Read the review |
| Approving and merging | [explanation/how-work-gets-done.md](explanation/how-work-gets-done.md) § Approving and merging<br>[how-to/delegate-and-merge.md](how-to/delegate-and-merge.md) § Approve and merge<br>[tutorials/first-task.md](tutorials/first-task.md) § Step 6: Approve and merge |
| GitHub's checks must pass first | [explanation/how-work-gets-done-github-checks.md](explanation/how-work-gets-done-github-checks.md) § GitHub's checks must pass first; The board decides from what the repo expects |
| A fine-grained GitHub token is enough | [explanation/how-work-gets-done-github-checks.md](explanation/how-work-gets-done-github-checks.md) § A fine-grained GitHub token is enough |
| When gh cannot read GitHub at all | [explanation/how-work-gets-done-github-checks.md](explanation/how-work-gets-done-github-checks.md) § When gh cannot read GitHub at all; The environment of the keeper |
| Staying current with main | [explanation/how-work-gets-done-main.md](explanation/how-work-gets-done-main.md) § Why the board does it; When the board does it; A clean update; A real conflict; Only a task at rest; What "main" is |
| A review that passed is not repeated for a merge of main | [explanation/how-work-gets-done-main.md](explanation/how-work-gets-done-main.md) § A review that passed is not repeated for a merge of main; The rule for "only the merge of main"; A repo with no check; What the task says |
| Stepping in | [explanation/how-work-gets-done.md](explanation/how-work-gets-done.md) § Stepping in |
| Chatting with a finished task re-reviews only what changed | [explanation/how-work-gets-done-chat.md](explanation/how-work-gets-done-chat.md) § Which chats this is about; No code changed; A review was under way when you wrote; The code changed; What does not change |
| When a task is done | [explanation/how-work-gets-done-clean-up.md](explanation/how-work-gets-done-clean-up.md) § The clean-up; Work is never lost; What counts as a running command; An agent of unknown status; The space of the repo<br>[how-to/delegate-and-merge.md](how-to/delegate-and-merge.md) § After the merge<br>[tutorials/first-task.md](tutorials/first-task.md) § Step 7: See the clean-up |
| With the board closed: the keeper | [explanation/how-work-gets-done-clean-up.md](explanation/how-work-gets-done-clean-up.md) § With the board closed: the keeper; The sidebar; No notifications; The life of the keeper |

## [`docs/install.md`](../docs/install.md)

| Part of the old page | Where it is now |
| --- | --- |
| Installing, the first run, and taking it out again | [how-to/install-and-update.md](how-to/install-and-update.md) § Install and update herdr-projects |
| intro | [how-to/install-and-update.md](how-to/install-and-update.md) § Install and update herdr-projects<br>[how-to/install-and-update.md](how-to/install-and-update.md) § Install with Herdr |
| What you need | [how-to/install-and-update.md](how-to/install-and-update.md) § What you need<br>[tutorials/first-board.md](tutorials/first-board.md) § Before you start |
| What the install does | [how-to/install-and-update.md](how-to/install-and-update.md) § Install with Herdr<br>[how-to/install-and-update.md](how-to/install-and-update.md) § Install from a checkout of your own<br>[reference/install-details.md](reference/install-details.md) § Commands |
| The first run | [how-to/install-and-update.md](how-to/install-and-update.md) § Do the first-run setup<br>[reference/install-details.md](reference/install-details.md) § The first-run screen<br>[reference/install-details.md](reference/install-details.md) § Item 1: a key that opens the board<br>[reference/install-details.md](reference/install-details.md) § Item 2: sidebar rows<br>[tutorials/first-board.md](tutorials/first-board.md) § Step 2: Open the board for the first time |
| Adding the blocks by hand | [how-to/install-and-update.md](how-to/install-and-update.md) § Add the config blocks by hand |
| The command on your PATH | [how-to/install-and-update.md](how-to/install-and-update.md) § Put the command on your PATH<br>[reference/install-details.md](reference/install-details.md) § The launcher<br>[how-to/set-up-another-machine.md](how-to/set-up-another-machine.md) § Put `herdr-projects` on the PATH of a login shell |
| Updating | [how-to/install-and-update.md](how-to/install-and-update.md) § Update<br>[reference/install-details.md](reference/install-details.md) § How an update is loaded<br>[reference/install-details.md](reference/install-details.md) § How the keeper updates itself |
| Building from source | [how-to/install-and-update.md](how-to/install-and-update.md) § Build from source |
| Coming from an older version | [how-to/move-from-an-old-install.md](how-to/move-from-an-old-install.md) § Find what you must do<br>[how-to/move-from-an-old-install.md](how-to/move-from-an-old-install.md) § Move from herdr-superwork or herdr-projects 0.2<br>[how-to/move-from-an-old-install.md](how-to/move-from-an-old-install.md) § Move from the older Node version<br>[how-to/move-from-an-old-install.md](how-to/move-from-an-old-install.md) § Move from a version with files in `~/.local/state`, `~/.config` and `~/.local/share` |
| Taking it out again | [how-to/remove.md](how-to/remove.md) § Why you must remove the extras first<br>[how-to/remove.md](how-to/remove.md) § Remove the plugin |
| Details: which program runs | [reference/install-details.md](reference/install-details.md) § Program files<br>[reference/install-details.md](reference/install-details.md) § How `bin/run.sh` picks the program<br>[reference/install-details.md](reference/install-details.md) § Environment variable<br>[reference/install-details.md](reference/install-details.md) § Commands |

## [`docs/keys.md`](../docs/keys.md)

| Part of the old page | Where it is now |
| --- | --- |
| Keys, questions and the ? menu | [reference/keys.md](reference/keys.md) § Keys |
| intro | [reference/keys.md](reference/keys.md) § The rule for the keys |
| Keys and the text box | [reference/keys.md](reference/keys.md) § Keys and the text box |
| The `?` menu | [reference/keys.md](reference/keys.md) § The `?` menu |
| Questions: the board asks before what cannot be undone | [reference/keys.md](reference/keys.md) § Questions |
| Work whose review failed is never merged by a plain yes | [reference/keys.md](reference/keys.md) § Work with a failed review |
| Closing the board | [reference/keys.md](reference/keys.md) § Close the board |
| The key that opens the board | [reference/keys.md](reference/keys.md) § The key that opens the board |
| The settings page owns its keys | [reference/keys.md](reference/keys.md) § Keys on the settings page |
| The hint line | [reference/keys-hint-line.md](reference/keys-hint-line.md) § What the hint line shows<br>[reference/keys-hint-line.md](reference/keys-hint-line.md) § Examples<br>[reference/keys-hint-line.md](reference/keys-hint-line.md) § In a narrow window<br>[reference/keys.md](reference/keys.md) § The hint line (Coordinator restart pointer) |
| Your own keys | [how-to/add-your-own-keys.md](how-to/add-your-own-keys.md) § Give a key your own chord<br>[how-to/add-your-own-keys.md](how-to/add-your-own-keys.md) § If the board refuses the chord<br>[how-to/add-your-own-keys.md](how-to/add-your-own-keys.md) § Rules for your own keys<br>[how-to/add-your-own-keys.md](how-to/add-your-own-keys.md) § Put back every key<br>[reference/keys.md](reference/keys.md) § Your own keys |
| All the keys | [reference/keys.md](reference/keys.md) § All the keys (snapshot) |

## [`docs/limits.md`](../docs/limits.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [reference/limits.md](reference/limits.md) § The two settings<br>[how-to/set-limits.md](how-to/set-limits.md) § Set a limit<br>[how-to/set-limits.md](how-to/set-limits.md) § Remove a limit<br>[explanation/limits.md](explanation/limits.md) § A cap, not a loss |
| Limits: time and tokens | [reference/limits.md](reference/limits.md) § Limits (title)<br>[explanation/limits.md](explanation/limits.md) § How limits work (title) |
| What happens at a limit | [reference/limits.md](reference/limits.md) § What occurs at a limit<br>[reference/limits.md](reference/limits.md) § Where the tokens run out<br>[explanation/limits.md](explanation/limits.md) § Where the tokens run out |
| Carrying on | [reference/limits.md](reference/limits.md) § Carrying on<br>[how-to/set-limits.md](how-to/set-limits.md) § Continue a task that stopped at a limit<br>[how-to/set-limits.md](how-to/set-limits.md) § Stop a task at a limit yourself<br>[explanation/limits.md](explanation/limits.md) § Continue, a higher limit, or as it is |
| What the board shows | [reference/limits.md](reference/limits.md) § What the board shows<br>[how-to/set-limits.md](how-to/set-limits.md) § Change limits on the web board |
| What is measured | [reference/limits.md](reference/limits.md) § What the board counts<br>[explanation/limits.md](explanation/limits.md) § What a run is<br>[explanation/limits.md](explanation/limits.md) § What the tokens are |
| A run | [reference/limits.md](reference/limits.md) § What the board counts<br>[explanation/limits.md](explanation/limits.md) § What a run is |
| Tokens | [reference/limits.md](reference/limits.md) § What the board counts<br>[explanation/limits.md](explanation/limits.md) § What the tokens are<br>[explanation/limits.md](explanation/limits.md) § Why one turn can pass the limit<br>[explanation/limits.md](explanation/limits.md) § Why a late limit still counts all<br>[how-to/set-limits.md](how-to/set-limits.md) § Set a limit (note) |
| When each is read | [reference/limits.md](reference/limits.md) § When the board reads them |
| Beside the other settings | [reference/limits.md](reference/limits.md) § Limits with other settings<br>[explanation/limits.md](explanation/limits.md) § Limits beside the other settings<br>[explanation/limits.md](explanation/limits.md) § A cap, not a loss (research of task herdr-projects/154) |
| Where it is | [contributing/limits-code.md](contributing/limits-code.md) § The code<br>[contributing/limits-code.md](contributing/limits-code.md) § The tests |

## [`docs/linking.md`](../docs/linking.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [contributing/linking.md](contributing/linking.md) § What the builds use |
| How it works | [contributing/linking.md](contributing/linking.md) § How it works |
| A test guards every platform | [contributing/linking.md](contributing/linking.md) § A test guards every platform |
| Why flags, and not a linker named in the config | [contributing/linking.md](contributing/linking.md) § Why flags, and not a linker named in the config |
| What it saves | [contributing/linking.md](contributing/linking.md) § What it saves |

## [`docs/list-headings.md`](../docs/list-headings.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro (List headings, example) | [reference/list-headings.md](reference/list-headings.md) § intro<br>[how-to/fold-list-headings.md](how-to/fold-list-headings.md) § Group the list |
| Machine and repo headings | [reference/list-headings.md](reference/list-headings.md) § Machine headings<br>[reference/list-headings.md](reference/list-headings.md) § Repo headings |
| Folding | [reference/list-headings.md](reference/list-headings.md) § Folding<br>[how-to/fold-list-headings.md](how-to/fold-list-headings.md) § Fold a heading<br>[how-to/fold-list-headings.md](how-to/fold-list-headings.md) § Unfold a heading |
| How a heading looks | [reference/list-headings.md](reference/list-headings.md) § How a heading looks |
| Folds are remembered | [reference/list-headings.md](reference/list-headings.md) § The board keeps the folds |
| What a fold never hides | [reference/list-headings.md](reference/list-headings.md) § What a fold never hides<br>[how-to/fold-list-headings.md](how-to/fold-list-headings.md) § See tasks under a folded heading |
| The web board | [reference/list-headings.md](reference/list-headings.md) § The web board<br>[how-to/fold-list-headings.md](how-to/fold-list-headings.md) § Fold a heading on the web board |

## [`docs/long-running-work.md`](../docs/long-running-work.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [how-to/run-long-work-in-one-repo.md](how-to/run-long-work-in-one-repo.md) § Run long work in one repo (intro and table) |
| Long-running work in one repo | [how-to/run-long-work-in-one-repo.md](how-to/run-long-work-in-one-repo.md) § Run long work in one repo (title) |
| A project coordinator per repo | [how-to/run-long-work-in-one-repo.md](how-to/run-long-work-in-one-repo.md) § Plan the work with the project coordinator |
| A tracking doc or pull request | [how-to/run-long-work-in-one-repo.md](how-to/run-long-work-in-one-repo.md) § Keep the whole picture in one place |
| Batches as tasks, chained with `--after` | [how-to/run-long-work-in-one-repo.md](how-to/run-long-work-in-one-repo.md) § Add the batches as a chain of tasks |
| The next batch: a fresh agent, or the same one? | [how-to/run-long-work-in-one-repo.md](how-to/run-long-work-in-one-repo.md) § Choose a fresh agent or the same agent for the next batch |

## [`docs/machine-and-project-coordinators.md`](../docs/machine-and-project-coordinators.md)

| Part of the old page | Where it is now |
| --- | --- |
| Machine and project coordinators | [explanation/machine-and-project-coordinators.md](explanation/machine-and-project-coordinators.md) § Machine and project coordinators (title) |
| intro | [explanation/machine-and-project-coordinators.md](explanation/machine-and-project-coordinators.md) § The three levels |
| Talking to one | [how-to/talk-to-a-coordinator.md](how-to/talk-to-a-coordinator.md) § Send a message to a machine coordinator<br>[how-to/talk-to-a-coordinator.md](how-to/talk-to-a-coordinator.md) § Send a message to a project coordinator<br>[explanation/machine-and-project-coordinators.md](explanation/machine-and-project-coordinators.md) § How the board chooses one |
| What each one is | [explanation/machine-and-project-coordinators.md](explanation/machine-and-project-coordinators.md) § What each one is |
| From a shell | [how-to/talk-to-a-coordinator.md](how-to/talk-to-a-coordinator.md) § Send a message from a shell |

## [`docs/machine-order.md`](../docs/machine-order.md)

| Part of the old page | Where it is now |
| --- | --- |
| Machine order | [explanation/machine-order.md](explanation/machine-order.md) § Machine order |
| intro | [explanation/machine-order.md](explanation/machine-order.md) § The board follows the sidebar |
| Where Herdr keeps the order | [explanation/machine-order.md](explanation/machine-order.md) § Where Herdr keeps the order |
| Which sidebar the board follows | [explanation/machine-order.md](explanation/machine-order.md) § Which sidebar the board follows |
| How the board finds itself in another machine's list | [explanation/machine-order.md](explanation/machine-order.md) § How the board finds this machine in the list of another machine<br>[explanation/machine-order.md](explanation/machine-order.md) § How the board matches the other machines |
| Machines the sidebar does not show | [explanation/machine-order.md](explanation/machine-order.md) § Machines that the sidebar does not show |
| A machine on an older program | [explanation/machine-order.md](explanation/machine-order.md) § A machine on an older program |

## [`docs/models.md`](../docs/models.md)

| Part of the old page | Where it is now |
| --- | --- |
| Models and effort | [reference/models.md](reference/models.md) § Models and effort |
| intro | [reference/models.md](reference/models.md) § Models and effort (intro)<br>[reference/models.md](reference/models.md) § The models command<br>[how-to/choose-a-model-and-effort.md](how-to/choose-a-model-and-effort.md) § Find the names |
| Examples | [how-to/choose-a-model-and-effort.md](how-to/choose-a-model-and-effort.md) § Set a model and an effort<br>[how-to/choose-a-model-and-effort.md](how-to/choose-a-model-and-effort.md) § Ask the Coordinator |
| A model | [reference/models.md](reference/models.md) § A model |
| A wrong name is refused when you set it | [reference/models.md](reference/models.md) § The board refuses a wrong name when you set it<br>[how-to/choose-a-model-and-effort.md](how-to/choose-a-model-and-effort.md) § If the board cannot read the list of models |
| A model refused while the agent works | [reference/models.md](reference/models.md) § A model that the program refuses while the agent works<br>[reference/models.md](reference/models.md) § When the board does not check a name<br>[how-to/choose-a-model-and-effort.md](how-to/choose-a-model-and-effort.md) § If the program refuses the model while the agent works |
| An effort | [reference/models.md](reference/models.md) § An effort |
| A new program | [contributing/models-new-program.md](contributing/models-new-program.md) § The models of a program<br>[contributing/models-new-program.md](contributing/models-new-program.md) § The effort levels of a program |

## [`docs/mouse.md`](../docs/mouse.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro (The mouse) | [reference/mouse.md](reference/mouse.md) § intro |
| The wheel | [reference/mouse.md](reference/mouse.md) § The wheel<br>[explanation/mouse.md](explanation/mouse.md) § Why the wheel does not move the selection while you type |
| Clicks | [reference/mouse.md](reference/mouse.md) § Clicks |
| Selecting and copying | [reference/mouse.md](reference/mouse.md) § Selecting and copying<br>[reference/mouse.md](reference/mouse.md) § For contributors<br>[explanation/mouse.md](explanation/mouse.md) § Why the board draws its own selection |
| Why the board takes the whole mouse | [explanation/mouse.md](explanation/mouse.md) § A terminal cannot give the wheel alone |
| In the popup | [reference/mouse.md](reference/mouse.md) § In the popup<br>[explanation/mouse.md](explanation/mouse.md) § How it was tested |

## [`docs/old-installs.md`](../docs/old-installs.md)

| Part of the old page | Where it is now |
| --- | --- |
| Older installs: herdr-superwork and herdr-projects 0.2 | [how-to/move-from-an-old-install.md](how-to/move-from-an-old-install.md) § Move from an old install<br>[reference/old-installs-takeover.md](reference/old-installs-takeover.md) § What the takeover of an old install does |
| intro | [how-to/move-from-an-old-install.md](how-to/move-from-an-old-install.md) § Find what you must do<br>[how-to/move-from-an-old-install.md](how-to/move-from-an-old-install.md) § Move from herdr-superwork or herdr-projects 0.2<br>[reference/old-installs-takeover.md](reference/old-installs-takeover.md) § What the takeover of an old install does |
| What they had | [reference/old-installs-takeover.md](reference/old-installs-takeover.md) § What the old programs had |
| herdr-projects 0.2: set aside, never mixed, never deleted | [reference/old-installs-takeover.md](reference/old-installs-takeover.md) § herdr-projects 0.2: set aside, never mixed, never deleted<br>[how-to/move-from-an-old-install.md](how-to/move-from-an-old-install.md) § Use herdr-projects 0.2 again |
| herdr-superwork: moved into ~/.herdr-projects, carried on | [reference/old-installs-takeover.md](reference/old-installs-takeover.md) § herdr-superwork: moved into `~/.herdr-projects`, carried on<br>[how-to/move-from-an-old-install.md](how-to/move-from-an-old-install.md) § Remove the old links |
| Only the copy the board runs from | [reference/old-installs-takeover.md](reference/old-installs-takeover.md) § Which copy of the program does the takeover<br>[explanation/home-folder.md](explanation/home-folder.md) § Only the copy that the board runs from moves anything |
| Undoing it | [how-to/move-from-an-old-install.md](how-to/move-from-an-old-install.md) § Undo the takeover of herdr-superwork or herdr-projects 0.2 |

## [`docs/other-machines.md`](../docs/other-machines.md)

| Part of the old page | Where it is now |
| --- | --- |
| Other machines | [explanation/other-machines.md](explanation/other-machines.md) § Other machines and several boards |
| intro | [explanation/other-machines.md](explanation/other-machines.md) § Each machine keeps its own tasks |
| Set up another machine | [how-to/set-up-another-machine.md](how-to/set-up-another-machine.md) § Before you start<br>[how-to/set-up-another-machine.md](how-to/set-up-another-machine.md) § Install herdr-projects on the other machine<br>[how-to/set-up-another-machine.md](how-to/set-up-another-machine.md) § Add the machine to Herdr<br>[explanation/other-machines.md](explanation/other-machines.md) § Why SSH |
| `herdr-projects` must be on a login shell's PATH | [how-to/set-up-another-machine.md](how-to/set-up-another-machine.md) § Put `herdr-projects` on the PATH of a login shell |
| Check every machine: `herdr-projects machines` | [how-to/set-up-another-machine.md](how-to/set-up-another-machine.md) § Check every machine |
| When a machine does not answer | [explanation/other-machines.md](explanation/other-machines.md) § When a machine does not answer<br>[how-to/set-up-another-machine.md](how-to/set-up-another-machine.md) § Repair a machine that does not answer |
| What you see of another machine | [explanation/other-machines.md](explanation/other-machines.md) § What you see of another machine |
| Going into another machine's agent | [explanation/other-machines.md](explanation/other-machines.md) § Going into the agent of another machine |
| A task's work settings travel with it | [explanation/other-machines.md](explanation/other-machines.md) § The work settings of a task go with it |
| In the Herdr web UI | [explanation/other-machines.md](explanation/other-machines.md) § In the Herdr web UI |

## [`docs/plan.md`](../docs/plan.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [explanation/plan.md](explanation/plan.md) § Why a plan first<br>[how-to/get-a-plan-first.md](how-to/get-a-plan-first.md) § Ask for a plan on one task<br>[how-to/get-a-plan-first.md](how-to/get-a-plan-first.md) § Let the reviewer check every plan in a repo |
| A plan first | [explanation/plan.md](explanation/plan.md) § A plan first (title) |
| The two settings | [reference/plan-settings.md](reference/plan-settings.md) § The two settings<br>[reference/plan-settings.md](reference/plan-settings.md) § When the board reads them<br>[explanation/plan.md](explanation/plan.md) § Why a plan first |
| Plan ready: say yes, or send it back | [reference/plan-settings.md](reference/plan-settings.md) § The plan ready state<br>[reference/plan-settings.md](reference/plan-settings.md) § What you can do on a plan ready task<br>[how-to/get-a-plan-first.md](how-to/get-a-plan-first.md) § Say yes to a plan<br>[how-to/get-a-plan-first.md](how-to/get-a-plan-first.md) § Send a plan back with your words |
| The reviewer's look | [reference/plan-settings.md](reference/plan-settings.md) § The reviewer's look at a plan<br>[explanation/plan.md](explanation/plan.md) § The reviewer's look |
| After the yes | [reference/plan-settings.md](reference/plan-settings.md) § After the yes<br>[explanation/plan.md](explanation/plan.md) § What the plan holds<br>[explanation/plan.md](explanation/plan.md) § The review after the build |
| Who writes the plan | [reference/plan-settings.md](reference/plan-settings.md) § Who writes the plan |
| The working agent (`plan.agent: none`) | [reference/plan-settings.md](reference/plan-settings.md) § Who writes the plan<br>[explanation/plan.md](explanation/plan.md) § What the plan holds |
| A planner of its own (`plan.agent: <program>`) | [reference/plan-settings.md](reference/plan-settings.md) § Who writes the plan<br>[reference/plan-settings.md](reference/plan-settings.md) § After the yes |
| The plan file | [reference/plan-settings.md](reference/plan-settings.md) § The plan file |
| Nothing in the repo is changed before the yes | [explanation/plan.md](explanation/plan.md) § Nothing in the repo changes before the yes<br>[explanation/plan.md](explanation/plan.md) § What the mount does not do<br>[reference/plan-settings.md](reference/plan-settings.md) § Messages<br>[how-to/get-a-plan-first.md](how-to/get-a-plan-first.md) § Before you start<br>[how-to/get-a-plan-first.md](how-to/get-a-plan-first.md) § If the plan step cannot start<br>[how-to/get-a-plan-first.md](how-to/get-a-plan-first.md) § If the board says the repo changed before the yes |
| The mount on Linux and on macOS | [reference/plan-settings.md](reference/plan-settings.md) § The mount on Linux and on macOS<br>[reference/plan-settings.md](reference/plan-settings.md) § The program's own sandbox on macOS<br>[reference/plan-settings.md](reference/plan-settings.md) § `HERDR_PROJECTS_SANDBOX`<br>[explanation/plan.md](explanation/plan.md) § The same guard on Linux and on macOS |
| A task in your own checkout | [explanation/plan.md](explanation/plan.md) § A task in your own checkout |
| Several attempts on one task | [explanation/plan.md](explanation/plan.md) § A plan and several attempts<br>[reference/plan-settings.md](reference/plan-settings.md) § When the board reads them |

## [`docs/provider-errors.md`](../docs/provider-errors.md)

| Part of the old page | Where it is now |
| --- | --- |
| A turn cut off by the provider | [explanation/provider-errors.md](explanation/provider-errors.md) § A turn cut off by the provider |
| intro | [explanation/provider-errors.md](explanation/provider-errors.md) § What the board does |
| How the board knows | [explanation/provider-errors.md](explanation/provider-errors.md) § How the board knows |
| The reviewer | [explanation/provider-errors.md](explanation/provider-errors.md) § The reviewer |
| The working agent | [explanation/provider-errors.md](explanation/provider-errors.md) § The working agent |
| Errors that waiting cannot fix | [explanation/provider-errors.md](explanation/provider-errors.md) § Errors that waiting cannot fix |
| With a fallback agent | [explanation/provider-errors.md](explanation/provider-errors.md) § With a fallback agent |
| Details | [explanation/provider-errors.md](explanation/provider-errors.md) § What the board does (the waits are fixed)<br>[contributing/fallback.md](contributing/fallback.md) § The retries after a provider error |

## [`docs/research.md`](../docs/research.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [reference/research-tasks.md](reference/research-tasks.md) § Reports; Research-only tasks<br>[how-to/make-a-research-task.md](how-to/make-a-research-task.md) § Add a new research task |
| Where a report goes | [reference/research-tasks.md](reference/research-tasks.md) § Where a report goes; Where you find the link<br>[how-to/make-a-research-task.md](how-to/make-a-research-task.md) § Read the report |
| Making a task research only | [reference/research-tasks.md](reference/research-tasks.md) § The setting `research.only`; Ways to set it<br>[how-to/make-a-research-task.md](how-to/make-a-research-task.md) § Add a new research task; Make a task research only later; Make every new task in a repo research only; Make an automation make research tasks; See where the value comes from |
| On the board | [reference/research-tasks.md](reference/research-tasks.md) § On the board |
| What its agent is told | [reference/research-tasks.md](reference/research-tasks.md) § What its agent is told |
| What the board does when it finishes | [reference/research-tasks.md](reference/research-tasks.md) § What the board does at the finish; Nothing changed (the usual case); Something changed<br>[how-to/make-a-research-task.md](how-to/make-a-research-task.md) § If the agent changed the repo |
| Changing it on a task that is under way | [reference/research-tasks.md](reference/research-tasks.md) § Changing it on a task that is under way<br>[how-to/make-a-research-task.md](how-to/make-a-research-task.md) § Make a task research only later |
| A task that works in your own checkout | [reference/research-tasks.md](reference/research-tasks.md) § A task that works in your own checkout |

## [`docs/roles-and-commands.md`](../docs/roles-and-commands.md)

| Part of the old page | Where it is now |
| --- | --- |
| Roles and commands | [reference/roles.md](reference/roles.md) § Roles<br>[reference/commands.md](reference/commands.md) § Commands |
| intro | (no intro text in the source) |
| Four roles | [reference/roles.md](reference/roles.md) § The four roles<br>[explanation/roles.md](explanation/roles.md) § Four roles, four jobs |
| The Watcher's guess | [reference/roles.md](reference/roles.md) § The Watcher<br>[explanation/roles.md](explanation/roles.md) § Why the guess of the Watcher can go down |
| The Coordinator | [reference/roles.md](reference/roles.md) § The Coordinator |
| Every chat knows the board | [reference/roles.md](reference/roles.md) § The how-to in every chat<br>[explanation/roles.md](explanation/roles.md) § Why every chat knows the board |
| Commands | [reference/commands.md](reference/commands.md) § The commands |
| What a command answers | [reference/commands.md](reference/commands.md) § What a command answers |
| A flag a command does not know is refused | [reference/commands.md](reference/commands.md) § Flags |
| Agents the board started: able, but not nudged | [reference/roles.md](reference/roles.md) § Agents that the board started<br>[explanation/roles.md](explanation/roles.md) § Why agents can act on the board, but do not |

## [`docs/settings.md`](../docs/settings.md)

| Part of the old page | Where it is now |
| --- | --- |
| Settings | [reference/settings.md](reference/settings.md) § Settings |
| intro | [reference/settings.md](reference/settings.md) § Settings (intro)<br>[how-to/change-a-setting.md](how-to/change-a-setting.md) § Change a setting with a command |
| Two kinds of setting | [reference/settings.md](reference/settings.md) § Two kinds of setting |
| The rule | [reference/settings.md](reference/settings.md) § The order of the places<br>[explanation/settings.md](explanation/settings.md) § The nearest answer wins<br>[explanation/settings.md](explanation/settings.md) § When the board reads a setting |
| The places | [reference/settings.md](reference/settings.md) § The places<br>[explanation/settings.md](explanation/settings.md) § Why the repo's own file is limited<br>[explanation/settings.md](explanation/settings.md) § A mistake in a file |
| Values | [reference/settings.md](reference/settings.md) § Values<br>[explanation/settings.md](explanation/settings.md) § Instructions add up, other settings replace |
| Commands | [reference/settings.md](reference/settings.md) § Commands<br>[how-to/change-a-setting.md](how-to/change-a-setting.md) § Change a setting with a command<br>[how-to/change-a-setting.md](how-to/change-a-setting.md) § Set a setting when you delegate a task<br>[how-to/change-a-setting.md](how-to/change-a-setting.md) § Put a setting back<br>[how-to/change-a-setting.md](how-to/change-a-setting.md) § Change a setting of a task on the board |
| The list | [reference/settings.md](reference/settings.md) § The list (snapshot) |
| Other machines | [reference/settings.md](reference/settings.md) § Other machines<br>[explanation/settings.md](explanation/settings.md) § Settings on other machines |
| What is fixed | [reference/settings.md](reference/settings.md) § What is fixed |
| Coming from the old names | [reference/settings.md](reference/settings.md) § Settings from the old names |

## [`docs/several-boards.md`](../docs/several-boards.md)

| Part of the old page | Where it is now |
| --- | --- |
| Several boards: repos, machines, sessions, Coordinators | [explanation/other-machines.md](explanation/other-machines.md) § Several boards |
| intro | [explanation/other-machines.md](explanation/other-machines.md) § Several boards<br>[how-to/move-the-home-folder.md](how-to/move-the-home-folder.md) § Move the home folder (note) |
| Sending new tasks to another repo | [how-to/send-tasks-to-another-repo.md](how-to/send-tasks-to-another-repo.md) § Open the popup from a pane of that repo<br>[how-to/send-tasks-to-another-repo.md](how-to/send-tasks-to-another-repo.md) § Tell the Coordinator which repo<br>[how-to/send-tasks-to-another-repo.md](how-to/send-tasks-to-another-repo.md) § Talk to the coordinator of the repo |

## [`docs/shorter-checks.md`](../docs/shorter-checks.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [explanation/shorter-checks.md](explanation/shorter-checks.md) § Why a shorter check<br>[how-to/shorter-checks-set-up.md](how-to/shorter-checks-set-up.md) § Set the shorter check |
| When the shorter check runs | [explanation/shorter-checks.md](explanation/shorter-checks.md) § When the shorter check runs<br>[how-to/shorter-checks-set-up.md](how-to/shorter-checks-set-up.md) § Before you start |
| What the shorter check is told | [explanation/shorter-checks.md](explanation/shorter-checks.md) § What the shorter check gets |
| While it runs, and after | [explanation/shorter-checks.md](explanation/shorter-checks.md) § While it runs, and after |
| This repo's shorter check | [explanation/shorter-checks.md](explanation/shorter-checks.md) § The shorter check of this repo |

## [`docs/sidebar.md`](../docs/sidebar.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro (Herdr's sidebar) | [reference/sidebar.md](reference/sidebar.md) § What the sidebar shows<br>[explanation/sidebar.md](explanation/sidebar.md) § Why the sidebar copies the row |
| The three rows of an entry | [reference/sidebar.md](reference/sidebar.md) § The three rows of an entry |
| Row 1: Herdr's dot, the number and the title | [reference/sidebar.md](reference/sidebar.md) § Row 1: Herdr's dot, the number and the title |
| Row 2: who is on it and where it stands | [reference/sidebar.md](reference/sidebar.md) § Row 2: who is on it and its state<br>[reference/sidebar.md](reference/sidebar.md) § For contributors |
| Row 3: the numbers, bare | [reference/sidebar.md](reference/sidebar.md) § Row 3: the numbers alone |
| The reviewer, the Coordinator and the spaces | [reference/sidebar.md](reference/sidebar.md) § The reviewer, the Coordinator and the spaces |
| With and without the plugin's sidebar rows | [reference/sidebar.md](reference/sidebar.md) § With and without the sidebar rows of the plugin |
| What is Herdr's own | [reference/sidebar.md](reference/sidebar.md) § What is Herdr's own<br>[explanation/sidebar.md](explanation/sidebar.md) § Why the dot is Herdr's |
| What Herdr 0.9.3 cannot do here | [reference/sidebar.md](reference/sidebar.md) § Limits with Herdr 0.9.3<br>[explanation/sidebar.md](explanation/sidebar.md) § Limits with Herdr 0.9.3 |
| How it gets there | [explanation/sidebar.md](explanation/sidebar.md) § How the board writes the entries |

## [`docs/spec-tools.md`](../docs/spec-tools.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [how-to/use-spec-tools.md](how-to/use-spec-tools.md) § Use a spec tool (OpenSpec) (intro) |
| Spec tools (OpenSpec) | [how-to/use-spec-tools.md](how-to/use-spec-tools.md) § Use a spec tool (OpenSpec) (title) |
| The settings | [how-to/use-spec-tools.md](how-to/use-spec-tools.md) § Set the order of work and the check |
| Approving the proposal | [how-to/use-spec-tools.md](how-to/use-spec-tools.md) § Approve the proposal before the build<br>[how-to/use-spec-tools.md](how-to/use-spec-tools.md) § Get the proposal by itself first |

## [`docs/tabs.md`](../docs/tabs.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro (A task's tabs in Herdr, table) | [reference/tabs.md](reference/tabs.md) § The tabs |
| When tabs open and close | [reference/tabs.md](reference/tabs.md) § When tabs open and close |
| Nothing jumps | [reference/tabs.md](reference/tabs.md) § Nothing jumps |
| Details | [reference/tabs.md](reference/tabs.md) § Details<br>[reference/tabs.md](reference/tabs.md) § For contributors |

## [`docs/task-numbers.md`](../docs/task-numbers.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro (Task numbers and repo keys) | [reference/task-numbers.md](reference/task-numbers.md) § The name of a task |
| The tag on a row | [reference/task-numbers.md](reference/task-numbers.md) § The tag on a row |
| Everywhere else a task is named | [reference/task-numbers.md](reference/task-numbers.md) § All other places that name a task |
| The key beside a repo's name | [reference/task-numbers.md](reference/task-numbers.md) § The key next to the name of a repo |
| Short form or id | [reference/task-numbers.md](reference/task-numbers.md) § Short form or id |
| How a repo gets its key | [reference/task-numbers-repo-keys.md](reference/task-numbers-repo-keys.md) § Every repo and folder has a key<br>[reference/task-numbers-repo-keys.md](reference/task-numbers-repo-keys.md) § How the board makes a key |
| Changing a key | [reference/task-numbers-repo-keys.md](reference/task-numbers-repo-keys.md) § Where a key comes from<br>[reference/task-numbers-repo-keys.md](reference/task-numbers-repo-keys.md) § Rules for a key<br>[how-to/task-numbers-change-a-repo-key.md](how-to/task-numbers-change-a-repo-key.md) (whole page) |
| The key names the task | [reference/task-numbers.md](reference/task-numbers.md) § The key names the task |
| Finding a task | [how-to/find-a-task.md](how-to/find-a-task.md) (whole page) |

## [`docs/task-panel.md`](../docs/task-panel.md)

| Part of the old page | Where it is now |
| --- | --- |
| The task panel | [reference/task-panel.md](reference/task-panel.md) § The task panel |
| intro | [reference/task-panel.md](reference/task-panel.md) § Open and close the panel |
| One form for every part | [reference/task-panel.md](reference/task-panel.md) § The form of each part |
| The parts, top to bottom | [reference/task-panel.md](reference/task-panel.md) § The parts, from top to bottom |
| The settings part | [reference/task-panel.md](reference/task-panel.md) § The settings part |
| The settings page | [reference/task-panel.md](reference/task-panel.md) § The settings page<br>[how-to/change-a-setting.md](how-to/change-a-setting.md) § Change a setting of a task on the board |
| The reviews | [reference/task-panel.md](reference/task-panel.md) § The reviews |
| A narrow panel | [reference/task-panel.md](reference/task-panel.md) § A narrow panel |
| A short window | [explanation/task-panel.md](explanation/task-panel.md) § A short window |
| A part that fails to draw takes only itself | [explanation/task-panel.md](explanation/task-panel.md) § A part that cannot show |
| Why it looks like this | [explanation/task-panel.md](explanation/task-panel.md) § Why the panel looks like this |

## [`docs/testing.md`](../docs/testing.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [contributing/testing.md](contributing/testing.md) § What the check does<br>[contributing/run-the-tests.md](contributing/run-the-tests.md) § The commands |
| One command | [contributing/testing.md](contributing/testing.md) § One command for everyone<br>[contributing/testing.md](contributing/testing.md) § The steps of the check<br>[contributing/run-the-tests.md](contributing/run-the-tests.md) § How a name picks tests |
| It remembers what passed | [contributing/testing.md](contributing/testing.md) § The check remembers what passed |
| The shorter check of a fix round | [contributing/testing.md](contributing/testing.md) § The shorter check of a fix round |
| One test program | [contributing/testing.md](contributing/testing.md) § One test program<br>[contributing/run-the-tests.md](contributing/run-the-tests.md) § Add a test file |
| Several processes at once | [contributing/testing.md](contributing/testing.md) § Several processes at once<br>[contributing/run-the-tests.md](contributing/run-the-tests.md) § Change how the check runs |
| Builds take turns | [contributing/testing.md](contributing/testing.md) § Builds take turns<br>[contributing/run-the-tests.md](contributing/run-the-tests.md) § Test a change |
| One build cache for every worktree | [contributing/testing-build-cache.md](contributing/testing-build-cache.md) § One build cache for every worktree (§ What the cache keeps, § Why not one shared `target/`) |
| A new worktree starts warm | [contributing/testing-build-cache.md](contributing/testing-build-cache.md) § A new worktree starts warm |
| Its share of the cores | [contributing/testing.md](contributing/testing.md) § Its share of the cores<br>[contributing/run-the-tests.md](contributing/run-the-tests.md) § Test a change, § Change how the check runs |
| It tells the board where it is | [contributing/testing.md](contributing/testing.md) § The check tells the board where it is |
| No time budget | [contributing/testing.md](contributing/testing.md) § No time budget |
| Light tests | [contributing/testing-light-tests.md](contributing/testing-light-tests.md) § What makes a test light |
| The stand-ins | [contributing/testing-light-tests.md](contributing/testing-light-tests.md) § The stand-ins |
| Four kinds of tests | [contributing/testing-light-tests.md](contributing/testing-light-tests.md) § Four kinds of tests, § A test never touches your real board, § Real programs only where they are what is tested<br>[contributing/run-the-tests.md](contributing/run-the-tests.md) § Write the kept frames again, § Run the program by hand |
| Looking at the real thing | [contributing/run-the-tests.md](contributing/run-the-tests.md) § Look at the real thing |

## [`docs/titles.md`](../docs/titles.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro (Titles) | [reference/titles.md](reference/titles.md) § The limit |
| A title that is too long | [reference/titles.md](reference/titles.md) § A title that is too long<br>[explanation/titles.md](explanation/titles.md) § Why the board shortens and does not refuse |
| Why 60 | [explanation/titles.md](explanation/titles.md) § Why 60 |
| How characters are counted | [reference/titles.md](reference/titles.md) § How the board counts characters<br>[explanation/titles.md](explanation/titles.md) § Why the board counts cells |
| Who is told | [reference/titles.md](reference/titles.md) § Who knows the limit |
| Titles from before the limit | [reference/titles.md](reference/titles.md) § Titles from before the limit<br>[reference/titles.md](reference/titles.md) § For contributors |

## [`docs/troubleshooting.md`](../docs/troubleshooting.md)

| Part of the old page | Where it is now |
| --- | --- |
| Troubleshooting | [how-to/troubleshoot.md](how-to/troubleshoot.md) § Troubleshoot the board |
| intro | [how-to/troubleshoot.md](how-to/troubleshoot.md) § Start here |
| Quick checks | [reference/troubleshooting-checks-and-logs.md](reference/troubleshooting-checks-and-logs.md) § Quick checks |
| The health check | [reference/troubleshooting-checks-and-logs.md](reference/troubleshooting-checks-and-logs.md) § The health check<br>[how-to/troubleshoot.md](how-to/troubleshoot.md) § Start here |
| An agent could not start | [how-to/troubleshoot.md](how-to/troubleshoot.md) § An agent could not start |
| A model name is refused | [how-to/troubleshoot.md](how-to/troubleshoot.md) § A model name is refused<br>[how-to/troubleshoot.md](how-to/troubleshoot.md) § The board says "its list cannot be read"<br>[how-to/troubleshoot.md](how-to/troubleshoot.md) § A new Claude model is refused<br>[how-to/troubleshoot.md](how-to/troubleshoot.md) § An effort is refused |
| "can't be used with your ChatGPT sign-in" while the agent works | [how-to/troubleshoot.md](how-to/troubleshoot.md) § "can't be used with your ChatGPT sign-in" while the agent works |
| A task seems stuck | [how-to/troubleshooting-stuck-task.md](how-to/troubleshooting-stuck-task.md) § one section per row message, § The agent shows a dialog, § The agent asked a question in plain words and stopped |
| A done task stays in Herdr's sidebar | [how-to/troubleshoot.md](how-to/troubleshoot.md) § A done task stays in the Herdr sidebar |
| Behind main: main could not be fetched | [how-to/troubleshooting-behind-main.md](how-to/troubleshooting-behind-main.md) § all sections<br>[explanation/troubleshooting-behind-main.md](explanation/troubleshooting-behind-main.md) § all sections |
| Where the logs are | [reference/troubleshooting-checks-and-logs.md](reference/troubleshooting-checks-and-logs.md) § The log files |
| The board itself | [how-to/troubleshoot.md](how-to/troubleshoot.md) § `herdr-projects` is not found, § The board key does nothing, § The board looks cut off or half empty, § Herdr refuses the plugin, § The tasks of another machine do not show |

## [`docs/under-way.md`](../docs/under-way.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [explanation/under-way.md](explanation/under-way.md) § The row shows the action at once |
| What a key shows at once | [explanation/under-way.md](explanation/under-way.md) § What a key shows at once |
| When it ends | [explanation/under-way.md](explanation/under-way.md) § When the work ends |
| When it fails | [explanation/under-way.md](explanation/under-way.md) § When the work fails |
| Pressing the key again | [explanation/under-way.md](explanation/under-way.md) § When you press the key again<br>[explanation/under-way.md](explanation/under-way.md) § A task that is not in view |

## [`docs/waiting-for-you.md`](../docs/waiting-for-you.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [explanation/waiting-for-you.md](explanation/waiting-for-you.md) § Waiting is not finishing |
| How an agent says it waits | [explanation/waiting-for-you.md](explanation/waiting-for-you.md) § How an agent says that it waits |
| A report saved early | [explanation/waiting-for-you.md](explanation/waiting-for-you.md) § A report saved early |
| Asking cannot dodge a review | [explanation/waiting-for-you.md](explanation/waiting-for-you.md) § Asking cannot prevent a review |
| A review that is already running | [explanation/waiting-for-you.md](explanation/waiting-for-you.md) § A review that is already running |
| Details: how the board decides which turn waits | [explanation/waiting-for-you.md](explanation/waiting-for-you.md) § How the board decides which turn waits |

## [`docs/watcher.md`](../docs/watcher.md)

| Part of the old page | Where it is now |
| --- | --- |
| The Watcher | [explanation/watcher.md](explanation/watcher.md) § The Watcher (title) |
| intro | [explanation/watcher.md](explanation/watcher.md) § What it is |
| When it looks | [explanation/watcher.md](explanation/watcher.md) § When it looks |
| Between looks | [explanation/watcher.md](explanation/watcher.md) § Between looks |
| Before the first look | [explanation/watcher.md](explanation/watcher.md) § Before the first look |
| A lighter start | [explanation/watcher.md](explanation/watcher.md) § A lighter start |
| Measured | [explanation/watcher.md](explanation/watcher.md) § Measured |

## [`docs/web-chat-box.md`](../docs/web-chat-box.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [reference/web-chat-box.md](reference/web-chat-box.md) § intro<br>[explanation/web-chat-box.md](explanation/web-chat-box.md) § One program for the eye<br>[explanation/web-chat-box.md](explanation/web-chat-box.md) § Built again, not loaded |
| # The web board's chat box | [reference/web-chat-box.md](reference/web-chat-box.md) § # The web board's chat box<br>[explanation/web-chat-box.md](explanation/web-chat-box.md) § # Why the web board's chat box is built this way |
| Using it | [reference/web-chat-box.md](reference/web-chat-box.md) § Controls |
| What is the same as the web UI | [reference/web-chat-box.md](reference/web-chat-box.md) § What is the same as in the web UI<br>[reference/web-chat-box.md](reference/web-chat-box.md) § Attaching files<br>[reference/web-chat-box.md](reference/web-chat-box.md) § Dictation |
| What is the board's own, in that shape | [reference/web-chat-box.md](reference/web-chat-box.md) § What is the board's own<br>[explanation/web-chat-box.md](explanation/web-chat-box.md) § Where the board's own parts go<br>[explanation/web-chat-box.md](explanation/web-chat-box.md) § Why a draft is never lost |
| What the web UI's box has and the Board's has not | [reference/web-chat-box.md](reference/web-chat-box.md) § What the box of the web UI has and the box of the Board does not<br>[explanation/web-chat-box.md](explanation/web-chat-box.md) § Why some controls are not there |
| How it was measured | [contributing/web-ui.md](contributing/web-ui.md) § How the chat box was measured<br>[contributing/web-ui.md](contributing/web-ui.md) § Where the script is<br>[contributing/web-ui.md](contributing/web-ui.md) § Results |
| When the web UI changes its box | [contributing/web-ui.md](contributing/web-ui.md) § When the web UI changes its chat box |

## [`docs/web-ui.md`](../docs/web-ui.md)

| Part of the old page | Where it is now |
| --- | --- |
| intro | [explanation/web-ui.md](explanation/web-ui.md) § The board inside the web UI<br>[reference/web-ui.md](reference/web-ui.md) § intro<br>[how-to/set-up-the-web-ui.md](how-to/set-up-the-web-ui.md) § Before you start |
| # The board in the Herdr web UI | [reference/web-ui.md](reference/web-ui.md) § # The web board<br>[explanation/web-ui.md](explanation/web-ui.md) § # How the web board works |
| Setting it up | [how-to/set-up-the-web-ui.md](how-to/set-up-the-web-ui.md) § Show the board in the web UI<br>[how-to/set-up-the-web-ui.md](how-to/set-up-the-web-ui.md) § Link or unlink by hand<br>[how-to/set-up-the-web-ui.md](how-to/set-up-the-web-ui.md) § Open the board on the machine itself<br>[how-to/set-up-the-web-ui.md](how-to/set-up-the-web-ui.md) § Turn the web board off<br>[reference/web-ui.md](reference/web-ui.md) § Commands<br>[reference/web-ui.md](reference/web-ui.md) § Addresses, files and names<br>[tutorials/web-board.md](tutorials/web-board.md) § Step 1: Show the board in the web UI |
| The Boards section | [reference/web-ui.md](reference/web-ui.md) § The Boards section<br>[reference/web-ui.md](reference/web-ui.md) § The entries<br>[reference/web-ui.md](reference/web-ui.md) § Picking an entry<br>[tutorials/web-board.md](tutorials/web-board.md) § Step 2: Find the Boards section<br>[tutorials/web-board.md](tutorials/web-board.md) § Step 3: Open the board of this machine |
| What the board looks like there | [explanation/web-ui.md](explanation/web-ui.md) § What the board looks like there<br>[explanation/web-ui.md](explanation/web-ui.md) § What the web chat does differently, and why<br>[reference/web-ui-parts.md](reference/web-ui-parts.md) § The web board and the terminal board (and its subsections The list, Selecting a task, The open task, More…, Questions and notices, Automations, Filters, Machines that do not answer)<br>[reference/web-ui.md](reference/web-ui.md) § The page<br>[reference/web-ui.md](reference/web-ui.md) § Refresh and browsers<br>[reference/web-ui.md](reference/web-ui.md) § Other machines<br>[reference/web-chat-box.md](reference/web-chat-box.md) § The switch<br>[reference/web-chat-box.md](reference/web-chat-box.md) § The draft lock<br>[reference/web-chat-box.md](reference/web-chat-box.md) § The conversation<br>[reference/web-chat-box.md](reference/web-chat-box.md) § Earlier and later<br>[reference/web-chat-box.md](reference/web-chat-box.md) § Hide and Show<br>[reference/web-chat-box.md](reference/web-chat-box.md) § Controls<br>[tutorials/web-board.md](tutorials/web-board.md) § Step 4: Open a task<br>[tutorials/web-board.md](tutorials/web-board.md) § Step 5: Send a message to the Coordinator |
| The same code decides | [explanation/web-ui.md](explanation/web-ui.md) § The same code decides<br>[contributing/web-ui.md](contributing/web-ui.md) § The code |
| Other machines | [reference/web-ui.md](reference/web-ui.md) § Other machines<br>[explanation/web-ui.md](explanation/web-ui.md) § The web UI on one machine, the tasks on another<br>[how-to/set-up-the-web-ui.md](how-to/set-up-the-web-ui.md) § Show the board of another machine<br>[how-to/set-up-the-web-ui.md](how-to/set-up-the-web-ui.md) § Show the tasks of a machine without the web UI<br>[how-to/set-up-the-web-ui.md](how-to/set-up-the-web-ui.md) § Fix a missing machine |
| The terminal board in the web UI's terminal view | [reference/web-ui-terminal-view.md](reference/web-ui-terminal-view.md) § What the board does in the terminal view<br>[reference/web-ui-terminal-view.md](reference/web-ui-terminal-view.md) § Keys<br>[reference/web-ui-terminal-view.md](reference/web-ui-terminal-view.md) § Going into an agent<br>[explanation/web-ui.md](explanation/web-ui.md) § The terminal board in the terminal view<br>[explanation/web-ui.md](explanation/web-ui.md) § Why the board does not go into an agent on another machine there<br>[how-to/set-up-the-web-ui.md](how-to/set-up-the-web-ui.md) § Type a key in the terminal view |
| Who gets in | [explanation/web-ui.md](explanation/web-ui.md) § Who gets in<br>[reference/web-ui.md](reference/web-ui.md) § Who can do what |
| Why a front door | [explanation/web-ui.md](explanation/web-ui.md) § Why a front door<br>[explanation/web-ui.md](explanation/web-ui.md) § What the front door does<br>[reference/web-ui.md](reference/web-ui.md) § Addresses, files and names |
| What the link step changes, and how it goes back | [explanation/web-ui.md](explanation/web-ui.md) § What the link step changes, and how it goes back<br>[explanation/web-ui.md](explanation/web-ui.md) § The two do not pull one link back and forth<br>[explanation/web-ui.md](explanation/web-ui.md) § Why the front door has a port of its own<br>[explanation/web-ui.md](explanation/web-ui.md) § An offline copy |
| When the web UI changes | [explanation/web-ui.md](explanation/web-ui.md) § When the web UI changes<br>[contributing/web-ui.md](contributing/web-ui.md) § When the web UI changes<br>[how-to/set-up-the-web-ui.md](how-to/set-up-the-web-ui.md) § After the web UI updates |
| Trying it against a copy | [contributing/web-ui.md](contributing/web-ui.md) § Trying it against a copy<br>[reference/web-ui.md](reference/web-ui.md) § Addresses, files and names (`HERDR_PROJECTS_WEBUI` row) |
| Checked | [contributing/web-ui.md](contributing/web-ui.md) § What was checked<br>[contributing/web-ui.md](contributing/web-ui.md) § Other machines and the terminal view<br>[contributing/web-ui.md](contributing/web-ui.md) § The Boards section |
