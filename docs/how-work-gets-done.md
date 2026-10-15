# How work gets done

**Delegating.** In a git repo the plugin asks herdr for a worktree workspace (`herdr worktree create`), on its own
branch `work/<n>-<title>`, started from the remote's default branch (or your commit with `--workspace.from here`).
Always: a task in a repo never works in your checkout. `workspace.setup` runs, then the agent starts in that workspace's
root pane and gets the task and its rules. Outside git the agent works in place in a plain herdr workspace. At most
`board.agents` (4) agents work at once; more wait their turn, and tasks that wait for you hold no slot.

**Finished = the agent says its turn ended.** The plugin starts every agent itself, and at launch it passes a
turn-end hook on the command line (Claude Code: `--settings <a file in the task's own folder>`; Codex:
`-c notify=[…]`; pi: `-e <a one-file extension>`), a session id of its own choosing where the program takes one, and
the task's rules as a standing system prompt. Nothing is written to your own agent settings, and your own hooks keep
running. At each turn end the hook runs `herdr-projects turn-ended` (silent, at most two seconds, protected by a
per-launch secret), and the finish line reads the end of that agent's saved conversation: its last message, an API or
login error, token use. With the board open or closed it then:

1. commits anything left over, brings the branch up to date with main in the task's own worktree (see "Staying
   current with main"), and runs the repo's `check` (a failure goes straight back to the agent, three tries);
2. pushes the branch and opens a draft pull request, when there is an `origin` remote and `gh`. A pull request the
   agent already opened for the task's branch is taken on instead, and goes through the review and the merge as usual.
   A branch that brings main nothing gets none: no change was needed, or the work is on main already (the agent merged
   its own pull request, or pushed to main itself). The task is done, and its row says why: "No change needed", "Already
   on main" or "Already merged in #133". Nothing is pushed then, so a repo the task archived on GitHub is never pushed
   to; work that is not on main is never done, and with the repo archived the task stops, saying so;
3. has a second agent review it adversarially, in a tab of its own in the same herdr space (see [A task's
   tabs](tabs.md)); it says what it found and brings evidence; whatever it changed is put back. When the repo has a
   `check`, the reviewer is told the command and that it passed on this commit, and to run tests only through it. A
   reviewer that finishes its turn with no verdict is started once more by itself, as the same review; a second one that
   does fails the review, with its last message. Only the reviewer's own turn ending counts: a helper it hands part of
   the review to (Codex's sub-agents, Claude Code's background agents) finishing is not the review ending, and while one
   of its helpers is still at work the review is still going (the same holds for a working agent and its task). A
   helper is back when its answer is in the conversation (a message of its own, or a queued one taken into a turn still
   going) or its own saved conversation ends in its finished reply. One
   whose turn was ended by its provider is tried again by the board (see [A turn cut off by the
   provider](provider-errors.md)). The board decides from the findings (see [The gate](gate.md)): a review that fails
   goes straight back to the agent. The row says the result and a tally (`Pass · 2 minor`), and its `R#2` tag which
   review it was and, by its colour, how it went; the task panel has its summary, and every earlier review of the task;
4. waits for your approval (`Ctrl+L`).

None of this happens when the agent stopped to wait for you (a question, a permission, a sign-in): that is not a
finish, however often it asks. How an agent says so, and what the board does then:
[An agent that waits for you is not finished](waiting-for-you.md).

**Without a hook** (a crash, a program older than the adapter's minimum version, another agent kind, Codex when you have
your own `notify` configured, or a hook that failed three turns in a row) herdr's status is only a guess from the
screen, so "idle" alone never finishes a task. The turn counts as ended when the saved conversation ends in a finished
message or an error, or a closing note was written to the task's own folder (`closing-note.md`, [where it
is](home-folder.md#nothing-of-the-boards-goes-into-the-folder-an-agent-works-in); the rules still ask for one as the
fallback). Otherwise the task is held as "Stopped without a summary": `Ctrl+R` tells the agent to continue, `Ctrl+L`
takes the work as it is. A task whose finish the board missed (its agent at rest, the row still "Finishing") is
finished with `Ctrl+L` or `herdr-projects finish <id>`: what the agent said last is the summary. Typing in the agent's own terminal, or replying from the board, re-arms all of this: an
approval is revoked, a pending auto-merge cancelled (a cancellation that fails is saved as a visible failure), the old
review marked stale. A chat with reviewed work is the exception below: what it re-arms only counts once the code
changed.

**Chatting with a finished task re-reviews only what changed.** When you talk to the agent of a task that is under
review, waits for your yes or is done (your own words, from the board, `tell` or typed in its own pane; the keys
that send a review or a failure back are no chat), the board keeps what the last review
covered: the commit it looked at, with nothing uncommitted on top. When the agent's turn ends it compares:

- *No code changed* (a question answered, something explained, nothing committed or edited): the task goes back to
  the stage it was in, with the same review, and no check, review or merge starts. Its row says so after the
  review's result (`Pass · 1 minor · Chatting · no changes, review kept`), and the task panel lists it under that
  review (`a chat changed no code, review 2 still stands`, with when and how often).
- *A review was under way* when you wrote: it goes on (the box says `Send message to Agent: the review goes on unless
  the code changes`), and what the reviewer changed is not put back while the agent may be at work in the same folder.
  When the agent's turn changed nothing, the review's result stands as it ends; when it changed the code, the review
  is stopped (or, if it is through by then, its result no longer covers the work) and the change waits as below.
- *The code changed*: nothing is reviewed after each message. The row says `Changes made · review starts when you're
  done`, and the board waits until you are: you left the chat (selected another task on the board, or herdr shows
  another space than the agent's, for a chat in its own pane), or `review.afterChat` (60 seconds) passed with no new
  message from you and the agent at rest. Then one review runs on the final state, through the whole finish line
  (commit, main brought in, check, push, review). Another message before then starts the quiet time again, and a turn
  that puts the code back as it was reviewed puts the task back too.

Everything else stays as it was: what you ask for in a chat is still reviewed before any merge, a task that merges by
itself never merges while a chat is open or with unreviewed changes, and an approval you gave is withdrawn when the
code changes (a merge that waited for GitHub's checks is called off by any chat: its change waits for you to be done,
like any chat's, and the task waits for your new yes after the review).

**With the board closed** a small background process, the keeper (`herdr-projects keeper`), does what the open board's
tick does: it polls pull requests, starts queued tasks, runs the Watcher, keeps herdr's sidebar current. An agent's
entry there is a small copy of its task's row on the board ([herdr's sidebar](sidebar.md)): the task's number and title;
who is on it and where it stands, in the board's own words (one list: `crates/core/src/status_words.rs`), with the
review round and the pull request; and the row's numbers, its percent and time to go (`8% 16m`). The task is named the
same way in the pane's title, so the web UI's sidebar reads the same. A review that is running has two entries, the
agent's and the reviewer's: they are two agents. It raises no notification, pop-up or sound: what needs you is on the
board and in herdr's sidebar. herdr starts it with its server; any herdr event or opening the board starts it again if
it died; it replaces itself when the plugin is updated, and exits when herdr is gone. An open board does the same: after
an update it loads the new version by itself, in place, once nothing is half typed. It looks after this machine's tasks
only. The sidebar shows only tasks with a live agent pane; the others (queued, your own) are on the board only.

**Staying current with main** is the board's job, not the agent's: nothing in an agent's rules asks for it. Tasks
run in parallel and touch the same files, so the board itself fetches main and merges it into each task's branch, in
that task's own worktree:

- *before review*: when an agent finishes, before the check and the review, so the reviewer sees the work as it
  will really land;
- *whenever main moves* (another task merged, a merge on GitHub, your own push; the remote is asked every two
  minutes): every task waiting for your approval is updated, and a task under review the moment its review ends;
- *after you approve*, once more, as the last safety net. It should rarely find anything.

A clean update is silent. If it changed the commit, the check runs again on the result. The review is not repeated when
taking main in is all that happened to work that already passed (below), or when the update touched none of the files
the task itself changed: the verdict you already have stands and the task does not ask for your eyes again. A real
conflict is rolled back at once (your branch and the task's are as they were) and goes straight to the task's agent as a
plain message: the conflicting files, the target commit, and that it should resolve them in its own worktree and finish
again. Until it has, the task is not waiting for approval: its row says "resolving a conflict with main". After
`review.rounds` repairs in a row that still conflict, you are asked, with the details kept. `review.rounds 0` on a task
keeps its conflicts for you to send with `Ctrl+R`. Every conflict leaves one line in `data/conflicts.jsonl` in [the home
folder](home-folder.md) once it is repaired: the task, the files, the minutes from found to up to date again, how often
the agent was sent back, and whether a new review followed (`crates/flow/src/sync/conflict_log.rs`). Only a task at rest
is ever updated: its worktree clean, on exactly the commit that was reviewed. A fetch that fails is never taken for a
sync: a finished turn is saved as "Could not synchronize main" instead of going on to review (and as "Task is on the
wrong branch" when its worktree has left the task's own branch). A waiting task whose main cannot be fetched, or that
was not at rest when main moved, is held as "behind main": its row says so, `Ctrl+L` is refused with the reason, and the
board tries again every minute until the branch really is up to date. "Main" is where the task will land: the remote's
main when it lands through a pull request; otherwise (no remote, or `pr` off) the branch your checkout was on when the
task first finished. For those, checking out another one merges nothing into waiting tasks, and they are held as "behind
main" until your checkout is back on it.

**A review that passed is not repeated for a merge of main.** When the last review of the work passed and the only
thing that changed since the commit it looked at is main being merged in, the board runs the repo's `check` on the
result instead of a full new review. Tests pass: the task goes on to approval or the merge, as a pass. Tests fail:
the failure goes back to the agent, as a failed check always does, and its fix is reviewed. The rule for "only the
merge of main" is fixed and checked in git, from the commit the reviewer looked at to the branch as it stands:

1. every commit since is a merge commit with two parents, the branch as it was and a commit of main. An ordinary
   commit on top, a rebase or another branch merged in is new work;
2. each of those merges is exactly what git makes of its two parents by itself. Only the files git could not merge
   (the conflicts) may differ, because resolving them is what the merge needed. One other file touched in the merge
   commit is new work.

So a conflict the agent resolved in its merge commit, touching only the conflicting files, keeps the review; the
agent is told this with the conflict. Anything else is reviewed as before, and so is everything the board cannot
read for sure: a last review that failed or gave no verdict, work outside a worktree. A repo with no `check` has
no tests to prove a merge into the task's own files, so there this rule does not apply: a review stands only when
a clean update touched none of the files the task changed (the narrower rule above), and the task then says `no
check is set` instead of `tests pass`. The task says plainly what happened: its message reads `Re-merged with main,
tests pass, review 2 still stands`, the box of that review carries the same line (with when, and how often), the
row keeps showing the review that passed (`Pass · 2nd review`), and `details` lists each time under that review as
`kept`. This is fixed behaviour, not a setting.

**Approving** makes that last sync, re-checks (and re-reviews, by the same rules) if it changed anything, then merges
through the pull request, only at the commit that was reviewed (a commit pushed to the pull request meanwhile
withdraws the approval; on a repo with a merge queue the task waits until GitHub says the pull request is merged) or
locally, squashed by default. A conflict found here goes back to the agent the same way, and the merge you approved is
retried after its repair. A checkout with unsaved files is never forced.

**A pull request merges only once its checks on GitHub pass** (`merge.checks`: `github`), by itself or on your yes, in
every repo the board merges in. Before the merge the board looks at them: still running, the task says how far they are,
`Waiting for GitHub checks · 2 of 3 passed` (each check counted once, a workflow for its jobs until GitHub lists them;
`details` names them: `checksWaitingFor`), has the board's own `◇` (nobody works on it meanwhile), and merges by itself
once they pass; failed, nothing merges, and the failure goes back to the agent like a failed check of the board's own,
with the failing check's name and the end of its log, within `review.rounds` (the merge you approved goes on once
GitHub's checks pass on the repair); after that it waits for you, and `approve <id>` says why and refuses, as for a
failed review, until `approve <id> --anyway`. Checks that never report, still running after `check.timeout`, leave it
waiting for you with that reason. Passed, or no checks at all: it merges as before. The board decides from what the repo
expects, not only from what GitHub lists so far. Each GitHub workflow of the branch that runs for this pull request
(`.github/workflows`, its triggers read as GitHub reads them: `push` and `pull_request`, their `branches` against the
branch, `paths` against the files the pull request changes, `types`) must have its run on the commit, and that run must
be through and passed: every job of it, every member of a matrix, also one GitHub has not listed yet (counted as one
check still to pass). A workflow that cannot run for it (only for other paths, only into another branch, only on tags)
is not waited for. Each check the branch it merges into requires (its protection or rulesets) must be listed and pass,
and a check listed on the commit before and gone from the list now has not passed. With nothing listed at all, the task
says `Waiting for GitHub checks to start` where an outside CI reports in the repo (on the newest commit of the branch it
merges into, or on its other pull requests); a question GitHub cannot answer counts as yes. All of these wait at most
`check.timeout`, then for you, with the reason. A repo with no checks at all merges with none listed. `merge.checks:
none` does not look at GitHub's checks.

**A fine-grained GitHub token is enough.** Such a token cannot have the Checks permission (GitHub offers it to GitHub
Apps only), so GitHub refuses it its list of a pull request's checks. The board then reads them from what the token
can read: **Actions** (the runs on the commit, each of their jobs and matrix members, and the log of a failed one) and
**Commit statuses** (an outside CI, Vercel). Give the token read access to those two; nothing asks for Checks, and a
refused list is no login problem. The same rules hold: running waits, a failure goes back with its log, a workflow
not started is waited for, all within `check.timeout`. What such a token cannot read at all is the check runs of
another app that posts no commit status: the task merges once everything it can read has passed and says what it
could not read (`Merged: … Not read: check runs of apps other than GitHub Actions …`); a check the branch requires and
no job or status of the commit answers is waited for while anything readable still runs, then named there too.
GitHub itself still refuses a merge that a required check blocks. A token that may not read Actions or Commit statuses
either leaves the task waiting for you with `Could not read GitHub's checks`, naming the two permissions to give it.
A check is named once whichever way it was read (a workflow `Check` and its job `check`, its runs for the push and for
the pull request), and what was remembered for an earlier commit, or read the other way, is not looked for again.

**When gh cannot read GitHub at all** (not logged in, the network), a merge that waits for its checks says so on its
row (`Waiting for GitHub checks · could not read them (…)`) and goes on by itself once gh reads again; past
`check.timeout` it waits for you with `Could not read GitHub's checks`. herdr may start the board's keeper (after a
reboot) without your login shell's environment: the board takes your PATH and gh's login variables (`GH_TOKEN`,
`GH_HOST`, `GH_ENTERPRISE_TOKEN`, …) from a login shell, never writing a token down.

**When a task is done** (merged here or on GitHub, finished with nothing to merge, or called done by you) its space
leaves herdr's sidebar by itself, and its worktree and branch are removed. Work is never lost for it: a checkout with
unsaved files, or with work that was never merged, stays on disk with its branch, the task says where, and a reply
continues there; a space you are using again (a turn in the agent's own terminal, a command running in any of its panes)
is not closed (that includes a command a finished agent left running in the background), and neither is one herdr could
not answer for or whose herdr session is not running (then nothing of the task is touched): the clean-up is tried again
by itself (about every half minute, board open or closed) and happens once the space is free, after a turn of the agent
only when that has been quiet for ten minutes. Work that appeared meanwhile is kept. An agent herdr cannot place (status
"unknown") counts as at work until its screen has stood still for two minutes and its saved conversation ends in a
finished turn and nothing runs under the agent program (an agent of unknown status that keeps helper programs running is
therefore never closed by itself: archive or remove its task). A reply to work that was cleaned up starts a new round in
a fresh worktree. herdr also opens the repo itself as a space when it makes a repo's first worktree; that one is closed
with the repo's last task space, unless it was open before or you use it (a second pane or tab, an agent, a running
command, or you are looking at it). `workspace.cleanup false` keeps everything.

**Agents run with full access by default**, as pi-work's did: `claude --dangerously-skip-permissions`,
`codex --dangerously-bypass-approvals-and-sandbox --no-daemon`. They can run anything as you without asking; the
rules and the review brief are guidance, not a sandbox. `worker.access ask` (and `reviewer.access`,
`coordinator.access`) starts a program without those flags, so it asks before it acts; pi cannot ask, and the value
is refused for it. So that a new
task does not wait behind a "trust this folder?" dialog, the plugin marks the task's repo as trusted in the agent's
own settings (`~/.claude.json`, `~/.codex/config.toml`) before starting it (only for a role with full access).
