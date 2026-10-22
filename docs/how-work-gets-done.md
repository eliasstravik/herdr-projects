# How work gets done

**You delegate a task. An agent works on it in its own worktree. When it says it is done, the board checks it, opens a
pull request, has a second agent review it, and waits for your yes. Then it merges and cleans up.**

```
delegate ─► agent works ─► finish line: commit, catch up with main, check ─► pull request ─► review ─► your yes ─► merge ─► clean-up
                 ▲                                    │                                         │
                 └──────── a failed check, a failed review or a conflict goes back ◄────────────┘
```

All of this runs with the board open or closed.

## Delegating

- **In a git repo**, the plugin asks Herdr for a worktree space (`herdr worktree create`), on its own branch
  (`work/<n>-<title>`), started from the remote's default branch (or from your commit, with `--workspace.from here`, or
  from another branch at the remote, with `--workspace.from develop`). A task in a repo never works in your checkout.
- **Tasks of one repo can start at the same moment.** Herdr is handed the commit the task starts from, not
  `origin/main`, so git writes no upstream into the repo's shared `.git/config` at the start (the push at the finish
  sets it). Another git command holding that file anyway (`could not lock config file`) is waited out, five tries in
  under four seconds, and a start that fails still removes the empty branch it made, so the next start takes the name.
- **The branch's name is `workspace.branch`**, `work/{n}-{slug}` unless you set it: for you, for one repo, or for one
  task, never in the repo's own file. `{n}` is the task's number (`7`, or `7b` for an attempt), `{slug}` its title
  (`fix-the-login`), `{key}` its repo's key and number (`KIC-7`). So `feat/{key}-{slug}` makes `feat/KIC-7-fix-the-login`.
  A name git would not take is refused, with the nearest one it does. A name already taken gets `-2` after it.
- **Started from a named branch** (`workspace.from develop`, for a long branch with small tasks on top), the task lands
  there too. The branch is fetched from origin on every start (10 seconds at most, never asking for a password); one
  origin does not have is refused with the reason, and the task waits as failed. If origin cannot be reached, the copy it
  last gave is used. The task keeps up with that branch, its pull request merges into it (`gh pr create --base develop`),
  a pull request the agent opened itself is taken on only while it merges into it, and `merge.checks` reads the checks of
  a pull request into it. It needs `merge.pr` on: a local merge lands on your checkout's branch, so a named branch with
  `merge.pr` off is refused at the start and at the merge, for now.
- `workspace.setup` runs. Then the agent starts in that space's root pane, with the task and its rules.
- **Outside git**, the agent works in place, in a plain Herdr space.
- **At most `board.agents` (4) agents work at once.** More wait their turn. Tasks that wait for you hold no slot.

## Agents run with full access by default

```
claude --dangerously-skip-permissions
codex --dangerously-bypass-approvals-and-sandbox --no-daemon
```

(`--no-daemon` only from Codex 0.156.0, the first that has it: an older one quits on it.) This is as pi-work's agents
did. They can run anything as you, without asking: the rules and the review brief are guidance, not a sandbox.

- `worker.access ask` (and `reviewer.access`, `coordinator.access`) starts a program without those flags, so it asks
  before it acts. pi cannot ask, and the value is refused for it.
- `worker.access auto` starts it in its program's own auto mode, which does the safe actions without asking and stops
  the risky ones: `claude --permission-mode auto`, `codex --approve-for-me` (and Cursor's, Qwen Code's, Qoder's).
  Refused for every other program ([auto mode](auto-mode.md)).
- So a new task does not wait behind a "trust this folder?" dialog, the plugin marks the task's repo as trusted in the
  agent's own settings (`~/.claude.json`, `~/.codex/config.toml`) before starting it. Only for a role with full or auto
  access.

## How the board knows a turn ended

The plugin starts every agent itself, and passes at launch:

- **a turn-end hook** on the command line: Claude Code `--settings <a file in the task's own folder>`, Codex
  `-c notify=[…]`, pi `-e <a one-file extension>`;
- **a session id** of its own choosing, where the program takes one;
- **the task's rules** as a standing system prompt: a file, or for Codex inline when short
  ([long rules and long messages](could-not-start.md#long-rules-and-long-messages)).

Nothing is written to your own agent settings, and your own hooks keep running.

At each turn's end, the hook runs `herdr-projects turn-ended` (silent, at most two seconds, protected by a per-launch
secret). The finish line then reads the end of that agent's saved conversation: its last message, an API or login error,
its token use.

**An agent that stopped to wait for you** (a question, a permission, a sign-in) has not finished, however often it asks:
none of the finish line runs. How an agent says so, and what the board does:
[an agent that waits for you is not finished](waiting-for-you.md).

### Without a hook

A crash, a program older than its adapter's minimum version, another kind of agent, Codex when you have your own `notify`
set, or a hook that failed three turns in a row: then Herdr's status is only a guess from the screen, so "idle" alone
never finishes a task. The turn counts as ended when:

- the saved conversation ends in a finished message or an error; or
- a closing note was written to the task's own folder (`closing-note.md`,
  [where it is](home-folder.md#nothing-of-the-boards-goes-into-the-folder-an-agent-works-in)). The rules still ask for
  one, as the fallback.

Otherwise the task is held as "Stopped without a summary": `Ctrl+R` tells the agent to continue, `Ctrl+L` takes the work
as it is.

A task whose finish the board missed (its agent at rest, the row still "Finishing") is finished with `Ctrl+L` or
`herdr-projects finish <id>`. What the agent said last is the summary.

## The finish line

When the agent's turn ends, the board:

1. **Commits** anything left over, **brings the branch up to date with main** in the task's own worktree
   ([below](#staying-current-with-main)), and **runs the repo's `check`**. A failure goes straight back to the agent,
   three tries.
   - Another Git process holding the worktree's index (`index.lock`: a repo gate's `git status`, an editor) is waited
     out: a few tries over about four seconds, and nothing left to commit by then is no failure. A lock that stays
     fails the commit, naming the lock file.
   - A new round starts clean: the failure of the round before (a failed commit, a reviewer out of credits) is cleared
     as soon as the check starts, so the row and the details never show it while the work is checked and reviewed.
2. **Pushes the branch and opens a draft pull request**, when there is an `origin` remote and `gh`.
   - A pull request the agent already opened for the task's branch is taken on instead, and goes through the review and
     the merge as usual.
   - The push may run for `merge.pushTimeout` (2 minutes unless set), the repo's pre-push hook included. A repo whose
     hook runs its tests needs more: `herdr-projects set merge.pushTimeout 10m --repo <name>`, or `none`. A push that
     runs out of time is stopped and the finish fails, saying so (`git push did not end within 2m
     (merge.pushTimeout)`); finish again once it is raised.
   - A branch that brings main nothing gets no pull request: no change was needed, or the work is on main already (the
     agent merged its own pull request, or pushed to main itself). The task is done, and its row says why: "No change
     needed", "Already on main" or "Already merged in #133". Nothing is pushed then, so a repo the task archived on
     GitHub is never pushed to. Work that is not on main is never done, and with the repo archived the task stops,
     saying so.
3. **Has a second agent review it**, adversarially, in a tab of its own in the same Herdr space
   ([a task's tabs](tabs.md)). It says what it found, with evidence. Whatever it changed is put back.
   - When the repo has a `check`, the reviewer is told the command and that it passed on this commit, and to run tests
     only through it.
   - A reviewer that ends its turn with no verdict is started once more by itself, as the same review. If it again gives
     none, the review fails, with its last message.
   - Only the reviewer's own turn ending counts. A helper it hands part of the review to (Codex's sub-agents, Claude
     Code's background agents) finishing is not the review ending, and while a helper is still at work the review is
     still going. (The same holds for a working agent and its task.) A helper is back when its answer is in the
     conversation (a message of its own, or a queued one taken into a turn still going), or its own saved conversation
     ends in its finished reply.
   - A reviewer whose turn its provider cut off is retried by the board ([a turn cut off by the provider](provider-errors.md)).
   - The board decides from the findings ([the gate](gate.md)). A review that fails goes straight back to the agent.
   - The row says the result and a tally (`Pass · 2 minor`). Its `R#2` tag says which review it was and, by its colour,
     how it went. The task panel has its summary, and every earlier review of the task.
4. **Waits for your approval** (`Ctrl+L`).

## Approving and merging

Your yes makes one last sync with main. If that changed anything, it checks (and reviews, by the same rules) again. Then
it merges:

- **through the pull request**, only at the commit that was reviewed. A commit pushed to the pull request meanwhile
  withdraws the approval. On a repo with a merge queue, the task waits until GitHub says the pull request is merged;
- **or locally**, squashed by default.

A conflict found here goes back to the agent the same way, and the merge you approved is retried after its repair. A
checkout with unsaved files is never forced. Merging without your yes: [merging by itself](gate.md#merging-by-itself).

### GitHub's checks must pass first

With `merge.checks: github`, a pull request merges only once its checks on GitHub pass, by itself or on your yes, in
every repo the board merges in. `merge.checks: none` does not look at GitHub's checks.

| GitHub's checks are | The board |
|---|---|
| still running | waits. The row says how far: `Waiting for GitHub checks · 2 of 3 passed`, with the board's own `◇` (nobody works on it meanwhile). It merges by itself once they pass |
| failed | merges nothing. The failure goes back to the agent like a failed check of the board's own, with the failing check's name and the end of its log, within `review.rounds` (a merge you approved goes on once GitHub's checks pass on the repair). After that it waits for you, and `approve <id>` refuses and says why, as for a failed review, until `approve <id> --anyway` |
| not reporting, still running after `check.timeout` | waits for you, with that reason |
| passed, or there are none | merges |

A failure is of one commit: once a repair is pushed, the old commit's failed checks block nothing (also when main is
merged in before it merges). Only the checks of the commit that would merge count.

Each check is counted once, a workflow for its jobs until GitHub lists them. `details` names what it waits for
(`checksWaitingFor`).

**The board decides from what the repo expects**, not only from what GitHub lists so far:

- Each GitHub workflow of the branch that runs for this pull request must have its run on the commit, and that run must
  be through and passed: every job of it, every member of a matrix, also one GitHub has not listed yet (counted as one
  check still to pass). The workflows are `.github/workflows`, their triggers read as GitHub reads them: `push` and
  `pull_request`, their `branches` against the branch, `paths` against the files the pull request changes, `types`.
- A workflow that cannot run for it (only for other paths, only into another branch, only on tags) is not waited for.
- Each check the target branch requires (its protection or rulesets) must be listed and pass. A check listed on the
  commit before and gone from the list now has not passed.
- With nothing listed at all, the task says `Waiting for GitHub checks to start` where an outside CI reports in the repo
  (on the newest commit of the target branch, or on its other pull requests). A question GitHub cannot answer counts as
  yes.
- All of these wait at most `check.timeout`, then for you, with the reason. A repo with no checks at all merges with none
  listed.

### A fine-grained GitHub token is enough

Such a token cannot have the Checks permission (GitHub offers it to GitHub Apps only), so GitHub refuses it the list of a
pull request's checks. The board then reads them from what the token can read:

- **Actions**: the runs on the commit, each of their jobs and matrix members, and the log of a failed one;
- **Commit statuses**: an outside CI, Vercel.

Give the token read access to those two. Nothing asks for Checks, and a refused list is not a login problem. The same
rules hold: running waits, a failure goes back with its log, a workflow not started is waited for, all within
`check.timeout`.

- What such a token cannot read at all is the check runs of another app that posts no commit status. The task merges
  once everything it can read has passed, and says what it could not read
  (`Merged: … Not read: check runs of apps other than GitHub Actions …`).
- A check the branch requires that no job or status of the commit answers is waited for while anything readable still
  runs, then named there too. GitHub itself still refuses a merge that a required check blocks.
- A token that may not read Actions or Commit statuses either leaves the task waiting for you with
  `Could not read GitHub's checks`, naming the two permissions to give it.
- A check is named once, whichever way it was read (a workflow `Check` and its job `check`; its runs for the push and for
  the pull request). What was remembered for an earlier commit, or read the other way, is not looked for again.

### When gh cannot read GitHub at all

(Not logged in, no network.) A merge waiting for its checks says so on its row
(`Waiting for GitHub checks · could not read them (…)`), and goes on by itself once gh reads again. Past `check.timeout`
it waits for you with `Could not read GitHub's checks`.

Herdr may start the board's keeper (after a reboot) without your login shell's environment. So the board takes your PATH
and gh's login variables (`GH_TOKEN`, `GH_HOST`, `GH_ENTERPRISE_TOKEN`, …) from a login shell, never writing a token
down. It takes the SSH agent (`SSH_AUTH_SOCK`, `SSH_AGENT_PID`) the same way, so that its `git fetch` and `git push` over
SSH work: only when the keeper has no agent of its own and the login shell's socket is there.

## Staying current with main

Keeping branches up to date is the board's job, not the agent's: nothing in an agent's rules asks for it. Tasks run in
parallel and touch the same files, so the board itself fetches main and merges it into each task's branch, in that task's
own worktree:

- **before review**: when an agent finishes, before the check and the review, so the reviewer sees the work as it will
  really land;
- **whenever main moves** (another task merged, a merge on GitHub, your own push; the remote is asked every two minutes):
  every task waiting for your approval is updated, and a task under review the moment its review ends;
- **after you approve**, once more, as the last safety net. It should rarely find anything.

**A clean update is silent.** If it changed the commit, the check runs again on the result. The review is not repeated
when taking main in is all that happened to work that already passed ([below](#a-review-that-passed-is-not-repeated-for-a-merge-of-main)),
or when the update touched none of the files the task itself changed. The verdict you have stands, and the task does not
ask for your eyes again.

**A real conflict is rolled back at once** (your branch and the task's are as they were) and goes straight to the task's
agent as a plain message: the conflicting files, the target commit, and that it should resolve them in its own worktree
and finish again.

- Until it has, the task is not waiting for approval: its row says "resolving a conflict with main".
- After `review.rounds` repairs in a row that still conflict, you are asked, with the details kept. `review.rounds 0` on a
  task keeps its conflicts for you to send with `Ctrl+R`.
- Every conflict leaves one line in `data/conflicts.jsonl` in [the home folder](home-folder.md) once it is repaired: the
  task, the files, the minutes from found to up to date again, how often the agent was sent back, and whether a new
  review followed (`crates/flow/src/sync/conflict_log.rs`).

**Only a task at rest is ever updated**: its worktree clean, on exactly the commit that was reviewed.

- A fetch that fails is never taken for a sync, and never fails finished work: the turn is checked and reviewed, then
  held as "behind main" until a fetch works ([troubleshooting](troubleshooting.md#behind-main-main-could-not-be-fetched)).
  A finished turn whose worktree has left the task's own branch is saved as "Task is on the wrong branch".
- A waiting task whose main cannot be fetched, or that was not at rest when main moved, is held as "behind main": its row
  says so, `Ctrl+L` is refused with the reason, and the board tries again every minute until the branch really is up to
  date.

**"Main" is where the task will land**: the remote's main when it lands through a pull request, or the branch it was
started from by name (`workspace.from develop`). Otherwise (no remote, or `pr` off), the branch your checkout was on when
the task first finished. For those, checking out another branch merges
nothing into waiting tasks, and they are held as "behind main" until your checkout is back on it.

### A review that passed is not repeated for a merge of main

When the last review passed and the only change since the commit it looked at is main being merged in, the board runs
the repo's `check` on the result instead of a new review.

- **Tests pass**: the task goes on to approval or the merge, as a pass.
- **Tests fail**: the failure goes back to the agent, as a failed check always does, and its fix is reviewed.

The rule for "only the merge of main" is fixed, and checked in git from the commit the reviewer looked at to the branch
as it stands:

1. every commit since is a merge commit with two parents: the branch as it was, and a commit of main. An ordinary commit
   on top, a rebase, or another branch merged in is new work;
2. each of those merges is exactly what git makes of its two parents by itself. Only the files git could not merge (the
   conflicts) may differ, because resolving them is what the merge needed. Any other file touched in the merge commit is
   new work.

So a conflict the agent resolved in its merge commit, touching only the conflicting files, keeps the review. The agent is
told this with the conflict. Anything else is reviewed as before, and so is everything the board cannot read for sure: a
last review that failed or gave no verdict, work outside a worktree.

A repo with no `check` has no tests to prove a merge into the task's own files. There, a review stands only when a clean
update touched none of the files the task changed (the narrower rule above), and the task says `no check is set` instead
of `tests pass`.

The task says plainly what happened: its message reads `Re-merged with main, tests pass, review 2 still stands`; that
review's box carries the same line (with when, and how often); the row keeps showing the review that passed
(`Pass · 2nd review`); and `details` lists each time under that review as `kept`. This is fixed behaviour, not a setting.

## Stepping in

Typing in the agent's own terminal, or replying from the board, starts all of this over: an approval is withdrawn, a
pending auto-merge is cancelled (a cancellation that fails is saved as a visible failure), and the old review is marked
stale. A chat with reviewed work is the exception: what it starts over only counts once the code changed.

### Chatting with a finished task re-reviews only what changed

When you talk to the agent of a task that is under review, waits for your yes, or is done (your own words: from the
board, `tell`, or typed in its own pane; the keys that send a review or a failure back are not a chat), the board keeps
what the last review covered: the commit it looked at, with nothing uncommitted on top. When the agent's turn ends, it
compares:

- **No code changed** (a question answered, something explained, nothing committed or edited): the task goes back to the
  stage it was in, with the same review, and no check, review or merge starts. Its row says so after the review's result
  (`Pass · 1 minor · Chatting · no changes, review kept`), and the task panel lists it under that review
  (`a chat changed no code, review 2 still stands`, with when and how often).
- **A review was under way** when you wrote: it goes on (the box says `Send message to Agent: the review goes on unless the code changes`). What the reviewer changed is not put back while the agent may be at work in the same folder. If
  the agent's turn changed nothing, the review's result stands as it ends. If it changed the code, the review is stopped
  (or, if already through, its result no longer covers the work), and the change waits as below.
- **The code changed**: nothing is reviewed after each message. The row says `Changes made · review starts when you're done`, and the board waits until you are:
  - you left the chat (selected another task on the board, or, for a chat in its own pane, Herdr shows another space
    than the agent's); or
  - `review.afterChat` (60 seconds) passed with no new message from you and the agent at rest.

  Then one review runs on the final state, through the whole finish line (commit, main brought in, check, push, review).
  Another message before then restarts the quiet time. A turn that puts the code back as it was reviewed puts the task
  back too.

Everything else stays as it was. What you ask for in a chat is still reviewed before any merge. A task that merges by
itself never merges while a chat is open or with unreviewed changes. An approval you gave is withdrawn when the code
changes. A merge waiting for GitHub's checks is called off by any chat: its change waits for you to be done, like any
chat's, and the task waits for your new yes after the review.

## When a task is done

Merged here or on GitHub, finished with nothing to merge, or called done by you: its space leaves Herdr's sidebar by
itself, and its worktree and branch are removed. `workspace.cleanup false` keeps everything.

**Work is never lost for it:**

- A checkout with unsaved files, or with work that was never merged, stays on disk with its branch. The task says where,
  and a reply continues there.
- A space you are using again is not closed: a turn in the agent's own terminal, a command running in any of its panes
  (including one a finished agent left running in the background).
- What an agent program keeps running as part of itself is no such command: its MCP servers, or its own program under
  an npm wrapper (`codex` installed with npm). They are in the agent's own process group, and an MCP server's input and
  output are wired to the agent itself (a pipe or a socket the agent holds the other end of). So a finished agent with
  MCP servers is closed like any other. A command whose streams go elsewhere (the terminal, a file, `/dev/null`) keeps
  the space open, in the agent's group or not, and so does one in a group of its own. What cannot be told counts as a
  command.
- Nor is one Herdr could not answer for, or whose Herdr session is not running: then nothing of the task is touched.
- The clean-up is tried again by itself (about every half minute, board open or closed) and happens once the space is
  free. After a turn of the agent, only once that has been quiet for ten minutes. Work that appeared meanwhile is kept.
- An agent Herdr cannot place (status "unknown") counts as at work until its screen has stood still for two minutes, its
  saved conversation ends in a finished turn, and nothing runs under the agent program but its own helpers (above). So
  an agent of unknown status that keeps a command of its own running is never closed by itself: archive or remove its
  task.

A reply to work that was cleaned up starts a new round in a fresh worktree.

Herdr also opens the repo itself as a space when it makes a repo's first worktree. That one is closed with the repo's last
task space, unless it was open before or you use it (a second pane or tab, an agent, a running command, or you are
looking at it).

## With the board closed: the keeper

A small background process, the keeper (`herdr-projects keeper`), does what the open board's tick does: it polls pull
requests, starts queued tasks, runs the Watcher, and keeps Herdr's sidebar current.

- **The sidebar.** An agent's entry there is a small copy of its task's row on the board ([Herdr's sidebar](sidebar.md)):
  the task's number and title; who is on it and where it stands, in the board's own words (one list:
  `crates/core/src/status_words.rs`), with the review round and the pull request; and the row's percent and time to go
  (`8% 16m`). The pane's title names the task the same way, so the web UI's sidebar reads the same. A running review has
  two entries, the agent's and the reviewer's: they are two agents. The sidebar shows only tasks with a live agent pane;
  the others (queued, your own) are on the board only.
- **No notifications.** It raises no pop-up or sound: what needs you is on the board and in Herdr's sidebar.
- **Its life.** Herdr starts it with its server. Any Herdr event, or opening the board, starts it again if it died. It
  replaces itself when the plugin is updated, and exits when Herdr is gone. It looks after this machine's tasks only.
- An open board does the same. After an update it loads the new version by itself, in place, once nothing is half typed.
- **One pass at a time.** This round of looking after the tasks also runs after every hook and every background job.
  Only one runs per Herdr session at a time: one that comes while another is under way (a slow Herdr) does nothing
  itself, and the one under way goes round once more at its end if it was asked to (a pull request poll is never lost).
  A clean-up still running for a task is not started again.
