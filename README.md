# Shepherdr

Run many coding agents across machines from a phone, tablet or desktop. Shepherdr is a herdr plugin: one private web
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
also has a side panel: its live herdr terminal, its files, its diff against main and its pull request.

Shepherdr stands alone: it talks to herdr and needs nothing else of yours. Agents keep running in herdr terminals; any
CLI herdr can start works (chats are read for Claude Code, Codex and pi).

## Install

Needs herdr 0.9 or later, on Linux or macOS. It runs on [Bun](https://bun.sh): with none found, it gets one pinned,
checksum-checked Bun into `~/.shepherdr/bun/` by itself. Tasks' shared browser also needs Chrome for agent-browser,
once on the main machine: `bin/agent-browser install` (on Linux, `install --with-deps` adds what Chrome needs). Other
machines need nothing installed by hand (More machines, below).

```sh
herdr plugin install eliasstravik/shepherdr     # or link a checkout: herdr plugin link /path/to/shepherdr
```

Its plugin id is `herdr-projects`, the plugin it replaced (below), so herdr lists it as `herdr-projects (Shepherdr)`
and its actions are `herdr-projects.<action>`. herdr builds it (`bun install`, `bun run build`) and starts it at every
herdr start; the action **Open Shepherdr** starts it too and shows its link. Then run the plugin action
**Share Shepherdr on my tailnet**: it serves the app privately on your tailnet (`tailscale serve`, never public) and
prints the link, `https://<machine>.<tailnet>.ts.net/shepherdr/`. On an iPhone, open it in Safari and add it to the
home screen: alerts need that.

### Upgrading from herdr-projects

Shepherdr 2.0 is the next version of herdr-projects: `herdr plugin update` takes you there.

1. Finish or stop the tasks in flight first: their agents keep running, but nothing follows them any more.
2. `herdr plugin update` needs herdr 0.9.4 or later: run `herdr update` first on an older one.
3. Run `herdr plugin update herdr-projects`, then press your old herdr-projects key (or run the action **Open
   Shepherdr**, or restart herdr). Shepherdr starts and shows its link.

On its first start Shepherdr brings nothing over from herdr-projects: `~/.herdr-projects` stays exactly as it was. It
copies what it is about to change into `~/.herdr-projects-backup-<date>/` (with a README.txt), then stops herdr-projects'
keeper, page server and open board, takes its agent skill out of `~/.claude/skills`, `~/.codex/skills` and
`~/.pi/agent/skills`, takes its sidebar rows out of herdr's `config.toml` (its key stays and opens Shepherdr), and
points a tailnet link its page server took over back where it was. It says so once in the home space's chat.

Later updates are `herdr plugin update` too: the next herdr start, or the next press of the open key, swaps the running
Shepherdr for the new version.

### Going back to herdr-projects

herdr-projects 1.2.27 stays on the branch `legacy-herdr-projects` and the tag `v1.2.27`, with its own README. To go back
with your old tasks and settings as they were:

```sh
herdr plugin action invoke herdr-projects.stop                     # stop Shepherdr
herdr plugin install eliasstravik/shepherdr --ref v1.2.27 --yes    # herdr-projects 1.2.27 in its place
cp ~/.herdr-projects-backup-<date>/config.toml ~/.config/herdr/config.toml   # its sidebar rows back
herdr server reload-config
rm ~/.shepherdr/took-over-herdr-projects.json                      # a later upgrade tidies up again
```

If you changed herdr's config since the upgrade, paste the `# >>> herdr-projects sidebar` block from the backup's
`config.toml` back in instead of copying the whole file. Then press the herdr-projects key: the board opens, and its
keeper and agent skill come back by themselves. Shepherdr's own data stays in `~/.shepherdr` for a later try; to
return to Shepherdr, `herdr plugin install eliasstravik/shepherdr --yes`.

### Install it as an app, with alerts

Install the link as an app on each device, open it, and turn alerts on in Settings → Alerts on → **Turn on for this
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

Shepherdr keeps itself up to date. When its own checkout moves to a new commit (a task merged into main), it makes a
release of that commit in `.releases/`: a frozen copy, with `bun install` run only when the lockfile changed, built and
checked there. Once no voice call is on, it stops new calls and restarts on the release. The server always runs from a
release, never from the checkout, so a later merge can't change what runs; a release that stops as it starts is taken
back. Every open screen then reloads itself to the new version: what you were typing and the files you attached come
back, and it waits until you stop typing, finish a voice call or stop talking to type. A screen reloads only when the
screen itself changed. If an update fails, Shepherdr keeps running the last version, says so at the top of every screen
(tap Why for the error) and tries again on the next commit. It follows the checkout herdr links, so a merge on GitHub
arrives once that checkout pulls it; `shepherdr merge` pulls main there itself after merging a pull request. A change to
the updater itself (`scripts/supervise.ts`, `supervisor.ts`, `update.ts`) takes effect at the next herdr start.

By hand: `bun run build && bun server/main.ts` serves on `http://127.0.0.1:7787` (`SHEPHERDR_PORT` to change it).

### More machines

Shepherdr is installed once, on the main machine. Every other machine you save in herdr's Remote shows up on its own
(`herdr machine add <ssh target>`, which also sets up herdr there): its spaces (its home space at least), tasks and
chats, with their coordinators, run from the main machine. The other machine needs only herdr, git, perl and the agent CLIs (gh for
pull requests of GitHub repos); nothing to install by hand.

- How: the main machine runs each one from a worker process of its own (`~/.shepherdr/machines/<name>/`). Agents,
  panes, worktrees and live events go through herdr's own remote bridge; git, the Files and Diff panels and merges
  run over SSH (one shared connection); the agents' logs and Claude Code's hook events are copied over as they grow.
- The helper: Shepherdr puts a small folder there over SSH, `~/.shepherdr/helper/`: the `shepherdr` command for its
  agents (it reaches Shepherdr through an SSH tunnel from the main machine, so the API stays off the network), the
  hook recorder, the log copier, a health reader, the skills, and the pinned agent-browser with Chrome. A new machine
  gets it as soon as herdr has it saved; it is put there again whenever Shepherdr updates and its copy changed.
- `shepherdr doctor` (on the main machine) checks every machine, puts the helper there again, starts its worker again
  and says what is missing. Settings → Machines says how each one is: Ready, Putting the helper there…, Can't reach it
  (and why), or what it needs.
- A task's shared browser runs on the task's own machine, next to its app. Where that machine can run `tailscale
  serve`, your screen takes the browser's frames and sends your input straight to it over the tailnet (the main
  machine only hands out the link, under a path no one can guess); elsewhere they come through the main machine.
- A machine that can't be reached says `Can't reach` on its heading; its chats keep what they last showed and what you
  send waits until it is back.
- From before: a machine that had its own copy of Shepherdr reporting to this one gives its spaces, tasks and chats
  to the main machine the first time it is reached, and that copy is turned off in its herdr (`herdr plugin disable
  shepherdr`; its files stay). `config.json`'s `"main"` and `peers.json` are no longer used.

### Voice

Calls go through an ElevenLabs voice agent (Shepherdr makes one for you, with no recording and no retention). Give it
your key once, on the main machine:

```sh
echo "<your ElevenLabs API key>" | shepherdr secret elevenlabs
```

## Use

- Pick a space; write to its coordinator. Ask it for a task; tasks are created only through a coordinator.
- What you send shows in the chat the moment you tap Send, faded until it is in. If it can't go through, it says so
  under the message, with Retry and Remove; it is still there after a reload.
- Write to a busy agent any time: it goes in at once and steers the agent mid-turn (Claude Code, Codex and pi take it
  at their next step). It waits only while the agent shows a menu or permission prompt, where typed text could pick an
  option (the bubble says so, and it goes in once the prompt is answered), or while the agent is still starting.
  Commands like /compact or a model change wait for the turn to end.
- Startup trust and MCP approval prompts appear as choice cards, even before the first message. The card keeps the
  CLI's screen text and offers Open terminal if it cannot read the choices. Answers are checked before messages go
  in. Only task worktrees Shepherdr creates are pre-trusted in Claude Code and Codex; a space you picked still asks.
- A slash command you type (`/clear`, `/new`, `/compact`, `/model sonnet`, a custom one) goes into the agent's CLI as
  you typed it, once its turn is over, and the CLI runs it. The chat shows it as you typed it, and what the CLI printed
  as its answer. After `/clear` or `/new` a quiet New session line starts the new session, whose log the chat reads
  from then on. A menu or view the command opens in the terminal (`/model` alone, `/usage`) comes into the chat and is
  closed, so your next message goes in; one that asks a question (Codex's `/new`: where should the new conversation
  run?) is a card to answer, and what you write meanwhile waits for it. A command the CLI didn't run fails with Retry.
- Every machine has at least one space: its **home space**, at its home folder (~), named `home`. It can be renamed
  but not removed; it comes first under its machine until dragged, and moves like any space. Its coordinator also looks after the machine: health, load and
  disk news, its processes, adding spaces. The main machine's home space also sees every machine: what needs you
  across them all, which alerts reach your phone, and alerts about a whole machine. A space's tasks list holds its
  own tasks, a home space's too; a machine's chip lists its spaces', grouped by status or by space, and chips on more
  machines (or none: everything) also by machine, or machine › space. A machine's heading has a + (New space) and a ⋯ (New space…, New section…,
  Rename machine…: its emoji and name), and says when it is in trouble: `Can't reach`, `herdr down`, or a limit it is over.
- Add a space with the + on a machine's heading (it shows on hover with a mouse), its ⋯ (New space…), `N` on a desktop
  (on the machine you are in), or by asking a coordinator (`add ~/dev/x as a space`, or `shepherdr space add <path>
  [--name NAME]`). Pick the machine when there is more than one, then a folder in your home there: the picker starts at
  ~ (~ itself is refused: it is the home space). Type the path to narrow the list (Tab goes into a folder, Enter
  picks); your latest repos are listed first. Then its name, filled in from the folder's. Names follow Slack's channel
  rules: lowercase letters, numbers, hyphens and underscores, at most 80 characters, each its own on its machine;
  typing lowercases and turns spaces into `-` (`My Project` becomes `my-project`), and a taken or bad name is refused
  in the box. The name is the space's slug in links (`?place=vm-str/gtm`) and in the CLI; its tasks' id prefix (SHE,
  TG…) is its own. A repo is added as it is; any other folder, or one not made yet, becomes a new repo first (git init
  on main with an empty first commit; files already there stay uncommitted). Either way it gets its own herdr space
  and coordinator, and opens. Only folders in your home are listed or made.
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
  keeps it (`names.json`). A space's new name follows the same rules as a new one's (`shepherdr space rename <name>
  <new-name>`). Its folder, repo, herdr space, task ids and the machine's SSH and herdr name stay as they were.
- Remove a space from its row's menu (a held finger on a phone or tablet; its ⋯ or a right click on a
  desktop, where it opens beside the row): Remove asks first. Its folder and code stay on disk; Shepherdr forgets it and closes its
  coordinator. Its open tasks hide with it (their worktrees and branches stay) and come back, with its chat, when it is
  added again under the same name. Not while one of its tasks is working: pause it or let it finish first. Never a
  home space, so a machine always keeps one. A coordinator can do the same (`shepherdr space remove <name>`).
- Machines, and each machine's spaces (its home space first), show A to Z. Drag one to reorder them (with a mouse at
  once; a finger holds the row, its menu opens, and sliding it closes the menu and moves the row); a machine takes its
  spaces and sections along, a section its spaces, and Option+Shift+↑/↓ (Alt+Shift off a Mac) moves a focused space. The main
  machine keeps the order (`order.json`), so every device sees it; a space added later slots in A to Z among the
  rest.
- A space's ⋯ pins it (Pin / Unpin), as Slack stars a channel: it moves out of the tree into Pinned, a heading at the
  top of the sidebar that folds like the others (each pin with its machine after its name), and Unpin puts it back.
  Only spaces pin. Pins drag into order and Option+↑/↓ walks them first; the main machine keeps them with the order,
  so every device has them.
- Sections, as Slack's, only organize spaces (`sections.json`, what is in them with the order). The sidebar header's +
  makes one at the top, beside Pinned, for any machine's spaces (each shows its machine after its name); a machine's ⋯
  (New section…) one inside it, one small step in, for that machine's only. A space's ⋯ › Move to section moves it
  into one, or back under its machine; a drag onto a section's heading or among its spaces does the same. A section
  folds, drags among its peers, and its ⋯ renames it (emoji and name) or deletes it: its spaces go back where they
  were. Pinned is one of them that can't be deleted.
- From before spaces (projects, machine coordinators and an All machines coordinator): the first start moves it all
  over once (`server/live/migrate.ts`). Projects become spaces with their ids, names and chats (`projects.json` becomes
  `spaces.json`); each machine coordinator's chat moves into that machine's home space, and All machines' into the main
  machine's home space, which takes over its jobs; modes, settings, pins, unread marks, heartbeats and artifacts
  follow. A pinned machine becomes its pinned home space. `shepherdr project …` and `--project` still work, as
  `space`.
- A task is one agent, started at once in its own git worktree and branch from main (`work/<n>-<slug>`). herdr makes
  the worktree (`herdr worktree create`) from the space's own herdr space (its coordinator's), so the task's herdr
  space nests under it in herdr's sidebar. With no herdr space of the repo open, herdr opens one for it, and Shepherdr closes
  it again with the last task if you never used it. Your own checkout is never touched. A fresh worktree gets a copy
  of the ignored files the repo's `.worktreeinclude` names (`.env` and the like); nothing else runs. A task has no
  settings: its brief is all the agent goes by. The coordinator writes the work and your wishes for it ("wait until I
  say go", "skip the review", "ask me before merging"), and Shepherdr ends every brief with its Settings block,
  one plain line per setting that shapes how the agent works, the source it follows: whether it is reviewed and by whom (the reviewer tags each finding critical, major, minor or trivial; the agent fixes
  those at or above Must fix, `review.mustFix`, major by default, and reviews again, at most Review rounds,
  `review.rounds`, 3 by default), and whether it merges on its own (by default once its own quick checks pass,
  without waiting for GitHub's checks), on its own after GitHub's checks, or asks you first, and the agent, model
  and effort it runs on (only so it knows). A line that differs from Shepherdr's default says who changed it: this
  space's setting, this machine's, yours for every machine, or set by the coordinator for this task (`task add --set
  review.rounds=1 --set merge="ask me first"`). The brief's own words win, so a wish for one task needs no setting
  changed. The task's chat shows the block at the end of its first message, the changed lines picked out. The rest is the agent's judgment, by its
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
  the same. Once started, only the agent puts a task in To do; one that stops for any other reason needs you, except
  while work it started in the background (a reviewer subagent, a shell, a monitor) still runs and will wake it: that
  is still Working (shells alone for up to 10 minutes, since a server it started for you never wakes it; a shell
  running a headless reviewer of another CLI, `codex exec` or `claude -p`, counts as a subagent), unless its last
  words ask you something. A task that needs you says why in at most 40
  characters, never by pasting the agent's message and never in a model's words: a standard reason (Plan to approve,
  Asks permission, Signed out, Question for you, Waits for your reply), or the agent's own question cut to fit
  (from its question tool, its choice card, or the last sentence of its last message that asks). The tasks list, the
  card in its space's chat, alerts and coordinator news all say the same; the full message stays in the task's chat.
- A turn cut off by the model's provider (overloaded, at capacity, a rate limit, a dropped connection) is no question
  for you: Shepherdr waits and tells the agent, task or coordinator, to carry on, after 1, 2, 4 and 8 minutes. Meanwhile
  a task stays Working, its line saying why and when (`Provider overloaded · retry in 2m`). Only when the fifth try is
  cut off too does it need you (`Provider overloaded, still after 5 tries`); a coordinator says so in its chat, with an
  alert. Errors waiting can't fix (signed out, out of allowance, a model that isn't there) go to you at once. Anything
  written to the agent starts the count again. Each CLI's log says what cut it off (`server/live/retry.ts` decides).
- When a task's agent can't go on (out of allowance, signed out, its provider still down after the tries), its
  **backup agent** takes over, once: Settings → Agents → Tasks → Backup (`task.fallback`, Codex unless set, or off).
  It starts in the same worktree and branch, on its CLI's own model, with the brief and a short handover: who it takes
  over from and why, the branch's commits, what lies uncommitted and the chat so far. A card in the task's chat says
  so. If the backup can't go on either, or isn't installed, the task needs you.
- Status, coordinators' too, is what the agent reports of itself. Every Claude Code Shepherdr starts gets a settings
  file of its own (`--settings`, in `~/.shepherdr/hooks/`; your own settings are never touched) whose hooks record
  each event: a turn starts or ends, what still runs in the background as it ends, a permission prompt (a subagent's
  too), its question tool, a plan. A turn cut short (Esc, a prompt turned down) runs no hook; its transcript says it
  ended. herdr's reading of the screen counts only where the agent reports nothing: Codex (its hooks each need
  trusting in a dialog first, and it writes OSC 7501 itself, which herdr will pass on), pi (it tells only a turn's
  end), an agent started before this, or a menu up between turns that no hook tells of. One function (`statusFrom` in
  `server/live/agents.ts`) decides, so taking herdr's OSC 7501 program status once it passes that on is one change.
- On a repo with a GitHub remote that `gh` is logged in to, each task gets a pull request. The shepherdr-agent skill runs
  `shepherdr pr <id>`: it pushes the branch and opens a draft (its title the task's, its words the agent's summary, the
  brief's gist and a "Generated with" footer), and pushes to it again after new commits. `shepherdr checks <id>` waits
  for GitHub's checks (merging waits for them only when Settings → How tasks work → Merge is "after GitHub checks", or the task's --set says so);
  failed ones go back to the agent to fix. Reporting it ready takes the pull request out of draft.
  While it waits, Shepherdr looks at it every two minutes and passes news on to the resting agent once, in
  `<shepherdr>` tags: a check that failed (its link and the end of its log) and new reviews and comments with words (a
  bare approval is not news). The agent decides what to do (shepherdr-agent says how); Shepherdr only passes it on.
  Its number shows on the task's row, links from its chat's header and from the Done card. The remote is read live, as
  written and as git rewrites it (`url.insteadOf`), and `gh` is told the host (`GH_HOST`, such as `github.localhost`).
  No remote, or no `gh` login: tasks merge here, as below. Logged in but GitHub not answering: nothing merges until it
  does.
- The agent merges when its brief or you say so: `shepherdr merge <id>` squash-merges the branch into main.
  On GitHub that goes through its pull request (opened then if the agent never did), as soon as the agent's own quick
  checks pass (in Shepherdr: `bun run typecheck` only), without waiting for GitHub's checks: they still run on
  every push to main (and on the pull request where a repo's checks do), and only a failure GitHub already shows at that commit stops the merge.
  With Settings → How tasks work → Merge on "after GitHub checks", it merges only once GitHub's checks pass. Either way only
  at the commit this machine has (`gh pr merge --squash --match-head-commit`), then its branch on GitHub is deleted
  and main here is pulled, so Shepherdr's own update sees the merge. Failed checks are sent back to the agent. A branch
  behind main is merged as it is: rebasing it would make a new commit and run all of GitHub's checks again, and again
  each time another task lands meanwhile. Only a conflict with main is rebased (by `shepherdr merge` when git can, else
  by the agent), and when waiting for GitHub's checks they run on that commit before it merges. For a few hours after
  a task merges through its pull request, Shepherdr watches GitHub's runs on main: once one fails, the space's
  coordinator hears it once (news), with the failing run, the end of its log and the tasks merged since main was last
  green, and decides on a fix-forward task (Autopilot adds one by itself). It hears again only after main was green. GitHub's
  merge queue would test each one on top of the others, but it needs an organisation's repo. Elsewhere it is rebased
  and squash-merged into main here (and main pushed when there is a remote). Then the task is marked merged. Once the
  agent's turn ends, the task is put away through herdr: the worktree removed (which closes its space and agent), then
  the branch deleted. Nothing is forced: a checkout with uncommitted changes, or a branch with work main lacks, is kept
  and said so (in the coordinator's news and in `shepherdr status`), and its space and agent close all the same.
  Something still running in its space puts it all off until later. You can also just ask its coordinator to merge it.
  Task agents learn the way from brief to merged from [shepherdr-agent](skill/shepherdr-agent/SKILL.md): the review the
  brief's Settings block asks for (Settings → How tasks work → Review, Reviewer, Must fix and Review rounds: only words
  in the brief; the agent runs it however it sees fit, a subagent or a
  headless CLI), what the change broke, left undone or added beyond the brief; only the choices no experiment can
  settle as cards for you, and your yes before anything irreversible; the proof in one small artifact; ready with a facts line; then merge or ask. How to
  build and check the work is the agent's own judgment: Shepherdr only gives it the brief, your defaults and the house
  rules.
- A task with nothing to merge (an answer, research, or work no longer wanted) is closed: `shepherdr task close <id>
  [--summary "..."]`, or just ask its coordinator. Its agent stops, its space closes, its worktree and branch go when
  they hold nothing unsaved or unmerged (else they are kept, and it says so) and it shows in Done with its summary
  (Nothing to merge by default); its card in the space's chat says so. An agent whose `shepherdr ready` finds no changes on
  its branch goes straight to Done the same way.
- `shepherdr task stop <id>` closes a task's agent and space. Its work stays: a worktree with unsaved or unmerged work
  is kept, one with none goes with its branch. Writing to the task opens its space again (or makes a fresh worktree)
  and resumes its agent.
- `shepherdr task delete <id>` removes a task for good, and only when you plainly ask its coordinator to: its agent
  stops, its space closes (with anything else running there), its worktree, branch, chat log and record go, so it
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
- Leftovers of merged or closed tasks are swept up when Shepherdr starts and every hour after (a merge updates main,
  and the restart that follows can cut a clean-up short). Only tasks Shepherdr made and finished are looked at, never
  one whose agent is at work or that you talk to again, never your own worktrees or branches, and nothing while herdr
  does not answer. A branch counts as merged when main holds its exact tree (what the squash merge made), when
  squash-merging it again would make its task's own squash commit (so nothing came after the merge, however main
  moved on since), or when merging it into main would change nothing.
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
  (its ⋯ menu); each keeps its last 20 versions.
- Every coordinator is an agent that does what you ask, standing requests too ("merge tasks as soon as they pass"):
  it wakes on news and acts on it when you asked it to handle such things, or tells you what matters. **Autopilot**
  is a switch at the top of its chat's ⋯ menu: on top of that it also works on its own, within what you asked for.
  What you switch stays, per space. A space's news goes to its
  coordinator, and to its machine's home space while that one is on Autopilot. Every space's news on every machine
  (needs you, ready, stalled, merged, main red, machine health) also reaches the main machine's home space, in one line
  each: it keeps the overview and picks what buzzes your phone; the space's own coordinator acts on it. While Autopilot is on, a small red pill
  at the chat's top right says so (on a phone after the subtitle, under the name), a tap on it opens the ⋯ menu, and
  the space's row in the sidebar has a small red tag (with a mouse, its dot alone); with it off neither shows. Alerts buzz only while the app is closed or in the
  background; with it on screen they show nothing.
- Autopilot answers task agents' questions it can answer from the brief or the space, has conflicts rebased and
  failed checks fixed, starts queued tasks and closes tasks whose work is over, adds small follow-up fix tasks for
  what reviews, proofs or a red main found, and switches itself off and tells you why when things keep breaking. It
  never deletes tasks, changes Settings, starts work nobody asked for, force-pushes, or merges past a brief's "ask me
  first". It has no limits of its own: the machine's max tasks at once already bounds it. New tasks a day, retries,
  how long a task may take and when to hand back are its own judgment: Shepherdr gives it the facts (news, the
  agents' own progress) and trusts the model with the rest. A limit you want goes in its HEARTBEAT.md in plain words
  ("at most 5 new tasks a day"), and the coordinator keeps it.
- Triggers: Autopilot's heartbeat and automations run on the same kind of trigger, a time rule, event rules, or both
  (`server/live/trigger.ts`). The time rule is an iCalendar recurrence rule (RFC 5545: FREQ, INTERVAL, BYDAY, BYHOUR,
  BYMINUTE, BYSETPOS, UNTIL or COUNT, EXDATE, DTSTART; read by the `rrule` package), or `once 2026-11-02 9:00`: every
  90 minutes, every other week, the last Friday of the month, start and end dates and skip dates. Event rules are news:
  a task added, ready, gone wrong, needing you, merged or stalled, main red, a machine struggling (heartbeat only), each
  with optional `space=`, `branch=` (a glob) and `title=` filters. Every time rule's times are in Settings → Time zone
  (`time.zone`, Everywhere; this machine's own zone by default). You never write a rule: ask a coordinator in your words
  ("every 30 minutes, 8 to 22 on weekdays") and it writes one (`--when '<rrule>'`, `--on '<events>'`); everywhere it
  shows in words, with its next three runs ("Every 30 minutes, 8:00-22:00, Monday to Friday (Stockholm)").
- Autopilot is a heartbeat, as OpenClaw's. Settings → Autopilot holds just three things, per space (or a machine, or
  Everywhere): the Heartbeat, a time rule (every 30 minutes, all day), shown in words with its next runs and a few
  presets (every 15, 30 or 60 minutes; workdays 8-18; anything else, ask the coordinator); Wakes at once on, its event
  rules (all news, or none); and the Checklist, its HEARTBEAT.md, which starts with a checklist for its role (a space,
  a home space, the main machine's home space). On each run of the time rule, while its coordinator is idle,
  Shepherdr sends it one fixed prompt in its own session: go through its HEARTBEAT.md (its standing checklist in plain
  words, in the prompt itself, so a coordinator on another machine has it too; edited in Settings → Autopilot or by
  the agent with `shepherdr heartbeat set`; kept in `~/.shepherdr/heartbeat` on the main machine) and act on what needs
  it, or reply only `<no-reply>`, which the chat never shows. A busy coordinator gets it once its turn is over; none
  while its HEARTBEAT.md is empty and no news waits. News its event rules name wakes the coordinator at once, so a news
  card gets its reply right after it; other news waits and comes with the next heartbeat (quiet nights: a time rule
  within the day's hours, and only what can't wait as its events).
- Automations are cron jobs for tasks: a space's task template (title, brief, agent) and its trigger: a time rule,
  events of its own space (or the space a rule names), or both. Each run adds a new task with a fresh agent; none runs
  in the coordinator's chat. Only agents add or change them (`shepherdr automation`), when you ask a coordinator in the
  chat. They list at the bottom of the space's tasks, paused ones too, each with its trigger in words, its next runs
  and its last runs: run it now, pause or remove it there. Automations from before triggers (`daily 9:00`, cron,
  `--when merged`) became time and event rules once.
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
  [agent-browser](https://github.com/vercel-labs/agent-browser) session per task (named after it, in the `shepherdr`
  namespace, with a profile in `~/.shepherdr/browser/<task>` that keeps its sign-ins), started when either side first
  uses it. agent-browser streams it on 127.0.0.1 only; the server relays that stream to the app, so the browser is as
  private as the app itself. A task that is Ready keeps it, to review in; stopping or archiving the task closes it
  (until you write to the task again), and merging, closing or deleting it also forgets its sign-ins. One left with no
  agent command and no one watching it in the panel closes too, sign-ins kept: Settings → Browser → Close idle browser
  after (`browser.idleClose`: 10 min, or off; per machine or space). The agent's next command or opening the panel
  starts it again. Its pages see `prefers-reduced-motion: reduce` (Chrome draws them in software, so an animation
  costs a core); an agent whose page must move turns that off with `set media no-preference`. agent-browser is
  pinned in package.json (`bin/agent-browser` runs that version, for agents too: they are told its full path, since a
  shell's start-up files can put another version first on the PATH, and two versions on one session keep restarting
  its daemon); it needs Chrome on the main machine once (`bin/agent-browser install`); other machines get it with
  their helper. Without it the item is greyed, with a
  hint. A browser that won't start says why over the panel, with Try again.
- Each machine watches its own health: every 30 seconds it reads its 1-minute load against its cores, memory in use,
  how full home and /tmp are, a GPU's use (nvidia-smi, where there is one) and whether herdr answers. A small meter at
  the top right of its home space's chat shows it (thin bars, the numbers in words where there is room; grey while all is
  well, amber while new tasks wait, red past an alert limit), and a tap shows the numbers, the tasks at work against
  the limit, herdr, and its settings. Settings → Each machine → its name holds its limits, each machine keeping its
  own, scaled to its cores and memory: limit working tasks by Number of tasks, Machine load or Both (Both); max tasks
  working at once (10); new tasks wait while the load is over 1× the cores or memory over 80%; its coordinator is told
  when the load stays over 1.5× the cores for 2 minutes, memory or a disk is over 90%, or herdr stops answering.
  - **Room for tasks** (Shepherdr enforces it): a task added, or told to go (a queued task's go, or a coordinator's
    `task tell` to one that waits), waits as Queued, `Waiting for a free slot` or `Waiting for the machine`, with no
    agent started yet, and starts by itself once there's room, oldest first; while load counts, one at a time, 30
    seconds apart. Only working tasks count. Your own message to a task starts it at once.
  - **Alerts** (the coordinator decides): past a limit, a card shows in the machine's home space and its coordinator (and
    the main machine's home space's) gets the numbers and what uses the machine most, by task; once more when it
    is back under. The shepherdr-coordinator skill says what to weigh: pause or queue tasks, clean the disk, or tell
    you. Shepherdr never pauses a running task itself.
  - **herdr gone**: its socket refusing is said plainly ("herdr isn't answering"), never `connect ECONNREFUSED …`; the
    machine's heading says `herdr down` (as it says a limit it is over), its chats say messages wait until it is
    back, and its home space's coordinator hears what happened once herdr answers again.
- Settings (sidebar, or `,` on a desktop) holds the few app settings, in two groups. What agents are told: How tasks
  work, only words in each new brief's Settings block (review, the Reviewer a brief suggests, Must fix, Review rounds,
  merge), each row showing the exact line it adds. How Shepherdr runs: the agent, model and effort tasks and
  coordinators start with (model:
  an id or alias, or default for the CLI's own; effort: default, lowest, low, medium, high, xhigh or max, each CLI
  taking it its own way) and tasks' backup agent, when Done tasks archive themselves, Autopilot's heartbeat (its trigger) and checklist, the time zone of every schedule, each machine's limits, the idle browser, Notify, alerts and voice. Its
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
  coordinator's, starts with **Search in this chat**. All of it is read only. Terminal is the agent's live herdr terminal: click
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
[skill/shepherdr-agent](skill/shepherdr-agent/SKILL.md). Their prompts (`server/live/prompts.ts`) say only who they are,
their space (and a home space's machine duties) or task, and which skills to read. `shepherdr help` lists the command.

## Plugins

Everything is a plugin: places, coordinators, tasks, chat, reactions, artifacts, terminal, worktree, browser, voice,
health, alerts, keyboard, autopilot are the base ones, in `plugins/<name>/` and named in [plugins.json](plugins.json).
A task's side panel takes its items from plugins too (`taskPanel`: Terminal from terminal; Files, Diff and Pull request
from worktree; Browser from browser). A plugin has a `client.tsx` (screen parts it adds to slots) and/or a `server.ts`.

### Your own plugins

Your own plugins live outside the repo, on the main machine, in `~/.shepherdr/plugins/<name>/` (under
`SHEPHERDR_HOME`), the same shape as a base plugin. Updates never touch that folder, and every release takes them in:
save a file there and Shepherdr rebuilds and restarts within seconds; open screens reload themselves.

- **Add one**: a new folder with a `client.tsx` (and a `server.ts` if you like). Its name: lowercase letters, digits
  and dashes, the same as `name` in its `definePlugin`.
- **Change a built-in one**: copy `plugins/<name>/` to `~/.shepherdr/plugins/<name>/` and edit it. Yours replaces the
  built-in one of that name, in its place; the server's own behaviour for that name stays.
- **Turn any off**: Settings → Plugins has a switch for each, built in or yours. They go in `~/.shepherdr/plugins.json`
  (`{ "off": ["voice"] }`), never in the repo's list, so a merge never undoes them.
- **When yours fails**: one that doesn't build is set aside and the rest updates as usual: its last good version runs
  if it had one, else it is left out (a built-in one it replaced comes back). One that throws as the server loads it is
  skipped there; one that throws on the screen stops only its own parts. A line at the top of the screen says so, and
  Settings → Plugins shows the error. Fix the file and it comes back with the next rebuild.

A hello world, `~/.shepherdr/plugins/hello/client.tsx`:

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

export default definePlugin({ name: 'hello', about: 'Says hello at the top of the screen' })
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
- The server part: `definePlugin({ name, about })`; it runs when the server starts.

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
On a machine where herdr runs as a systemd user service, give it (and the server and agents it holds) the CPU first:
`systemctl --user set-property herdr.service CPUWeight=1000`.

Agents working on Shepherdr: [AGENTS.md](AGENTS.md) says what they check before merging.

The demo source (`SHEPHERDR_SOURCE=demo`) holds a fictional world for reviewing every screen and case without real
agents. Your data lives in `~/.shepherdr` (`SHEPHERDR_HOME` to move it).

```text
server/            the web server, the demo source and the live source
  live/            herdr link, chats from logs (readers/), tasks (engine/), settings, machines (box, mirror, install), voice, browser
src/               the screen's kernel, layout and shared pieces
plugins/<name>/    each base plugin's parts (yours: ~/.shepherdr/plugins/<name>/)
skill/             what every agent is taught (shepherdr/), coordinators (shepherdr-coordinator/), task agents (shepherdr-agent/)
helper/            what Shepherdr puts on another machine: its shepherdr command, hook recorder, log copier, health, browser relay
bin/shepherdr      the command agents use
bin/agent-browser  agent-browser at the pinned version, for each task's shared browser
```
