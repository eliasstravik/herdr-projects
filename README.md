# Shepherdr

Run many coding agents across machines from a phone, tablet or desktop. Shepherdr is a standalone herdr-core client: one private web
app on your tailnet (installable on a phone) with your spaces in a sidebar, as herdr has them: a space is a folder on
a machine plus a name, with its own coordinator. Each machine is a section, as Slack's: its emoji (a server icon without
one) and its name, both set in its ⋯ menu's Rename machine and kept for every device, with its spaces under it, its
home space (its home folder, ~) first until dragged; the machines sit side by side with Pinned and the sections you
make. Sections only organize spaces: one at the top (the sidebar header's +) holds spaces from any machine, one inside a
machine (its ⋯, New section) only that machine's; a space moves into one from its ⋯ (Move to section) or by a drag onto
it, and back where it was when taken out or its section is deleted. Pinned is one of them, built in. Headings are only
headings: no chat, never selected; a tap anywhere on one folds it with its chevron, remembered on the device. Folded,
it hides all its spaces, with a quiet dot where one is unread (grey) or waits on you (red); going to a space opens the
heading it sits under. Spaces keep Slack's # icon. Each space's coordinator chat is in the middle and its
tasks on the right. Under their filter box, chips say what Tasks and Artifacts show: a space (`# gtm · vm-str`) or a
whole machine, any of them (none: everything), and the filter's words narrow within them. They follow the space you
pick; × takes one off and **+ Filter** adds a space or a machine from the sidebar's tree. The pin in their header keeps
the chips while the middle changes (a task from them opens in the middle); unpinned, they follow again. Chips and pin
are remembered on the device; a phone's scroll sideways. Every agent, coordinator or
task agent, gets the same chat: replies, reactions, files, search, talk-to-type, voice calls and artifacts. A task's chat
also has a side panel: its live core terminal, its files, its diff against main and its pull request.

Shepherdr stands alone: it talks to herdr-core and needs nothing else of yours. Agents keep running in core terminals,
with the app open or closed; Claude Code and Codex work (chats are read from their own logs).

## Install

Needs herdr-core 0.1.14 (its `hc` on the PATH), [Bun](https://bun.sh) 1.4.2 and Git, on Linux or macOS, Claude Code or
Codex installed and signed in, and Tailscale on the machine and your devices. Tasks' shared browser also needs Chrome for
agent-browser, once on each machine that runs tasks: `bin/agent-browser install` (on Linux, `install --with-deps` adds
what Chrome needs). Other machines need a few of these too (More machines, below).

```sh
git clone https://github.com/eliasstravik/shepherdr && cd shepherdr
bun install --frozen-lockfile
bun run build
export PATH="$PWD/bin:$PATH"     # for good: put this checkout's bin on your PATH
shepherdr start
```

[docs/core/start.md](docs/core/start.md) has the details. `shepherdr start` starts core's own `shepherdr` session if it
isn't running, then the app in the background, and makes its home, `~/.shepherdr`, on a first run. `shepherdr status`
says how it is, and `shepherdr stop` stops the app: agents keep running in core. To start it at login and bring it back
after a crash, the same page has a service for systemd and launchd. Shepherdr serves on 127.0.0.1 only and never
changes Tailscale itself, so share it once, privately on your tailnet (`tailscale serve`, never public):

```sh
tailscale serve --bg --set-path /shepherdr 7787
shepherdr status
```

`shepherdr status` then prints the link, `https://<machine>.<tailnet>.ts.net/shepherdr/`. On an iPhone, open it in Safari and add it to the
home screen: alerts need that.

### Upgrading from 2.0

Shepherdr 2.1 is not an update of the 2.0 herdr plugin: `herdr plugin update` doesn't take you there, and it brings
nothing over. It starts with a fresh home, with no tasks, chats, spaces, settings, secrets or sign-ins from 2.0, and
refuses a `~/.shepherdr` that holds 2.0's state. To move:

1. Keep 2.0 from following main: stop its updater and run it from a checkout of the `v2.0` branch.
2. Finish or stop the tasks in flight first, on every machine: nothing may still write to its home.
3. Stop 2.0, back up all of `~/.shepherdr`, check the backup, and move the home to `~/.shepherdr-legacy`.
4. Install 2.1 as above. Add your spaces again, and give your secrets again in Settings → Secrets.

[The full steps](docs/core/cutover.md) name everything to stop and check. Your own plugins stay with 2.0's home; a copy
needs a small change for 2.1 (Plugins, below).

Later updates are by hand (Updates, below).

### Going back to 2.0

2.0 stays on the branch `v2.0`, with its own README. To go back with your old tasks and settings as they were, finish or
stop 2.1's tasks (`shepherdr stop` leaves them running in core), then:

```sh
shepherdr stop                       # stop 2.1
mv ~/.shepherdr ~/.shepherdr-2.1     # keep its home whole, for a later try
mv ~/.shepherdr-legacy ~/.shepherdr  # 2.0's home back where it was
```

Then start your `v2.0` checkout, or install it afresh: `herdr plugin install eliasstravik/shepherdr --ref v2.0 --yes`.
Never run the two on one home, and never merge the homes. To return to 2.1, swap the two folders back.

herdr-projects 1.2.27 stays on the branch `legacy-herdr-projects` and the tag `v1.2.27`, with its own README; how to go
from it to Shepherdr 2.0 and back is in the `v2.0` README.

### Install it as an app, with alerts

Install the link as an app on each device, open it, and turn alerts on in Settings → Notifications → **Add this
device**.

- **iPhone or iPad**: in Safari, Share → **Add to Home Screen**, then open it from the home screen (alerts only work
  there).
- **Mac**: in Safari, File → **Add to Dock**; or in Chrome, the install icon at the right of the address bar (or ⋮ →
  Cast, save and share → **Install page as app**). Chrome alerts come while Chrome runs.
- **Omarchy** (or any Linux): in Chromium, ⋮ → Cast, save and share → **Install page as app**, or Omarchy's own
  Install → Web App with the link. Alerts show through mako while Chromium runs.

Tasks' alerts always come; what a space's coordinator says follows that space's Notify (All, Needs me or Off: below, under Use). Each alert says the task and what it needs (`SHE-63 · Plan to approve`), and the question or the card's choices under
it (`Approve the plan · Send it back`), so you can decide from the lock screen. A tap opens the app on that very card,
lit up for a moment, its buttons ready (a done task: its card in the space's chat). One alert per task: its newer
alert takes the old one's place, and it goes once the task no longer waits on you (with the next alert, or when you
open the app or that card). The app icon counts what waits on you (on the home screen and the Mac's Dock); Linux has
no icon count, so the window title says it: `(3) Shepherdr`. On Android, Mac Chrome and Linux, alerts also carry the
card's first two choices as buttons that answer it at once (never one that needs a note). Alerts buzz only while the
app is closed or in the background.

### Updates

Shepherdr doesn't update itself: it never follows main, so a merge can't change what runs. To update, stop the app, move
your checkout to the new version, build it and start it again with the same home:

```sh
shepherdr stop
git pull --ff-only
bun install --frozen-lockfile && bun run build
shepherdr start
```

Agents keep running in core meanwhile. Check first that the new version pins the core you run (start refuses another),
and keep the old build and a backup of `~/.shepherdr` to go back to; never run two apps on one home. Every open screen
then reloads itself to the new version: what you were typing and the files you attached come back, and it waits until
you stop typing, finish a voice call or stop talking to type. A screen reloads only when the screen itself changed.

By hand: `shepherdr start --foreground` serves on `http://127.0.0.1:7787` from the terminal (`--port` or `SHEPHERDR_PORT` to change it).

### More machines

Shepherdr is installed once, on the main machine. Every other machine you add to herdr-core shows up on its own (with
core's own remote commands, `hc --help`, over the SSH login you already have; Shepherdr sets up no access or keys): its
spaces (its home space at least), tasks and chats, with their coordinators, run from the main machine. The other machine
needs herdr-core 0.1.14, git, Bun and the agent CLIs, signed in (gh for pull requests of GitHub repos, Chrome for the
shared browser); nothing of Shepherdr's to install by hand.

- How: the main machine talks only to its own core, which owns the link to the others. Agents, terminals, live events,
  files, git, the Files and Diff panels and merges all go through it. Shepherdr installs no worker, mirror, tunnel or
  listener there.
- The helper: Shepherdr puts a small private folder there through core, under `~/.local/share/shepherdr-agents/`: the
  `shepherdr` command for its agents (it reaches Shepherdr through an authenticated, encrypted mailbox in that machine's
  core, so no API goes on the network), the hook recorder, the skills, and the pinned agent-browser. It goes there when
  an agent first starts on that machine, and again whenever its copy changed.
- Settings → Machines says how each one is: Reachable or Can't reach.
- A task's shared browser runs on the task's own machine, next to its app; your screen takes its frames and sends your
  input through the main machine and core.
- A machine that can't be reached says `Can't reach` on its heading; its chats keep what they last showed and what you
  send waits until it is back.

### Voice

Calls go through an ElevenLabs voice agent (Shepherdr makes one for you, with no recording and no retention). Give it
your key once, as `ELEVENLABS_API_KEY` in Settings → Secrets, or on the main machine:

```sh
shepherdr secret set ELEVENLABS_API_KEY < key.txt
```

## Use

- Pick a space; write to its coordinator. Ask it for a task; tasks are created only through a coordinator.
- What you send shows in the chat the moment you tap Send, faded until it is in. If it can't go through, it says so
  under the message, with Retry and Remove; it is still there after a reload.
- Write to a busy agent any time: it goes in at once and steers the agent mid-turn (Claude Code and Codex take it
  at their next step). It waits only while the agent has a permission card open (it goes in once that is answered), or
  while the agent is still starting. A model or effort change restarts the agent, from the message box's + menu.
- Permission prompts appear as choice cards. Answers are checked before messages go in. A CLI's own startup prompts
  (trusting a folder, Codex's hooks) stay in its terminal, since Shepherdr never reads the screen: answer them in the
  Terminal panel; for Codex the chat says so.
- A slash command you type (`/clear`, `/new`, `/compact`, `/model sonnet`, a custom one) goes into the agent's CLI as
  you typed it, and the CLI runs it. A menu or view the command opens stays in the terminal: use or close it in the
  Terminal panel, so your next message doesn't go into it.
- Every machine has at least one space: its **home space**, at its home folder (~), named `home`. It can be renamed
  but not removed; it comes first under its machine until dragged, and moves like any space. Its coordinator also looks after the machine: health, load and
  disk news, its processes, adding spaces. The main machine's home space also sees every machine: what needs you
  across them all, which alerts reach your phone, and alerts about a whole machine. A space's tasks list holds its
  own tasks, a home space's too; a machine's chip lists its spaces', grouped by status or by space, and chips on more
  machines (or none: everything) also by machine, or machine › space. A machine's heading has a + (New space) and a ⋯ (New space…, New section…,
  Rename machine…: its emoji and name), and says when it is in trouble: `Can't reach`, or a limit it is over.
- Add a space with the + on a machine's heading (it shows on hover with a mouse), its ⋯ (New space…), `N` on a desktop
  (on the machine you are in), or by asking a coordinator (`add ~/dev/x as a space`, or `shepherdr space add <path>
  [--name NAME]`). Pick the machine when there is more than one, then a folder in your home there: the picker starts at
  ~ (~ itself is refused: it is the home space). Type the path to narrow the list (Tab goes into a folder, Enter
  picks); your latest repos are listed first. Then its name, filled in from the folder's. Names follow Slack's channel
  rules: lowercase letters, numbers, hyphens and underscores, at most 80 characters, each its own on its machine;
  typing lowercases and turns spaces into `-` (`My Project` becomes `my-project`), and a taken or bad name is refused
  in the box. The name is the space's slug in links (`?place=vm-str/gtm`) and in the CLI; its tasks' id prefix (SHE,
  TG…) is its own. A repo is added as it is; any other folder, or one not made yet, becomes a new repo first (git init
  on main with an empty first commit; files already there stay uncommitted). Either way it gets its own
  coordinator, and opens. Only folders in your home are listed or made.
- New messages show like Slack's unread, for coordinators' chats only: a coordinator's chat with something new from it
  or Shepherdr that you haven't seen makes its space's row bold in the sidebar (a done task's last word comes as its
  card in the space's chat, so it counts). A folded heading still shows the spaces under it with something new or a
  red count, and the open one, as a collapsed Slack section does. Headings carry no marks of their own. Having a chat on screen reads it,
  and the main machine keeps the marks, so every device shows the same; a row's menu has **Mark as read**. Tasks' chats never make a row bold: the tasks list shows each
  task's own status. A row's red count still shows what waits on you, a task's need ending as a card in its chat.
- Each space's row menu sets its coordinator's **Notify**:
  **All** (every new message from it buzzes too), **Needs me** (the default: its questions and alerts) or **Off**
  (nothing from it; muted, its row stays dimmed with a crossed-out bell and new messages don't make it bold, though
  a red count still shows what waits on you). Tasks alert as always, whatever it says. Settings → Notify sets it
  Everywhere, for a machine or for a space; the most specific one set wins, as Settings' other defaults, and picking
  in a row's menu what it would get anyway goes back to following it.
- Rename a space from its row's menu, or a machine from its heading's ⋯: its name becomes a box right there; Enter (or tapping
  away) keeps the new name, Escape the old, and an empty one gives it back its own. Only the name you read changes,
  everywhere it shows (the sidebar, the header, search, task rows, alerts, links) and on every device: the main machine
  keeps it. A space's new name follows the same rules as a new one's (`shepherdr space rename <name>
  <new-name>`). Its folder, repo, task ids and the machine's own name in core stay as they were.
- Remove a space from its row's menu (a held finger on a phone or tablet; its ⋯ or a right click on a
  desktop, where it opens beside the row): Remove asks first. Its folder and code stay on disk; Shepherdr forgets it and closes its
  coordinator. Its open tasks hide with it (their worktrees and branches stay) and come back, with its chat, when it is
  added again under the same name. Not while one of its tasks is working: pause it or let it finish first. Never a
  home space, so a machine always keeps one. A coordinator can do the same (`shepherdr space remove <name>`).
- Machines, and each machine's spaces (its home space first), show A to Z. Drag one to reorder them (with a mouse at
  once; a finger holds the row, its menu opens, and sliding it closes the menu and moves the row); a machine takes its
  spaces and sections along, a section its spaces, and Option+Shift+↑/↓ (Alt+Shift off a Mac) moves a focused space. The main
  machine keeps the order, so every device sees it; a space added later slots in A to Z among the
  rest.
- A space's ⋯ pins it (Pin / Unpin), as Slack stars a channel: it moves out of the tree into Pinned, a heading at the
  top of the sidebar that folds like the others (each pin with its machine after its name), and Unpin puts it back.
  Only spaces pin. Pins drag into order and Option+↑/↓ walks them first; the main machine keeps them with the order,
  so every device has them.
- Sections, as Slack's, only organize spaces. The sidebar header's +
  makes one at the top, beside Pinned, for any machine's spaces (each shows its machine after its name); a machine's ⋯
  (New section…) one inside it, one small step in, for that machine's only. A space's ⋯ › Move to section moves it
  into one, or back under its machine; a drag onto a section's heading or among its spaces does the same. A section
  folds, drags among its peers, and its ⋯ renames it (emoji and name) or deletes it: its spaces go back where they
  were. Pinned is one of them that can't be deleted.
- A task is one agent, a child in core, started at once in its own git worktree and branch from its space's base branch
  (`work/<n>-<slug>`). It lives on when its coordinator restarts. Your own checkout is never touched. A fresh worktree gets a copy
  of the ignored files the repo's `.worktreeinclude` names (`.env` and the like); nothing else runs. A task has no
  settings: its brief is all the agent goes by. The coordinator writes the work and your wishes for it ("wait until I
  say go", "skip the review", "ask me before merging"), and Shepherdr ends every brief with its Settings block,
  one plain line per setting that shapes how the agent works, the source it follows: whether it is reviewed and by whom (the reviewer tags each finding critical, major, minor or trivial; the agent fixes
  those at or above Must fix, `review.mustFix`, major by default, and reviews again, at most Review rounds,
  `review.rounds`, 3 by default), and whether it merges on its own (by default once its own quick checks pass,
  without waiting for GitHub's checks), on its own after GitHub's checks, or asks you first, and the agent, model
  and effort it runs on (only so it knows). The coordinator can set a line for this task alone, and the block says so
  (`task add --set review.rounds=1 --set merge="ask me first"`). The brief's own words win, so a wish for one task needs no setting
  changed. The task's chat shows the block at the end of its first message. The rest is the agent's judgment, by its
  skills: whether to plan first, how to build and check it, what to fix after a review and what proof to show. Ask for another CLI, model or effort for one task and the coordinator adds
  `--agent`, `--model` or `--effort` to `task add`; for another reviewer, `--reviewer`, `--reviewer-model` or
  `--reviewer-effort`. Tap the task and you are in its agent's chat, where it asks, shows
  artifacts and tells you when it is done; the task's brief shows exactly what its agent got.
- Want something noted but not started? Ask for it "for later": the coordinator adds it with `--later`, and its agent
  starts all the same, with "don't start yet, wait for a go" in its brief. It says in one line that it has it and the task shows To do (`shepherdr todo`).
  Tap it and you are in its chat, like any other: say go when you are ready, or say what to change. The agent decides
  from your words.
- "Do this after that" ("then", "once that is merged", "after SHE-12") queues it: the coordinator adds it with
  `--after SHE-12` (`shepherdr task queue <id> --after …` changes or clears it on a task that exists). Its agent starts
  at once, reads the brief and waits; the task shows under Queued, its line naming what it still waits for (`after
  SHE-12, SHE-13`). Once they are all done (merged, or nothing to merge), Shepherdr tells its agent to go,
  and a small card in its chat says why it started. A task of another machine is named with its machine first
  (`vm-str:SHE-12`), from the main machine's home space. If one it waits for is stopped or goes wrong, the queued task needs
  you, saying which; it goes back to waiting once that task is going again. Want it started early? Just write to it:
  its agent decides from your words, and clears its queue itself when they are a go (a note leaves it queued).
- A task's dot comes from its agent alone: Working, Needs you (it asks, or waits for your reply), Queued (it waits
  for other tasks), To do (it waits for a go, or is still starting up; a To do row is its title alone), Ready
  (`shepherdr ready`: finished, waits for you to merge it), Merging (`shepherdr merge` under way: GitHub's checks run,
  a rebase or the merge itself, nothing for you to do) and Done (`shepherdr merge`: Merged, or closed: Nothing to merge). The list and the task's chat always show
  the same. Once started, only the agent, or stopping the task, puts it in To do; one that stops for any other reason needs you, except
  while work it started in the background (a reviewer subagent, a shell, a monitor) still runs and will wake it: that
  is still Working (shells alone for up to 10 minutes, since a server it started for you never wakes it; a shell
  running a headless reviewer of another CLI, `codex exec` or `claude -p`, counts as a subagent), unless its last
  words ask you something. So is a task whose agent ended its turn for the reviewer `shepherdr review` started: its
  line says Waiting for review, with no card, news or alert, unless its last words ask you something. A reviewer that asks, dies, stays quiet for 20 minutes or
  ends with no report reaching the agent ends that wait, and the task needs you. A task that needs you says why in at most 40
  characters, never by pasting the agent's message and never in a model's words: a standard reason (Plan to approve,
  Asks permission, Signed out, Question for you, Waits for your reply), or the agent's own question cut to fit
  (from its question tool, its choice card, or the last sentence of its last message that asks). The tasks list, the
  card in its space's chat, alerts and coordinator news all say the same; the full message stays in the task's chat.
- A turn cut off by the model's provider (overloaded, at capacity, a rate limit, a dropped connection) is no question
  for you: Shepherdr waits and tells the agent, task or coordinator, to carry on, after 1, 2, 4 and 8 minutes. Meanwhile
  a task stays Working, its line saying why and when (`Provider overloaded · retry in 2m`). Only when the fifth try is
  cut off too does it need you (`Provider overloaded, still after 5 tries`); a coordinator says so in its chat, with an
  alert. Errors waiting can't fix (signed out, out of allowance, a model that isn't there) go to you at once. Anything
  written to the agent starts the count again. Each CLI's log says what cut it off (`server/common/provider-error.ts` decides).
- When a task's agent can't go on (out of allowance, signed out, its provider still down after the tries), its
  **backup agent** takes over, once: Settings → Agents → Backup (`task.fallback`, Codex unless set, or off).
  It starts in the same worktree and branch, on Backup's model and effort, with the brief and a short handover: who it takes
  over from and why, the branch's commits, what lies uncommitted and the chat so far. A card in the task's chat says
  so. Tasks of an Autopilot run never switch agent. If the backup can't go on either, or isn't installed, the task needs you.
- Status, coordinators' too, is what the agent reports of itself. Every Claude Code and Codex Shepherdr starts gets hooks of
  its own for that start (your own settings are never touched), which record each event: a turn starts or ends, what
  still runs in the background as it ends, a permission prompt (a subagent's too), its question tool, a plan. A turn cut
  short (Esc, a prompt turned down) runs no hook; its transcript says it ended. Shepherdr never reads the screen: what
  no log, hook or the agent's own OSC 7501 report says stays unknown, and a process that lives or ends never means Done.
- On a repo with a GitHub remote that `gh` is logged in to, each task gets a pull request. The shepherdr-agent skill runs
  `shepherdr pr <id>`: it pushes the branch and opens a draft (its title the task's, its words the agent's summary, the
  brief's gist and a "Generated with" footer), and pushes to it again after new commits. `shepherdr checks <id>` shows
  GitHub's checks (merging needs them green first only when Settings → How tasks work → Merge is "after GitHub checks", or the task's --set says so);
  failed ones go back to the agent to fix. Reporting it ready takes the pull request out of draft.
  While it waits, Shepherdr looks at it every minute or so and passes news on to the resting agent once, in
  `<shepherdr>` tags: a check that failed (its link and the end of its log) and new reviews and comments with words (a
  bare approval is not news). The agent decides what to do (shepherdr-agent says how); Shepherdr only passes it on.
  Its number shows on the task's row, links from its chat's header and from the Done card. The remote is read live, as
  written and as git rewrites it (`url.insteadOf`), and `gh` is told the host (`GH_HOST`, such as `github.localhost`).
  No GitHub remote: tasks merge here, as below. No `gh` login, or GitHub not answering: nothing merges until it
  does.
- The agent merges when its brief or you say so: `shepherdr merge <id>` squash-merges the branch into main.
  On GitHub that goes through its pull request, as soon as the agent's own quick
  checks pass (in Shepherdr: `bun run typecheck` only), without waiting for GitHub's checks: they still run on
  every push to main (and on the pull request where a repo's checks do), and only a check GitHub already shows at that commit, failed or still running, stops the merge (or a login that can't read them).
  With Settings → How tasks work → Merge on "after GitHub checks", it merges only once GitHub's checks pass. Either way only
  at the commit this machine has (`gh pr merge --squash --match-head-commit`), then main here is pulled, when that
  checkout is on main and clean (else it tries again). Failed checks are sent back to the agent. A branch
  behind main is merged as it is: rebasing it would make a new commit and run all of GitHub's checks again, and again
  each time another task lands meanwhile. Only a conflict with main is rebased (by the agent), and when waiting for GitHub's checks they run on that commit before it merges. For a few hours after
  a task merges through its pull request, Shepherdr watches GitHub's runs on main: once one fails, the space's
  coordinator hears it once (news), with the failing run, the end of its log and the tasks merged since main was last
  green, and decides on a fix-forward task (Autopilot adds one by itself). It hears again only after main was green. GitHub's
  merge queue would test each one on top of the others, but it needs an organisation's repo. Elsewhere it is
  squash-merged into main here. Then the task is marked merged. Once the
  agent's turn ends, the task is put away through core: its agent closed, the worktree removed, then
  the branch deleted. Nothing is forced: a checkout with uncommitted changes, or a branch with work main lacks, is kept
  and said so (in the task's chat), and its agent closes all the same.
  Another terminal still in the worktree keeps it too. You can also just ask its coordinator to merge it.
  Task agents learn the way from brief to merged from [shepherdr-agent](skill/shepherdr-agent/SKILL.md): the review the
  brief's Settings block asks for (Settings → How tasks work → Review, Reviewer, Must fix and Review rounds; the agent
  starts it with `shepherdr review <id>`, and with Review on, `shepherdr ready` needs one of that very commit), what the change broke, left undone or added beyond the brief; only the choices no experiment can
  settle as cards for you, and your yes before anything irreversible; the proof in one small artifact; ready with a facts line; then merge or ask. How to
  build and check the work is the agent's own judgment: Shepherdr only gives it the brief, your defaults and the house
  rules.
- A task with nothing to merge (an answer, research, or work no longer wanted) is closed: `shepherdr task close <id>
  [--summary "..."]`, or just ask its coordinator. Its agent stops, its worktree and branch go (one holding
  unsaved or unmerged work is refused with what it holds, unless `--force`, which keeps it and says so) and it shows in Done with its summary
  (Nothing to merge by default); its card in the space's chat says so. An agent whose `shepherdr ready` finds no changes on
  its branch goes straight to Done the same way.
- `shepherdr task stop <id>` closes a task's agent. Its work stays: a worktree with unsaved or unmerged work
  is kept, one with none goes with its branch. Writing to the task makes a fresh worktree if it needs one
  and resumes its agent.
- `shepherdr task delete <id>` removes a task for good, and only when you plainly ask its coordinator to: its agent
  stops, its worktree, branch, chat log and record go, so it
  leaves every list, and nothing is posted. A worktree with uncommitted changes, or a branch with work main lacks, is
  refused with what it holds, unless `--force`. A merged task can be deleted too: its commits stay on main. Its number
  is never given out again.
- Archive a task from its row's ⋯ (a mouse finds it on hover; on a phone, hold the row), or ask its coordinator
  (`shepherdr task archive <id>`, only when you ask). It leaves every list and count but keeps its record, chat and
  summary: ⌘K finds it, and **Archived · 12** at the bottom of the tasks list shows a space's archived tasks, the
  latest first, each with Unarchive in its ⋯ (`shepherdr task unarchive <id>`). Archiving a working task asks first:
  its agent stops, as with `task stop`, and its work stays. Writing to an archived task brings it back. Done tasks
  (merged or closed) archive themselves after a week: Settings → Tasks → Archive Done after (`tasks.archiveAfter`: a
  number of days, or off; each machine keeps its own). `shepherdr task list` leaves archived tasks out; `--archived`
  lists them.
- The field at the top of the Tasks and the Artifacts lists filters them as you type (F gets to it on a keyboard):
  tasks by id, title, status line or summary, Done and archived ones too; artifacts by title or the task that made
  them. Esc clears it.
- Leftovers of merged tasks are put away in the background: Shepherdr tries again every minute or so until they go, and
  says so in the task's chat if one can't. Only tasks Shepherdr made and finished are looked at, never
  one whose agent is at work, never your own worktrees or branches, and nothing while the task's machine
  can't be reached. A branch counts as merged when it is still the very commit that landed, or when main holds it.
- Any agent can show you an **artifact**: a report, a dashboard, a deck, a draft screen or a small app, made as one
  HTML file (`shepherdr artifact new`). It shows as a card in the chat where it was made and opens full screen on a
  tap. Each space keeps its own in **Artifacts**, beside Tasks (a third tab on a phone): a home space lists its
  machine's spaces' too, the main machine's home space every one. When an agent changes one it updates it (`shepherdr artifact update`), making a new
  version instead of a new artifact. The chat folds the older card to a line, and an open view swaps in the new
  version while you look, with only its numbers changing when only its data did. Full screen you can step back
  through its versions, mark it up, or write to the agent that keeps it, so you change it by chatting beside it. Its
  forms and buttons send their answers to that agent, but only once you've tapped the page. Every artifact is checked at phone width, light and dark,
  before you see it: one whose scripts break, or that reaches for the network, is sent back to its agent. It runs
  sealed off: no network, no access to the app or your data, everything inline. Artifacts stay until you delete one
  (its ⋯ menu); each keeps its last 20 versions. An artifact a task's agent makes always names its task, then what it
  is (`SHE-2: Stop Goes in note on every send · Proof`), so you can tell what it was for; Shepherdr puts the name in
  when the agent left it out. Artifacts made earlier keep their titles.
- Every coordinator is an agent that does what you ask, standing requests too ("merge tasks as soon as they pass"):
  it wakes on news and acts on it when you asked it to handle such things, or tells you what matters. **Autopilot**
  is a switch at the top of its chat's ⋯ menu: it replaces the coordinator with a fresh one in the same chat that also
  works on its own, sealed in, within its Heartbeat. Off, a fresh regular coordinator takes over; the chat stays.
  What you switch stays, per space. A space's news goes to its
  coordinator. Every space's news on every machine
  (needs you, ready, stalled, merged, main red, machine health) also reaches the main machine's home space, in one line
  each: it keeps the overview and picks what buzzes your phone; the space's own coordinator acts on it. While Autopilot is on, a small red pill
  at the chat's top right says so (on a phone after the subtitle, under the name), a tap on it opens the ⋯ menu, and
  the space's row in the sidebar has a small red tag (with a mouse, its dot alone); with it off neither shows. Alerts buzz only while the app is closed or in the
  background; with it on screen they show nothing.
- Each wake, Autopilot reads its Heartbeat and saved progress, looks at the space, picks useful work the Heartbeat allows,
  acts, checks the evidence and saves where it got to (`shepherdr progress`). A direction with no end ("keep improving
  the game") goes on; a finite goal ends when it proves it (`shepherdr complete`). When things keep breaking it stops
  itself and tells you why. It
  never deletes tasks, changes Settings, starts work nobody asked for, force-pushes, or merges past a brief's "ask me
  first". It runs in an Isolated workspace, and needs a repository space on Linux with bubblewrap (a home space can't):
  its agents see only their project and own worktree, reach only the HTTPS hosts and secrets you list, and merges need
  your yes unless you choose otherwise. Settings can cap its traffic, reported tokens, time and tasks at once; there is
  no cap in money. [docs/autopilot.md](docs/autopilot.md) has the whole of it.
- Triggers: Autopilot's heartbeat and automations run on the same kind of trigger, a time rule, event rules, or both
  (`server/common/trigger.ts`). The time rule is an iCalendar recurrence rule (RFC 5545: FREQ, INTERVAL, BYDAY, BYHOUR,
  BYMINUTE, BYSETPOS, UNTIL or COUNT, EXDATE, DTSTART; read by the `rrule` package), or `once 2026-11-02 9:00`: every
  90 minutes, every other week, the last Friday of the month, start and end dates and skip dates. Event rules are news:
  a task added, ready, gone wrong, needing you, merged or stalled, main red, a machine struggling (heartbeat only), each
  with optional `space=`, `branch=` (a glob) and `title=` filters. Every time rule's times are in Settings → Time zone
  (`time.zone`, Everywhere; this machine's own zone by default). You never write a rule: ask a coordinator in your words
  ("every 30 minutes, 8 to 22 on weekdays") and it writes one (`--when '<rrule>'`, `--on '<events>'`); everywhere it
  shows in words, with its next three runs ("Every 30 minutes, 8:00-22:00, Monday to Friday (Stockholm)").
- Autopilot is a heartbeat, as OpenClaw's. Settings → Autopilot holds, per space (or a machine, or
  Everywhere): the Heartbeat, one plain-text brief (direction, priorities, limits, reporting, an optional stopping goal),
  edited there or by the agent with `shepherdr heartbeat set`, and kept on the main machine; When, a time rule (every 30
  minutes, all day), shown in words with its next runs and a few presets (anything else, ask the coordinator); Events,
  its event rules (all news, or none); its agent, model and effort; and its access and limits. On each run of the time
  rule, while its coordinator is idle, Shepherdr wakes it in its own session to go through its Heartbeat and act on what
  needs it, or reply only `<no-reply>`, which the chat never shows. A busy coordinator gets it once its turn is over;
  none while its HEARTBEAT.md is empty and no news waits. News its event rules name wakes the coordinator at once, so a news
  card gets its reply right after it; other news waits and comes with the next heartbeat (quiet nights: a time rule
  within the day's hours, and only what can't wait as its events). Run now wakes it as soon as it is idle.
- Automations are cron jobs for tasks: a space's task template (title, brief, agent) and its trigger: a time rule,
  events of its own space (or the space a rule names), or both. Each run adds a new task with a fresh agent; none runs
  in the coordinator's chat. Only agents add or change them (`shepherdr automation`), when you ask a coordinator in the
  chat. They list at the bottom of the space's tasks, paused ones too, each with its trigger in words, its next runs
  and its last runs: run it now, pause or remove it there.
- While a task's agent works, it says itself what it is doing and how far along it is, at the start and at each
  natural step: `shepherdr status <id> "Fixing the two failing tests" --done 40 --left 5m` (its instructions tell it
  to). Shepherdr guesses nothing: it shows what the agent said, and counts the minutes left down while it works. Its dot fills like a pie with
  the % done and the minutes left sit small under it in the same orange (`6m`, `<1m`); its row's second line says
  what it does in at most 40 characters, so the title and that line each keep the row's whole width. Its chat header
  says the same with the minutes at the end (`Fixing the two failing tests · ~6 min`). Coordinators report no
  progress: their typing dots say a message is on its way (at once for yours; for news, once the reply shows on its
  screen and isn't a hidden `<no-reply>`); while they only work in the background they show nothing (no dots, no word in
  the header). Task titles
  are at most 32 characters, so each fits one line on a phone. A line longer than 40 characters is cut at a word,
  and the agent is told so. An agent that waits on you keeps its last line, frozen at its % (its minutes stop too);
  once it reports ready, or waits for a go or for other tasks, it has none.
- Every chat shows what its agent does, calmly and in plain words (Claude Code and Codex, read from their logs). Just
  before a turn's last reply its steps sit as at most three quiet rows, each an icon for its kind (read, edit, run,
  search, web, subagent) and what it did: `Checked the task list`, `Edited 2 files: app.ts, panel.tsx  +12 −3`,
  `Ran tests: 41 passed`, `Searched the code for “review”`. Files read or changed group into one row, and the rows that
  say most (changes, test runs, failures) show first; `4 more · 5m` shows the rest. A tap on a row opens what it ran
  (the command, the path, the pattern) and its output or diff; no command line shows until then. The words come from
  the agent's own short description of a command (Claude Code's `description`), else from what the command runs
  (`git status` is `Checked git status`, `bun test` is `Ran tests`, an unknown program `Ran <its name>`), else the
  tool. While a turn runs, one quiet line in place of the typing dots says what it does now and for how long
  (`Checking the task list  12s`), coordinators' turns too (their header still says nothing of it). While work it
  started in the background runs (subagents, background shells and monitors, a headless `codex exec` or `claude -p`,
  which counts as a subagent), a small tag sits just above the message box (`1 subagent · 1 shell`), only in its own
  chat (task rows stay clean); a tap shows each piece, what it does now (its latest step, or its output's last line)
  and for how long, and each opens to its own steps or its recent output. Nothing streams by default and nothing
  flashes; a turn it answered with nothing (news it had nothing to say about) shows no rows.
- Every task has a shared browser, the Browser item of its side panel (a full-screen sheet on a phone). Its agent
  drives it with Shepherdr's `bin/agent-browser`, to check its app or sign in to a site, and you watch it live. Take
  over at any time (your mouse, keys, touches and the address bar go in; the agent is told to hold off) and hand it
  back. Fit sizes the page to the panel; Desktop goes back to 1280×720. Mark up: tap elements, write a note
  on each, and send them together into the task's chat; numbered pins mark them on the page, and the agent gets each
  element's selector, text, React components, box, key styles and a cropped shot. It runs on the task's machine: one
  [agent-browser](https://github.com/vercel-labs/agent-browser) session per task (private to this Shepherdr,
  with a profile of its own that keeps its sign-ins), started when either side first
  uses it. agent-browser streams it on 127.0.0.1 only; a child in core relays that stream to the server and the app, so the browser is as
  private as the app itself. A task that is Ready keeps it, to review in; stopping or archiving the task closes it
  (until you write to the task again), and merging, closing or deleting it also forgets its sign-ins. One left with no
  agent command and no one watching it in the panel closes too, sign-ins kept: Settings → Browser → Close idle browser
  after (`browser.idleClose`: 10 min, or off; per machine or space). The agent's next command or opening the panel
  starts it again. Its pages see `prefers-reduced-motion: reduce` (Chrome draws them in software, so an animation
  costs a core); an agent whose page must move turns that off with `set media no-preference`. agent-browser is
  pinned in package.json (`bin/agent-browser` runs that version, for agents too: they are told its full path, since a
  shell's start-up files can put another version first on the PATH, and two versions on one session keep restarting
  its daemon); it needs Chrome on each machine once (`bin/agent-browser install`); other machines get agent-browser, not Chrome, with
  their helper. Without it the item is greyed, with a
  hint. A browser that won't start says why over the panel, with Try again.
- Each machine watches its own health: every 30 seconds it reads its 1-minute load against its cores, memory in use,
  how full home and /tmp are, a GPU's use (nvidia-smi, where there is one) and whether core answers. A small meter at
  the top right of its home space's chat shows it (thin bars, the numbers in words where there is room; grey while all is
  well, amber while new tasks wait, red past an alert limit), and a tap shows the numbers, the tasks at work against
  the limit, core, and its settings. Settings → Machine, with that machine picked, holds its limits, each machine keeping its
  own, scaled to its cores and memory: max tasks
  working at once (10); new tasks wait while the load is over 1× the cores or memory over 80%; its coordinator is told
  when the load stays over 1.5× the cores for 2 minutes, memory or a disk is over 90%, or core stops answering.
  - **Room for tasks** (Shepherdr enforces it): a task added, or told to go (a queued task's go, or a coordinator's
    `task tell` to one that waits), waits as Queued, its line saying why (`Waiting: 10 tasks are working (limit 10)`), with no
    agent started yet, and starts by itself once there's room, oldest first. Only working tasks count. Your own message
    to a waiting task is saved and goes in once there's room.
  - **Alerts** (the coordinator decides): past a limit, a card shows in the machine's home space and its coordinator (and
    the main machine's home space's) gets the numbers; once more when it
    is back under. The shepherdr-coordinator skill says what to weigh: pause or queue tasks, clean the disk, or tell
    you. Shepherdr never pauses a running task itself.
  - **core gone**: it is said plainly ("core isn't answering") on the meter; the
    machine's heading says `Can't reach`, what you send waits until it is
    back, and its home space's coordinator hears what happened once core answers again.
- Settings (sidebar, or `,` on a desktop) holds the few app settings, in two groups. What agents are told: How tasks
  work, what goes in each new brief's Settings block (review, the Reviewer, Must fix, Review rounds,
  merge), each row offering its exact line under Brief wording. How Shepherdr runs: the agent, model and effort tasks and
  coordinators start with (model:
  an id or alias, or default for the CLI's own; effort: default, lowest, low, medium, high, xhigh or max, each CLI
  taking it its own way) and tasks' backup agent, when Done tasks archive themselves, Autopilot's Heartbeat, schedule, access and limits, the time zone of every schedule, each machine's limits, the idle browser, Notifications and Voice. Its
  defaults are set Everywhere, for a machine or for a space (`shepherdr set <key> <value> [--machine | --space NAME]`, from any machine);
  the most specific one set wins. And, kept on each device, its Appearance: System (the device's own light
  or dark), Light or Dark, set before the first paint, with the browser's bar and artifacts following it. A phone
  lists the sections in groups, as iOS's Settings does, and opens one over the list; a wider screen keeps the list
  down its side.
- On a phone or tablet it behaves like an app: every screen it opens (a space, a task, the sidebar drawer, settings,
  search, a task's Files or Diff, an artifact) is a step back, so iOS's swipe back and Android's back go one step at a
  time (task, then space, then the sidebar) and an alert's link opens with its space behind it. Swipe from the
  left edge for the sidebar (left again to put them away), right across a task's chat or settings to go back, and down
  on a sheet's grabber to close it; each follows your finger, and a quick flick is enough. A message slid right still
  replies, and held it still reacts. Screens push and pop, sheets slide, menus spring, tasks glide between groups; with
  reduced motion they just fade. No pull to refresh: everything is live.
- On a desktop, or a tablet with a keyboard, everything works from the keyboard: press `?`. Option+↑/↓ (Alt off a
  Mac) goes to the space above or below in the sidebar, as it shows (headings and folded ones skipped), from
  anywhere, the message box too, and leaves you typing in that space's box.
- On a wide screen, drag a sidebar's inner edge to resize it (double-click resets). Hide the sidebar for more chat room
  with the button at its top or `⌘B`. Each device remembers its own.
- Split view, as Slack's: on a desktop the middle holds up to three panes side by side (as many as the window has room
  for), each a space's chat, a task's chat, an artifact or a space's tasks, with its own header and message box. Open
  one with **Open in split view** in a space's or task's ⋯, an artifact card or full screen, or a task's header ⋯, or
  `⌘`-click (Ctrl-click off a Mac) a space, a task or an artifact's title. The focused pane has a thin line along its
  top: the sidebar's selection, the lists on the right and the keys follow it, and a space picked opens in it. Drag
  the line between two panes to resize them (double-click shares the room evenly); each pane's header has **Make it
  the only one** and **Close pane**. Each device remembers its own; a phone or tablet shows one at a time, as always.
- A task's chat has a side panel, like T3 Code's: on a desktop a column right of the chat, opened with the toggle at
  the end of the task's header, resized by its edge, its tabs **Terminal**, **Files**, **Diff** and **Pull request**
  (open or shut, width and tab remembered on the device); on a phone or tablet the same items in the task header's ⋯,
  each a full-screen sheet that Back closes (on a desktop its ⋯ lists them too). Every chat's ⋯, a task's or a
  coordinator's, starts with **Search in this chat**. All but Terminal is read only. Terminal is the agent's live core terminal: click
  it and every key goes in, Esc too; `Ctrl+`` ` opens it and goes back to the chat. Files lists the worktree (tracked
  files and new ones git doesn't ignore), finds one by name and shows it highlighted. Diff shows what the task changed
  against where it left main, committed or not: its files as a tree with lines added and taken out, and each file's
  unified diff. Pull request shows the task's PR on GitHub (state, checks, a link), asked of `gh` on the remote's own
  host; greyed out until one is open. The server reads only that worktree: a file is read only when git lists it
  there and it really lies inside it (no `..`, no links out, nothing ignored, never `.git`). Coordinator chats have no
  panel; their terminal flips the chat, from the ⋯ menu or `Ctrl+`` `.

Every agent learns the chat lines, cards, artifacts and the `shepherdr` command from
[skill/shepherdr](skill/shepherdr/SKILL.md); coordinators their tasks, news, merges and Autopilot from
[skill/shepherdr-coordinator](skill/shepherdr-coordinator/SKILL.md); task agents the way from brief to merged from
[skill/shepherdr-agent](skill/shepherdr-agent/SKILL.md). Their prompts say only who they are,
their space (and a home space's machine duties) or task, and which skills to read. `shepherdr help` lists the command.

## Plugins

Everything is a plugin: places, coordinators, tasks, chat, reactions, artifacts, terminal, worktree, browser, voice,
health, alerts, keyboard, autopilot are the base ones, in `plugins/<name>/` and named in [plugins.json](plugins.json).
A task's side panel takes its items from plugins too (`taskPanel`: Terminal from terminal; Files, Diff and Pull request
from worktree; Browser from browser). A plugin has a `client.tsx` (screen parts it adds to slots) and/or a `server.ts`.

### Your own plugins

Your own plugins live outside the repo, on the main machine, in `~/.shepherdr/plugins/<name>/` (under
`SHEPHERDR_HOME`), the same shape as a base plugin. Updates never touch that folder, and every release takes them in:
save a file there and Shepherdr rebuilds its plugins within seconds, with the app and agents still running; open
screens reload themselves. A plugin from 2.0 needs checking against [docs/plugins.md](docs/plugins.md) first: nothing of
it runs without that manifest.

- **Add one**: a new folder with a `shepherdr.json` holding `{ "api": 2 }` and a `client.tsx` (and a `server.ts` if you like). Its name: lowercase letters, digits
  and dashes, the same as `name` in its `definePlugin`.
- **Change a built-in one**: copy `plugins/<name>/` to `~/.shepherdr/plugins/<name>/`, add its `shepherdr.json` and edit it. Yours replaces the
  built-in one of that name, in its place; the server's own behaviour for that name stays.
- **Turn any off**: Settings → Plugins has a switch for each, built in or yours. They go in `~/.shepherdr/plugins.json`
  (`{ "off": ["voice"] }`), never in the repo's list, so a merge never undoes them.
- **When yours fails**: one that doesn't build is set aside and the rest updates as usual: its last good version runs
  if it had one, else it is left out (a built-in one it replaced comes back). One that fails as the server loads it gets the
  same; one that throws on the screen stops only its own parts. A line at the top of the screen says so, and
  Settings → Plugins shows the error. Fix the file and it comes back with the next rebuild.

A hello world, `~/.shepherdr/plugins/hello/client.tsx` beside its `shepherdr.json`:

```tsx
import { definePlugin, useSnap } from '@client/kernel.ts'

function Hello() {
  const snap = useSnap()
  return <div className="bg-bubble px-4 py-1.5 text-center text-[0.8125rem]">Hello: {snap?.tasks.length ?? 0} tasks</div>
}

export default definePlugin({
  name: 'hello',
  setup(reg) {
    reg.topBars.push(Hello)
  },
})
```

and, if it has a server part, `~/.shepherdr/plugins/hello/server.ts`:

```ts
import { definePlugin } from '../../server/plugin-api.ts'

export default definePlugin({ api: 2, name: 'hello', about: 'Says hello at the top of the screen' })
```

Imports: `@client/…` (the screen's kernel and pieces, `src/`), `@shared/…` (types), `@plugins/<name>/…` (another
plugin's parts: yours where you replaced it, so every plugin gets the same one), and the packages Shepherdr has
(React, clsx, Heroicons). A plugin folder sits where a base one does, so a base plugin's relative imports
(`../../server/plugin-api.ts`) work in a copy too.

**The contract.** `definePlugin` (the screen's in `src/kernel.ts`, the server's in `server/plugin-api.ts`) and the
slots below are public: plugins outside the repo rely on them, so they change only with care.

- `setup(reg)` gets the registry of slots (`Registry` in `src/kernel.ts`, each one documented there):
  - views: `reg.views.places`, `placesMenu` (the sidebar header's buttons), `chat`, `tasks`, `artifacts`, `artifact` (one
    in a split view's pane), `tasksMenu`, `settings`, `search` (one each; the last plugin to set one wins)
  - chats: `messageParts`, `messageEdge` (under and on a message's bubble), `chatTail` (after the messages),
    `chatBody` (replaces a chat's body while its `when` holds)
  - headers and menus: `placeHeader`, `placeMark`, `placeMenu` (a coordinator's), `agentHeader`, `agentMenu` (a task's)
  - the composer: `composerAbove`, `composerLive`, `composerSettings`, `composerIdle`, `composerPlus`, `composerMic`
  - the screen: `taskPanel` (side panel items), `overlays`, `topBars`, `tasksFooter`, `keys` (keyboard handlers)
  - the lists show what their chips hold, `listView(ui, snap)`: `tasks`, `artifacts` and `tasksMenu` still get the
    place picked (`placeId`, `place`), and `tasksFooter` also gets the `list` shown
  - Settings: `settingsPages` (a page under This device), `settingActions`
  - items with an `order` are sorted by it; base plugins space theirs out (10, 20, …), so yours can fit between.
- Hooks and actions from `@client/kernel.ts`: `useSnap()` (the server's data: places, tasks, artifacts, settings),
  `useUI()` and `setUI()` (what is on screen), `useStore()`, `getState()`, `act()` (send an action to the server),
  `has(name)` (is a plugin on), and helpers such as `openTask`, `goPlace`, `placeOf`, `taskOf`.
- The server part: `definePlugin({ api: 2, name, about, setup? })`; it runs when the server starts, and `setup(context)`
  gets state of its own and actions its screen part can call ([docs/plugins.md](docs/plugins.md)).

## Develop

```sh
bun install
bun run demo          # the server with fictional data (the prototype's world), on :7787
bun run dev           # the screen with hot reload, on :5173, talking to the server above
bun run typecheck     # types, in seconds: all a task runs here before it merges
bun run test changed  # by hand only: the tests that reach what you changed from main
bun run test          # by hand only: every test, in parallel (about 30 s)
bun run check         # by hand only: types, tests, build
```

Agents run no tests on this machine, to keep its CPU for herdr and the agents: a task merges once `bun run typecheck`
passes. GitHub runs the whole check on every push to main (`.github/workflows/check.yml`), not on pull requests, to
save Actions minutes, and a red main reaches the coordinator, who has it fixed forward. Tests run by hand run at the
lowest priority (nice 19, idle disk, a systemd scope with the least CPU weight).

Agents working on Shepherdr: [AGENTS.md](AGENTS.md) says what they check before merging.

The demo source (`SHEPHERDR_SOURCE=demo`) holds a fictional world for reviewing every screen and case without real
agents. Your data lives in `~/.shepherdr` (`SHEPHERDR_HOME` to move it).

```text
server/            the web server and the demo source
  core/            core client, product state, tasks, standalone lifecycle, machines, voice, browser
  common/          shared log readers (readers/), settings, triggers, vault
src/               the screen's kernel, layout and shared pieces
plugins/<name>/    each base plugin's parts (yours: ~/.shepherdr/plugins/<name>/)
skill/             what every agent is taught (shepherdr/), coordinators (shepherdr-coordinator/), task agents (shepherdr-agent/)
helper/            what runs through core on each machine: its shepherdr command, hook recorder, health, browser relay
bin/shepherdr      start, stop and status, and the command agents use
bin/agent-browser  agent-browser at the pinned version, for each task's shared browser
```

## FAQ

**What is herdr-core?** The small engine under Shepherdr, with the `hc` command. It holds your agents' terminals, keeps
them running when the app or your screen closes, and links your machines. Shepherdr is its private web app, talking to
it over a local socket that never goes on the network.

**How is 2.1 different from the 2.0 herdr plugin?** 2.0 was a plugin inside terminal herdr: installed with `herdr plugin
install`, updating itself from main, and reaching other machines with SSH workers, helpers and tunnels. 2.1 stands
alone beside herdr-core and needs no herdr app: you install it from a checkout, start it with `shepherdr start` and update
it by hand, and core links the machines. The app itself looks and works the same. It runs Claude Code and Codex (not pi,
for now), its home starts fresh, and your own plugins need a small change.

**I'm on 2.0. What happens to me?** Nothing by itself: 2.0 and its data stay as they are, and 2.1 never imports them. But
main is 2.1 now, so keep 2.0 on the `v2.0` branch, and don't pull main into it or update it from main. When you want
2.1, see Upgrading from 2.0.

**Where does 2.0 live now?** On the `v2.0` branch: the whole plugin, with its own README.
