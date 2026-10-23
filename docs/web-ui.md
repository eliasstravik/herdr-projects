# The board in the web UI (Ovis)

**With the web UI [Ovis](https://github.com/devswha/ovis) installed in Herdr beside this plugin, the board is part of
that web UI**: a **Boards** section in its sidebar, one entry per machine, at the same link and behind the same login.
It does what the terminal board does, with buttons and touch instead of keys, on a phone and on a desktop. Without the
web UI, none of this runs.

Ovis was called herdr-web-ui before, and an install from then may still have its old plugin id until Ovis's installer
moves it: the board finds it under either, `devswha.ovis` first, then `devswha.herdr-web-ui`
([its two names](#its-two-names)).

## Setting it up

1. **Install both plugins** (Ovis as [its own page](https://github.com/devswha/ovis) says). From then on a small
   **front door** of the board's own runs on this machine, in front of the web UI. It starts with Herdr and is kept up
   by the keeper.
2. **Share the web UI** as it says itself (its Settings → Phone shows the `tailscale serve` command).
3. **Say yes once.** The next time you open the board in the terminal, it asks: *Show this board in the web UI (Ovis)?*
   **Yes** points the web UI's shared link at the front door. **No** leaves the link alone for good. **Later** asks again
   the next time the board opens.

That one change is the whole setup. By hand, any time:

    herdr-projects web-ui            how things stand: the web UI, the front door, which links go through it
    herdr-projects web-ui link       point the web UI's shared link at the front door
    herdr-projects web-ui unlink     point it back

- `herdr-projects remove-extras` also points the link back and stops the front door ([installing](install.md)).
- To turn the whole thing off and keep the rest: `herdr-projects set board.webUi none` ([settings](settings.md)). The front
  door points the link back by itself, and closes.
- On the machine itself, the board is also at the front door's own address, which `web-ui` prints
  (`http://127.0.0.1:7327/` unless that port was taken).

## The Boards section

In the sidebar, under **Agents** and above its foot (Install app, Settings), the board has a section of its own:
**Boards**.

- Its heading is the web UI's own Agents heading, built again from the web UI's own classes: the same type, spacing and
  caret, and it folds the same way. Like Agents, it starts folded on a phone (the sidebar is a drawer there) and open on a
  wide screen, and a fold stays while the page is open. Folded, the heading still shows the dot of all the boards
  together.
- The section is the board's one place in the sidebar (the single Board entry at its foot is gone). The web UI redraws its
  sidebar whenever it likes (on a phone, each time the drawer opens): the script puts the same section back in its place.

Under it, one entry per machine that has a board, drawn as an agent's entry is:

- **All machines** first, at the top, once the board knows another machine. While this machine is the only one it is
  not there (it would show this machine's tasks again), and a board that showed every machine shows this machine's own;
  it comes back as soon as herdr has a second machine.
- Under it, **this machine** and **each other machine the board knows** (the ones its Other machines goes round: Herdr's
  saved machines, or `board.machines`), in the order of Herdr's sidebar, this machine where that sidebar puts it
  ([machine order](machine-order.md)).
- **Names:** each as the web UI names that machine (`Mac`, `elias-macbook-pro-m1`, `vm-str`), matched to its PCs as below
  ([other machines](#other-machines)). One the web UI has no PC for keeps the board's name for it. A web UI that names
  this machine by no name of its own gives `This machine`.
- **A dot in the board's status colours**, where an agent's status stands: red when a task needs you, green when one is
  ready to approve, the working colour when work goes on (it breathes as the web UI's working dot does, where the web UI
  has that), the dim grey when all is quiet.
- **A short second line**: `1 needs you · 2 working · 3 to do` (`to approve` for a task ready to approve), or
  `all quiet`. Archived and done tasks are not counted.
- **A machine that cannot be reached** says `not reachable`, dimmed like an agent on a PC that is away, with a ring for a
  dot. Pointing at it says why, in the board's words. All machines adds up the ones that answered and says how many did
  not (`· 1 not reachable`). While a machine is asked for the first time, its entry says `asking…`.

**Picking an entry opens that machine's board** over the main area, with its tasks and its coordinators, and the entry
is highlighted as a selected agent is.

- It does what choosing that machine on the board's filter row (or its **Other machines**) does. No line confirms it:
  the filter row's first item names whose board shows, the chat's switch who gets your words. Only where the board moved
  by itself, following the web UI to another PC, does it say so (`Only vm-str. What you type goes to vm-str.`). The two
  stay in step: the board's own pick, and the board following the web UI, move the highlight too.
- On a phone, the drawer closes, as it does when you pick a pane.
- The section asks every four seconds while the page is in view. The other machines are asked for their tasks every twenty
  seconds at most, as the board asks them.

## What the board looks like there

It uses the web UI's own colours, type, sizes and spacing (its CSS variables, `DESIGN.md` there), so it follows light,
dark and every palette of the web UI by itself.

- The web UI does not read Herdr's theme, so there the Board matches Herdr as the web shows it.
- A status is the web UI's own colour for it; a task nobody started is its dim text colour.
- **Colour means status only**, as on the terminal board: names are in the base colour (the top row's and a machine's
  bold, a project's and a task's in normal weight), what goes with them dimmed (keys, counts, times, a title's tags);
  marks, a status group's heading, the lit filter and the selected row keep their colours ([colours](colours.md)).
- The chat is the web UI's own chat, docked under the list: [the web board's chat](web-chat.md). Where the board's own box
  stands in, its frame is in the status colour of whoever gets your words, as on the terminal board: base when idle or
  never started, amber while working, red when it needs you or failed ([colours](colours.md)). That box is the web UI's
  own chat input, built again and measured against it: [the web board's chat box](web-chat-box.md).

| On the terminal board | On the web board |
|---|---|
| the filter row under the header: whose board, then `All`, `Coordinators`, `Needs you`, `Working`, `Ready`, `To do`, `Done`, `Automations`, `Pinned` with their counts (`tab`) | the same row under the header, each filter a pill with its count, the one shown lit in its own colour (`Needs you` red, `Working` amber, `Ready` green, `Coordinators` the accent, `Automations` purple, the others the base colour), a count of what needs you in red. A tap shows a filter. Whose board stands first, where there is another machine (`All machines`, or the machine shown, lit): a tap lists every board to pick one, as the sidebar's Boards section does. On a phone the row scrolls sideways and the names are the short ones (`Needs`, `Coords`, `Autos`). The page keeps the filter shown for the next page. None in the archive |
| the list, its groups, marks, progress, review and pull request tags | the same list; a pull request's tag is a link. The agent at work is a full dot, its reviewer the right half of one, and the board's own steps the lower half: all three drawn at one size (as text the two half discs came out a third the size of the dot). Every other mark is the terminal board's own character (one cross, `✗`; `◆` ready to approve; `@` a sign-in). Each row stands as far in as the terminal sets it: all machines → machine → project → task ([list headings](list-headings.md)) |
| a machine's row and a project's row: its coordinator, read as a task's row | the same: its coordinator's mark where a task has its mark (`○` idle, `◐` working, `?` asks you, `✗` could not start), its name, its time on the right; on autopilot a dimmed `[autopilot]` after its name and what autopilot does under it, in a task's grey second line. The top row (`All machines`, or the machine shown) says what it holds (`2 projects · 14 tasks`). A tap selects it, as a task's row; its caret folds and unfolds it. Selected, its **Open** shows the coordinator whole, with its buttons (start, stop, autopilot, archive this project). A status group's heading folds with a tap anywhere on it |
| moving between tasks | tap a task (or a machine's or a project's row): it is selected, as the cursor selects it on the terminal board, and nothing opens. The selected row has a bar in its gutter as well as its tint, and an **Open task** button (**Open automation** on an automation's row), which shows it whole; Enter on the selected row does the same. On a wide screen, with a task already open beside the list, the open task follows the selection, as the terminal board's side panel does |
| the task panel: status, what it needs from you, description, reviews, evidence, what the agent said, settings, session | the open task, every part whole, evidence as links. The settings stand right under the status and what it needs from you, so a long description never pushes them off the first screen |
| the settings page (`ctrl+e`): every work setting of the task by its plain name and value, `*` on the ones set for this task only | **All settings**, under the open task (and under an open automation), folded until you open it or pick `this task's settings` in More…: each setting with its plain label, what it means where that needs saying (`Research only`), its value, and `* this task only` on the ones that are. A switch or a choice is a list to pick from, anything else a box that saves on enter; **Put back** takes the task's own answer away. A refused value is refused in the board's own words and changes nothing; a device that only watches cannot change one; an archived task's are read-only |
| the hint line: what can be done to the selected task now | the buttons at the top of the open task, in the same words |
| the `?` menu | **More…** (and `⋯` in the header): only what applies now and is not a button already, in the buttons' own words (`Remove this task`, `Show every machine's tasks`). A key the page has no button for has no entry |
| whose board you see (`Alt+M`) | the filter row's first item, an entry of the sidebar's **Boards** section, or **Other machines** in More… |
| the chat, and who it talks to (`space` flips it); the box's edge names them (`vm-str ›  gtm  · Project coordinator`) | the web UI's own chat of whoever it talks to, docked under the list ([the web board's chat](web-chat.md)), under a bar whose switch has the two you can talk to: the top row's coordinator (`All machines`, or the machine shown) and the selected row's agent (`HP-179`, or on a machine's or a project's row its coordinator, `gtm`). The one you talk to is pressed, its name in the status colour of where it stands; after the switch, dimmed, the kind of agent it is (`· Agent`, `· Reviewer`, `· Project coordinator`, `· Machine coordinator`). A tap on the other presses the board's own key, as space does. With nobody else to talk to (the top row selected, a status group, an automation's row), only their name. There is no main Coordinator: the top row's is who the chat talks to by default |
| where they stand: the mark, the time, what they do | each choice of the switch with its mark, and beside it where they stand, with the check's progress (on a phone a word the mark says steps aside, a longer one has a line of its own) |
| the chat's box: the newest thing said, under a line that names who said it (the reviewer, during a review) | the web UI's own transcript of that agent, whole: its markdown, its tools, its scrolling. Where the web UI has none to show (a Coordinator not started yet, a task with no agent in its pane, another machine's Coordinator), the board's own conversation in the same place, drawn as the web UI draws its own, in the terminal board's inks: no labels on your words or on the replies of the one you talk to, yours bright, theirs grey; only someone else is named (`Reviewer`, `Autopilot`, the board), never the program's screen but where its conversation cannot be read. Markdown is read as the terminal reads it. **Full screen** (the arrows in the bar) gives the chat the whole screen |
| `alt+k` `alt+j` and the wheel: earlier in the conversation, and back | scrolling the chat, as anywhere in the web UI. The bar's grip sets how much of the screen the chat has under the list |
| `alt+c`: hide and show the latest message | **Hide** and **Show** (the chevron in the bar): the board's own key, so the board remembers which (also over a reload). Hidden, the bar and the box stay and the list has the screen |
| typing, then sending | the web UI's own box. A task's agent's words go as the web UI sends them; a coordinator's the board sends, as enter does, so the Coordinator knows where you were, and it decides what to start. The board's own box, where it stands in, is the web UI's built again ([the chat box](web-chat-box.md)) |
| who typed words are for | the switch says it. In the board's own box the words are locked to who it named when the first of them was typed: tapping another task or the switch meanwhile changes what you look at, never who gets the words. While the page shows somebody else, the box says so in its lower left (`To: HSW-1's agent · not who is shown above`) and its frame is the plain colour. Which machine's Coordinator is part of it: words typed for `vm-a`'s go to `vm-a` also after the view moved to another machine, and to nobody when `vm-a` is no longer reachable. The draft is kept with who it is for, so a reload brings both back |
| a question (approve a merge, remove a task) | a sheet with the same words and answers. For a merge or a remove, neither answer is the filled button, and after a failed review the yes reads **Merge anyway** |
| the board's message to you | a short notice above the chat. One that says something went wrong stays until dismissed. Nothing confirms what went well: the row says what is under way (`Starting…`, `Stopping…`, `Archiving…`), on a coordinator's row in its time's place |
| the automations: a group of each machine's part of the list, one of them whole, on and off, fire now, remove | the same group in the same place (after the machine's task groups; grouped by repo, under the repo), on a phone and on a wide screen, under a heading in the automations' own purple that folds as the others do ([colours](colours.md)). Its rows are in the task rows' places: the mark (`⟳`, an arrow going round, drawn at the size of the round marks; dimmed while off); the word `on` or `off` where a task has its number (on a list too narrow for the numbers it goes with them, so every title still starts at one left edge, and is then the first word under the name: `off · every 2h`); the name; the next or last firing where a task has its time (`next in 4m`); under the name the trigger (and why it was turned off, never clamped to two lines); and the task its last firing made where a task has its tags. A task an automation made has its `[auto]` tag in that purple. A row tapped open is the automation, whole, with the same buttons, each firing a link to its task. Another machine's is named as its tasks are ([automations](automations.md)) |
| find, the archive, grouping by status, repo or machine, all done tasks, other machines | the search button, and **Show** in More…; what is on shows as a chip under the filter row (`Find: …`, `Archive`, `By repo`, `By machine`), and its × puts the plain view back (by status, for a grouping). Whose tasks show is the filter row's first item |
| pins (`ctrl+p`) | the yellow pin mark `★` after a pinned row's name; **Pin** in More… on a task, a project's or a machine's row; `Pinned`, the last filter on the filter row, shows only what is pinned ([pins](pinned.md)) |
| archiving a project (`ctrl+a` on its row), restoring it in the archive | select the project's row: **Archive** is among its coordinator's buttons (Open) and in More… (`Archive this project with its tasks`), with the same question first where a task of it works, waits or is to do. In the archive, the archived project's row, selected, has **Restore this project** under it ([archiving a project](board.md#archiving-a-project)) |
| a machine whose tasks cannot be read: a line where its tasks would be, with why and what to do | one notice over the list: `vm-a and vm-b do not answer, so their tasks are not shown. The board keeps asking by itself.`, then a line each with why (`vm-a: cannot connect (its name is not known).`) and **What to do** behind it. Its × dismisses it in that browser until another machine stops answering or one answers for another reason. A machine asked for the first time is one quiet line, `Asking vm-a for its tasks…` |
| going into a task's agent | **Open this task's agent**: the web UI's own view of that pane (chat, or the terminal when no agent is in it); a closed space is opened again first ([opening a task's agent](#opening-a-tasks-agent)) |

**The page speaks web and fits a thumb.**

- No terminal key is named on it. A task's settings point to **All settings** below them, not to `ctrl+e`; a setting that
  is the task's own says so in words, not with a star.
- There is one back arrow at a time: on a phone, the open task's stands for the Board's.
- Nothing that is read is under 13 px, and everything that is tapped is at least 40 px high (the chat box's two send
  controls are the web UI's own sizes: 32 and 34 px with a mouse, 40 px on touch).
- The header names the repo, with no folder path.
- Every [key](keys.md) of the board is a button or a menu entry, except the ones that belong to a terminal: closing the
  board's terminal, the `?` menu itself, the side panel, and the full screen (the page scrolls, and shows the list and the
  task side by side when it is wide enough). The chat's keys are its own controls (the switch, Hide and Show, Full
  screen, tab and enter in the box).

**What the web chat does differently, and why.**

- `Esc` does not put a draft away (the page keeps it in its box, and over a reload), so there is nothing for `Ctrl+Z` to
  bring back there. The terminal's way stays on the terminal.
- The switch flips also while words are typed: the words stay locked to whom they were for and the box says so. (The
  terminal lets nothing but enter, tab and esc act on a draft.)
- The conversation is the web UI's: it scrolls as a chat does, the whole of it, rather than opening on the newest thing
  said alone.

**Fresh, and per browser.** While open, it asks every two seconds and redraws only what changed. Each browser has its own
selection, question and message, so a phone and a desktop do not get in each other's way. What you type stays in that
browser tab until it is sent.

A task on [another machine](other-machines.md) shows and acts as on the terminal board. Its agent's pane is on that
machine: the board says so instead of opening it.

## Opening a task's agent

**Open this task's agent** opens the task's space again when it was closed (approving or archiving closes it), as the
terminal board does, and the page then goes to the web UI's own view of its pane.

- **The page goes only once the web UI shows that pane.** A space opened again is a new workspace and pane, and the web
  UI's page drops an address that names a pane it does not know yet: Ovis 0.4.13 stayed on the pane it was on, and the
  button seemed to do nothing ([#356](https://github.com/eliasstravik/herdr-projects/issues/356)). So the board asks the
  web UI which panes it has (its `/api/machines`) until the new one is there, for 8 seconds at most. That took about a
  second in the check below.
- **When it still is not there, the page stays and says so**: *“HP-1 Make f” is open in herdr, but the web UI (Ovis) does
  not show it yet, so the page stayed here. Pick it in the web UI's sidebar in a moment, or open the agent again.*
- **The page checks that the web UI went.** Where the web UI still drops the address, it is written again a few times, a
  second apart; then the address is put back, the Board comes back, and it says the web UI did not go to that space.
- **A space that cannot be opened again** says why (its worktree is gone, say), and the page stays.
- A web UI that does not say which panes it has gets the address at once, as before.

## The chat full screen

**Full screen** (the arrows in the chat's bar) gives the chat the whole screen: the board's header and list step aside,
the bar stays at the top. It is the web UI's own chat view of whoever the chat talks to, docked
([the web board's chat](web-chat.md)), so it reads as it does for any agent; where the web UI has none to show, the
board's own conversation. The same button (its arrows now going in), Esc or a phone's back gesture go back.

## The same code decides

The web board works nothing out again. On the server it *is* the terminal board, opened with no terminal
(`crates/web/src/webui/view.rs`, and the board as data for the page in `crates/web/src/webui/model/`): the same list,
selection, questions and actions.

- A button presses the board's own key. What is typed is sent as enter sends it.
- The row's mark, words and columns, the task panel's lines, the hint line and the `?` menu are drawn by the board's own
  code, through a theme that names each colour instead of painting it, and the page gives each name a colour.
- So the two boards cannot disagree, and a change to the terminal board is a change to the web board.
- A key's button has a name of its own in the board's words there (`web` in `crates/flow/src/board/keys.rs`): "“Undo”
  brings it back".

## Other machines

The web UI has machines of its own, its **PCs**: this one, and the ones added in its Settings → Remote PCs. The board has
Herdr's saved machines ([other machines](other-machines.md)). The front door asks the web UI which PCs it has
(`GET /api/machines`, with your own login, nothing kept), and the board matches the two: by Herdr's own id for the saved
machine where the web UI names it, else by where they connect (the same host, port and Herdr session, and the same user
where both say one).

- **Every machine, or one.** An entry of the sidebar's **Boards** section ([above](#the-boards-section)) shows that
  machine's board, or all of them. `More` → **Other machines** goes round as `Alt+M` does on the terminal board: this
  machine, every machine, each one by itself. A chip says which: a tap on it picks another, its × goes back to this
  machine's own. Every button works on another machine's task as on the terminal board, and what you type while one
  machine shows goes to that machine's Coordinator.
- **It opens on the terminal board's choice.** A page shows whose board the terminal board shows (this machine, every
  machine, or one), so the two show the same tasks. A board picked on the web is the web's own and stays for the next
  page. Picking the terminal board's choice again makes the web follow the terminal board again.
- **Open this task's agent** on another machine's task: that machine reopens the task's space and restarts an exited
  agent, as it does for the terminal board, and the page goes to the web UI's own view of that pane on that PC
  (`?machine=<its PC>&pane=…`). Work never moves: only the page does.
- **The board goes with the web UI.** Opened while the web UI shows a pane of another PC, the Board is that machine's board
  by itself. Back on a pane of this machine, it shows what it showed before. A choice of your own stands while the web UI
  stays where it is.
- **What is missing is said, not hidden**: the web UI has no PC for that machine (add it in Settings → Remote PCs); the PC
  is not connected there; the web UI did not say which PCs it has (an older one); the other machine did not answer, or
  runs a plugin too old to reopen a space (the pane it last named is opened, and the board says to update); or the web UI
  is on a PC the board knows no machine for (`herdr machine add`).

Each machine with both plugins also has a front door and a Board of its own, at its own web UI's link.

**When the web UI runs on one machine and the tasks are on another.** The Board is served by the machine the web UI runs
on. So that machine needs this plugin too, on this version, and it needs the other machine as one of Herdr's saved
machines there (`herdr machine add`, or `board.machines`), reachable over SSH with `herdr-projects` on its path. The
machine with the tasks needs no web UI and no front door. The Board then shows that machine's tasks when the web UI is on
one of its panes, or through `More` → **Other machines**. The terminal board running on the machine with the tasks is
that machine's own program: that machine's plugin must be on this version for it to stay put in the web UI's terminal
view.

## The terminal board in the web UI's terminal view

The terminal board is an ordinary pane, so the web UI shows it in its terminal view like any other, on this machine and
on its other PCs. Three things made it unusable there, and the board handles them ([its size](board-size.md),
[its keys](keys.md)):

- **It is not disconnected.** The web UI's view of a pane sets that pane's size. The board used to take its size back
  from every such view, the web UI reattached by itself, and the two took the terminal from each other every four
  seconds. A view somebody is looking through now keeps the terminal, and the board is drawn for it.
- **`Alt` keys and `Shift+Enter` work.** The web UI's terminal sends the old byte forms for them, and the board takes those
  too.
- **A phone's input line** sends an Enter of its own after what you type. While a view of the pane is open, that Enter is
  dropped after `?`, `y`, `n` or space, so `?` opens the menu and leaves it open.

| The key | In a desktop browser | On a phone |
|---|---|---|
| letters, `Enter`, `Tab`, `Esc`, arrows | typed | the input line and the key bar (`Esc` `Tab` `Enter` and the arrows) |
| `Ctrl` keys | typed. A browser keeps a few for itself (`Ctrl+N`, `Ctrl+T`, `Ctrl+W` in Chrome): pick that line in the `?` menu | `Ctrl` on the key bar, then the letter with direct typing on (the keyboard button); or the `?` menu |
| `Alt` keys | typed. On a Mac, Option types a character instead: pick the line in the `?` menu | `Alt` on the key bar, then the letter with direct typing on; or the `?` menu |
| `?` | typed | typed into the input line and sent |

**Going into an agent** from the terminal board there: for a task on this machine, Herdr's own window goes to the agent,
and the board says to pick it in the web UI's sidebar. For a task on another machine, the board does not turn its
terminal into that agent's while the web UI shows it, and says where the agent is in the sidebar instead: the way back is
Herdr's `ctrl+b q`, and the web UI's view of the pane takes that chord for itself, so the board would stay inside the
agent. The web Board's **Open this task's agent** is the way that works from a browser.

## Who gets in

The board is as protected as the web UI, by the web UI:

- **The front door keeps no token, no cookie and no list of devices.** For every call of the board's, it asks the web UI
  itself, with the caller's own headers exactly as they came, whether that caller is logged in (its token and device
  gate: `server/auth.ts`, `server/access.ts` there). Anything but a plain yes is a no: no answer, an answer it cannot
  read, a web UI that is down. Not logged in, nothing of the board can be read or changed.
- **A call that changes something must come from the board's own page**: the same origin, and a header of the board's own
  (`x-herdr-projects`), which a page of another site cannot send. The web UI asks the same of its own controls.
- **A device the web UI paired to *watch*** can look at the board and change nothing: it can select, find, pick a filter on
  the filter row, put the grouping back to by status, and show or hide; it cannot send, approve, stop or archive.
- The page's own files (scripts and styles) hold no task and are public, as the web UI's own files are.
- **Private only.** The front door listens on `127.0.0.1` and nowhere else. The only way to it from another device is the
  web UI's own private link (`tailscale serve`). A link open to the internet (Tailscale Funnel) is never pointed at it,
  and nothing here ever makes one.

## Its two names

The web UI was renamed from herdr-web-ui to Ovis. Ovis 0.4.13, its release at the time of writing, still has the old
plugin id; its code after that has the new one, its installer moves an install over, and the names of its own files
changed with it. The board reads each of them as the web UI under that id does:

| What | Ovis (`devswha.ovis`) | Before the rename (`devswha.herdr-web-ui`) |
|---|---|---|
| Found in Herdr's plugins | first: where Herdr lists both (while Ovis's installer moves it), Ovis | when Ovis is not installed, or turned off |
| The port a start of it settled on (`plugin-port`) | in `OVIS_STATE_DIR`, else `~/.config/ovis` (`$XDG_CONFIG_HOME/ovis`); in `~/.config/herdr-web-ui` while that still holds state of its own, as Ovis itself does when it could not move it over | in `HERDR_WEB_STATE_DIR`, else `~/.config/herdr-web-ui` |
| Files pasted in its chat box, beside a pane | `.ovis/` in the pane's folder (or `OVIS_PASTE_DIR`) | `.herdr-web-ui/` |
| Its settings in the browser | `ovis:settings` | `herdr-web-ui:settings` |

- A task's worktree leaves both paste folders out of git (in the repo's `.git/info/exclude`; a repo whose rule was written
  before gets `.ovis/` added the next time a task starts there).
- The board's page reads `ovis:settings` first, then the old record (Ovis moves the old one over the first time it
  opens): Enter sends, Files at once, and Voice input are the ones you chose there.
- `herdr-projects web-ui` says which id it found, and its version: `Web UI (Ovis): found at http://127.0.0.1:7317
  (devswha.ovis 0.4.13)`. Not found, it names both ids.
- Its `PORT` and `HOST` are in the `env` or `.env` of the plugin's config folder (`herdr plugin config-dir devswha.ovis`),
  as before.

## Why a front door

The web UI has no place for an add-on (checked on herdr-web-ui 0.4.8). Its pages allow scripts and connections from its
own address only, and may not be put in a frame (`server/static.ts` there; Ovis 0.4.13 sends the same rules), so the
board has to come from the very address the web UI comes from. Changing the web UI's files is no way either: its updater builds each release from the
official code and wants a clean checkout, so a patch would be wiped by every update and could block updating.

So the front door stands in front of it instead:

- **It passes every request on as it is**: pages, `/api/*`, the `/ws` socket, streams, ranged and ready-compressed files,
  the service worker. The same method, address and headers go in, in their own order and spelling; the same status,
  headers and bytes come out as they arrive. Only the two connection headers every HTTP hop writes for itself
  (`Connection`, `Keep-Alive`) are its own, both ways: each request and its answer is one exchange that ends with
  `Connection: close`, so a proxy in front that keeps its connections open (envoy) brings the next request to the
  front door again, never straight to the web UI.
- **It adds one line to the web UI's page**: a script, from the same address, that puts the Boards section into the
  sidebar. Nothing else in the page changes, and its security policy stays as sent. A page that cannot be read is passed
  on as it is.
- **It corrects one answer**: what the web UI's Settings → Phone is told about its own link (`GET /api/access`). The web
  UI finds its link by looking for a `tailscale serve` entry that points at its own port. Through the front door the entry
  points at the front door's port, so the web UI would say it has no link and offer the command for a second one. The
  answer is given the link it already has. Anything else in it, and the answer itself whenever it already names a link or
  cannot be read, is passed on as it came.
- **It answers under one path the web UI does not use**, `/herdr-projects/`: the board's page files and its calls.
- **A fault of the board never breaks the web UI.** Whatever goes wrong on the board's side, a request that is not the
  board's still goes through. The board's work is done in a thread of its own, so reading tasks never holds up a
  terminal's keystrokes on their way through.
- **It shows the Herdr the web UI shows**, worked out as the web UI's own start does: the session that started both
  plugins comes first (Herdr hands every plugin its socket), then a socket the web UI's settings name. Not always the
  default session.
- **It follows the web UI**: through its restarts and its own updates (it keeps its port), and to another port when it
  takes one. While the web UI is down, its address answers "not answering right now", and the board lets nobody in.

The front door is part of the board's one page server (`herdr-projects serve`, which also serves
[evidence pages](task-panel.md)): one long-lived process, not two. Its bookkeeping (its process, its port, where it passes
things on to) is `data/front-door.json` in [the home folder](home-folder.md). When the plugin is updated, the process
hands over to a new one on the same port; a terminal open in the web UI reconnects by itself.

## What the link step changes, and how it goes back

The web UI does not share itself: it shows the command, then recognises its link by where that `tailscale serve` entry
points. The board changes exactly that: each private HTTPS entry whose root goes to the web UI's port goes to the front
door's port instead.

- What it changed is written down (`data/extras.json`).
- Going back only touches an entry that still points at the front door, and points it at where the web UI listens *now*,
  also when the web UI took another port since.
- An entry for another app, one for a sub-path, and one open to the internet are never touched.

**The two do not pull one link back and forth.** Only your yes (or `web-ui link`) takes a link; the front door never takes
one by itself afterwards. The web UI's Settings → Phone, opened at the link, keeps showing that link as its own (the one
corrected answer above). Its Phone setup run from Herdr on the machine reads Tailscale itself, finds no entry at its own
port, and makes one more link on another port: that one goes straight to the web UI, is the one it finds from then on,
and is left alone. `herdr-projects web-ui` lists both and says which has the Board.

The front door has a port of its own because Tailscale proxies to `127.0.0.1` only (checked on 1.102: a target on the
other loopback address is refused), so it cannot stand at the web UI's port number.

A browser that saved the web UI's page while the link went through the front door (the web UI's own offline copy) still
has the one script line in it after the link was pointed back. The script is then not found, nothing shows, and the next
load online replaces the copy.

## When the web UI changes

- **It updates itself**: nothing to do. The front door changes none of its files.
- **It moves to its new plugin id** (Ovis's installer does that): the board finds it under the new one by itself, within
  half a minute, and the front door follows ([its two names](#its-two-names)).
- **It was redesigned and the sidebar's foot is gone**: a small floating **Board** button shows instead of the Boards
  section, and says once why. The board itself is unchanged.
- **It gets an add-on slot of its own**: the front door can go. Until then, check these on a new version of the web UI:
  - the sidebar foot (`.sidebar-footer`);
  - the Agents section's classes the Boards section is built of (`AgentSidebar.tsx` and `.css` there:
    `agent-section-toggle`, `agent-section-caret`, `agent-list-contents`, `agent-item`, `agent-row`, `agent-copy`,
    `agent-title`, `agent-context`);
  - the drawer's scrim (`.scrim.is-open`);
  - its address parameters `ws`, `pane` and `view` (`src/lib/deepLink.ts`), and its `machine` address parameter;
  - `/api/health?scope=bridge` saying who asks, and that `/herdr-projects/` is still free;
  - what `/api/access` says about its link (`server/tailscale.ts`);
  - `/api/machines` naming each PC's `id`, `kind`, `target` and `state`;
  - its plugin id, its state folder, its paste folder and its settings record ([its two names](#its-two-names)).
- **It changes its chat input**: the Board's box is measured against it again, with one command
  ([the web board's chat box](web-chat-box.md)).

## Trying it against a copy

`HERDR_PROJECTS_WEBUI=<port>` (or `host:port`) names a web UI that Herdr does not list as a plugin. That is how a copy of
it is tried out, with a scratch Herdr and a scratch [home folder](home-folder.md). `off` there means none.

## Checked

On herdr-web-ui 0.4.8 with Herdr 0.9.3, a real web UI behind the front door, in a browser at a phone's and a desktop's
size, in dark and light:

- its chat and terminal, files (also ranged), its service worker and manifest, its push and update calls answer the same
  bytes through the front door as directly;
- the board lists, opens, sends, asks and acts;
- a restart of the web UI is followed;
- the link step and its way back change only the web UI's own private link, and its Settings → Phone, at the link, still
  names that link.

Its self-update was not run for real: the front door changes none of its files and keeps none of its state, and a restart
is what an update ends in.

**Other machines and the terminal view**, checked on 2026-10-13, herdr-web-ui 0.4.8 with Herdr 0.9.3. Two separate Herdrs
on one computer stood in for two machines: each with its own settings, state and board, both plugins installed in each,
the second reached only over SSH, saved in the first with `herdr machine add`, and added in the first web UI as a PC. A
real browser went through the front door at a phone's (390×800) and a desktop's (1280×800) size.

| What | Result |
|---|---|
| The web Board: every machine, one machine, back | the other machine's tasks show with its name; the chip says which |
| The web Board on another machine's task: pin, archive, undo, talk to its agent, approve and merge | each done on that machine (the merge at the phone's size); the answer shows on the board |
| What is typed while one machine shows | went to that machine's Coordinator, which added the task there |
| Open this task's agent, on another machine's task | landed in that agent's chat in the web UI, on that PC |
| The Board opened while the web UI shows a pane of the other PC | that machine's board by itself; back on this machine, what it showed before |
| The terminal board in the terminal view, before the change | the web UI's view and the board took the terminal from each other every four seconds, with a herdr window on the session and with none; no `Alt` key did anything |
| The terminal board there now, this machine's and the other PC's, desktop | stays; every `Alt` and `Ctrl` key, `?`, `Shift+Enter`, drafts, find, the arrows did what they do in a terminal |
| The same on the phone layout | `?` from the input line opens the menu and it stays open; the key bar's arrows move in it; `Alt` then `m` with direct typing shows every machine; a message from the input line reached the Coordinator |
| From the terminal board there: an agent put on the other machine's task with `Tab`, then into it with `Enter` | the agent started there. Going in showed its screen, and `ctrl+b q` did not come back: the web UI's own view detached instead. Hence the board no longer goes in from there |
| The mouse on the terminal board there (desktop) | the wheel over the list moves the selection a task a step; a drag over text is highlighted by the board; the selected row has its colour |
| The board as herdr's popup (`board.open popup`), in the terminal view | the popup is a pane the web UI lists: it stays, and `Alt+M`, `?` and `Shift+Enter` work in it as in the tab |
| The other machine on the plugin as it was before this change | Open this task's agent still lands in that agent's chat: its answer names no space, and a pane's id starts with its space's |
| The web UI's address with a PC and no pane (`?machine=<pc>#board`) | the web UI goes to a pane of this machine by itself, so the Board is this machine's: it follows a pane, not a bare PC |
| What a program gets from the terminal view | desktop: old bytes (`esc m` for `Alt+M`, `esc return` for `Shift+Enter`), also after it asked for the newer keyboard protocol; `ctrl+b` is held and sent with the next key. Phone: the input line sends the text, then a lone return 120 ms later, and ignores the key bar's `Ctrl` and `Alt`; the key bar and direct typing send the newer protocol's sequences. A click and the wheel arrive as mouse reports |

Not checked: a browser's own reserved keys (the test browser has none: `Ctrl+N` and the like are Chrome's known ones), a
Mac's Option key, the wheel over the chat and the mouse on a phone, a plugin so old that it sends no version (its message
is covered by a test only), a machine with no web UI or front door of its own, a web UI newer than 0.4.8 that takes its
PCs from Herdr's own list, and real separate computers: everything above was two Herdrs on one computer.

**The Boards section**, checked on 2026-10-14 on herdr-web-ui's newest code then (0.4.8 and later, `38d6863`) with Herdr
0.9.3: a scratch Herdr with two agents, the web UI in front of it, the front door in front of that, a board with tasks on
this machine, one other machine that answered with its tasks, and one that refused the connection (a stand-in for SSH).
In a real browser at a desktop's size (1280×800) and a phone's (390×800):

| What | Result |
|---|---|
| The sidebar | Boards under Agents, above Install app and Settings, in the same type, spacing and caret as Agents; this machine, the two others and All machines, each with its dot and line |
| The machine that refused | dimmed, `not reachable`, a ring for its dot; All machines says `· 1 not reachable` |
| Picking the other machine's entry | its board opened (`Only vm-str`, its Coordinator in the box), the entry highlighted |
| On the phone | folded at the start as Agents is, its dot beside the heading; unfolded, the same entries; a pick closed the drawer and opened that board over the page |

Not checked: a web UI that serves real other PCs (the PCs' names were checked by a test), and the section through the web
UI's own restart and update (the script and the place it goes are the ones the Board entry had).

**All machines first**, checked on 2026-10-19 the same way (herdr-web-ui 0.4.12, Herdr 0.9.3, one other machine that
answered and one that refused), at a phone's size (390×844) and a desktop's (1280×800): All machines stood at the top of
the Boards section and of the board's **Whose tasks to show**, and the machines under it kept their order. The terminal
board was the same as before: it lists no boards, and `Alt+M` still goes this machine, every machine, then each other one.

**Under its new name, Ovis**, checked on 2026-10-23 with Herdr 0.9.3: a scratch Herdr with Ovis's code under its new id
(its main, `22104941`, `devswha.ovis` 0.4.13) and its release under the old id (0.4.13, `devswha.herdr-web-ui`), each
with its own port; a board with two tasks, a real Coordinator and a real task's agent (Claude Code); the front door in
front of it, in Chromium at a desktop's size (1280×800) and a phone's (390×844, touch):

| What | Result |
|---|---|
| `herdr-projects web-ui`, the program before this change, Ovis installed | `not found: the plugin devswha.herdr-web-ui is not installed in herdr` |
| `herdr-projects web-ui` now, Ovis only, then both listed | `found at http://127.0.0.1:17417 (devswha.ovis 0.4.13)` both times |
| Ovis turned off in Herdr, the old id on | `found at … (devswha.herdr-web-ui 0.4.13)`; the front door followed it by itself within half a minute |
| The Boards section | under Agents, above Install app and Settings, under both ids; on the phone (Ovis) folded as Agents is, with its dot |
| A pick in it (Ovis at both sizes, the old id on the desktop) | the board opened over the page, docked on Ovis's own chat of the Coordinator, the Call button in its box; on the phone the drawer closed |
| A message typed in Ovis's box while docked | went through the board to the Coordinator, which answered in that chat; Back closed the board and gave the page back |
| A picture attached in Ovis's box, docked on the task's agent | kept in the task's worktree in `.ovis/`, which git leaves out (`!! .ovis/`); the box held its `@path` |
| A browser whose settings were under the old key (Enter as a new line, 3 files at once) | Ovis moved them to `ovis:settings`; the board's box read Enter as a new line and 3 files at once there, and from `herdr-web-ui:settings` in front of the old id |

Not checked: Ovis's installer moving a real install from the old id to the new one (both were linked by hand), and
other machines under the new name (their PCs come from the same `/api/machines` as before).

**Opening a task's agent whose space was closed** ([#356](https://github.com/eliasstravik/herdr-projects/issues/356)),
checked on 2026-10-23 with Herdr 0.9.3: a scratch Herdr with Ovis 0.4.13 (its release, `devswha.herdr-web-ui`), the front
door in front of it, a real task's agent (Claude Code) whose task was ready to approve, in Chromium at a desktop's size
(1280×860):

| What | Result |
|---|---|
| Open this task's agent, its space open | the page went to the agent's pane |
| The same, its space closed, the program before this change | the space came back as a new workspace (`w3`) and the page was sent there at once; Ovis did not know it yet, put its address back to the pane it was on, and nothing said why |
| The same now | the board waited for Ovis to list the new pane (about a second), and the page went there; done three times, each time a new workspace |
| Ovis sent to a pane it never shows (the board's answer changed on its way to the page) | the address was written again three times, then put back; the Board came back with *The web UI (Ovis) did not go to “DEMO-1 Write f.txt with the word hello”…* |

Not checked: another machine's task whose space was closed (the same wait, on that PC's list of panes: covered by tests),
and Ovis's newest code, which handles that address the same way (`resolveLink`).
