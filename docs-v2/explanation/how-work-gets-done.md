# How work gets done

This page tells how a task goes from "delegated" to "merged", and why each step is there. For the steps that you do,
see [delegate a task and merge its work](../how-to/delegate-and-merge.md).

## The path of a task

You delegate a task. An agent works on it in its own worktree. When the agent says that it is done, the board checks
the work and opens a pull request. Then a second agent reviews it, and the board waits for your yes. After your yes,
the board merges the work and cleans up.

```
delegate ─► agent works ─► finish line: commit, catch up with main, check ─► pull request ─► review ─► your yes ─► merge ─► clean-up
                 ▲                                    │                                         │
                 └──────── a failed check, a failed review or a conflict goes back ◄────────────┘
```

All of this runs when the board is open and when the board is closed. When the board is closed, the keeper does the
work ([when a task is done](how-work-gets-done-clean-up.md#with-the-board-closed-the-keeper)).

## Delegating

### In a git repo

The plugin asks Herdr for a worktree space (`herdr worktree create`). The space has its own branch
(`work/<n>-<title>`). A task in a repo never works in your checkout.

The branch starts from one of these:

- the default branch of the remote (this is the default);
- your commit, with `--workspace.from here`;
- another branch at the remote, with `--workspace.from develop`.

Then `workspace.setup` runs. Then the agent starts in the root pane of that space, with the task and its rules.

### The name of the branch

The setting `workspace.branch` gives the name of the branch. Its default is `work/{n}-{slug}`. You can set it for you,
for one repo or for one task. You cannot set it in the repo's own file.

| Placeholder | Becomes | Example |
| --- | --- | --- |
| `{n}` | the number of the task (an attempt has a letter) | `7`, or `7b` for an attempt |
| `{slug}` | the title of the task | `fix-the-login` |
| `{key}` | the key of its repo and its number | `KIC-7` |

For example, `feat/{key}-{slug}` makes `feat/KIC-7-fix-the-login`.

- If git does not accept a name, the board refuses it. It tells you the nearest name that git accepts.
- If a name is already in use, the board adds `-2` to it.

### A task started from a named branch

You can start a task from a named branch (`workspace.from develop`). This is for a long branch with small tasks on
top of it. The task then lands on that branch too.

- The board fetches the branch from origin at every start. The fetch takes 10 seconds at most and never asks for a
  password.
- If origin does not have the branch, the board refuses it and gives the reason. The task waits as failed.
- If the board cannot reach origin, it uses the copy that origin gave last.
- The task stays current with that branch. Its pull request merges into it (`gh pr create --base develop`).
- The board takes on a pull request that the agent opened itself only while that pull request merges into the branch.
- `merge.checks` reads the checks of a pull request into that branch.
- It needs `merge.pr` on. A local merge lands on the branch of your checkout. Thus the board refuses a named branch
  with `merge.pr` off, at the start and at the merge. This is so for now.

### Outside git

Outside git, the agent works in place, in a plain Herdr space.

### How many agents work at once

At most `board.agents` agents work at the same time. The default is 4. More tasks wait for their turn. A task that
waits for you does not use a slot.

## Agents run with full access by default

The board starts the agent programs with these flags:

```
claude --dangerously-skip-permissions
codex --dangerously-bypass-approvals-and-sandbox --no-daemon
```

Codex gets `--no-daemon` only from Codex 0.156.0, the first version that has it. An older Codex quits on it. The
agents of pi-work worked the same way.

**CAUTION:** An agent with full access can run any command as you, and it does not ask first. The rules and the
review brief are guidance. They are not a sandbox.

- `worker.access ask` starts the program without those flags, so it asks before it acts. `reviewer.access` and
  `coordinator.access` do the same for their roles. pi cannot ask, so the board refuses this value for pi.
- The plugin marks the repo of the task as trusted in the agent's own settings (`~/.claude.json`,
  `~/.codex/config.toml`) before it starts the agent. Thus a new task does not wait behind a "trust this folder?"
  dialog. The plugin does this only for a role with full access.

## How the board knows that a turn ended

The plugin starts every agent itself. At the start, it gives the agent these:

- **A turn-end hook** on the command line. For Claude Code, it is `--settings <a file in the task's own folder>`. For
  Codex, it is `-c notify=[…]`. For pi, it is `-e <a one-file extension>`.
- **A session id** that the plugin chooses, if the program accepts one.
- **The rules of the task** as a standing system prompt. This is a file, or, for Codex, inline text when the rules are
  short ([long rules and long messages](could-not-start.md#long-rules-and-long-messages)).

The plugin writes nothing to your own agent settings, and your own hooks continue to run.

At the end of each turn, the hook runs `herdr-projects turn-ended`. This command is silent, takes two seconds at most,
and a secret for each launch protects it. Then the finish line reads the end of the saved conversation of that agent.
It reads the last message, an API error or a login error, and the token use.

**An agent that stopped to wait for you has not finished.** This is true for a question, a permission and a sign-in.
It does not change if the agent asks many times. None of the finish line runs. See
[an agent that waits for you is not finished](waiting-for-you.md).

### Without a hook

Sometimes there is no hook. These are the cases:

- the program crashed;
- the program is older than the minimum version of its adapter;
- the agent is of another kind;
- you set your own `notify` in Codex;
- the hook failed three turns in a row.

Then the status from Herdr is only a guess from the screen, so "idle" alone never finishes a task. The turn counts as
ended when one of these is true:

- the saved conversation ends in a finished message or in an error;
- the agent wrote a closing note (`closing-note.md`) in the task's own folder
  ([where it is](../reference/home-folder.md#files-that-an-agent-writes-for-the-board)). The rules still ask for
  one, as the fallback.

If neither is true, the board holds the task as "Stopped without a summary". `Ctrl+R` tells the agent to continue.
`Ctrl+L` takes the work as it is.

Sometimes the board does not see a finish: the agent is at rest, but the row still says "Finishing". Finish such a
task with `Ctrl+L` or `herdr-projects finish <id>`. The last thing that the agent said becomes the summary.

## The finish line

When the turn of the agent ends, the board does these steps:

1. **It commits** the work that is left over. **It brings the branch up to date with main** in the task's own
   worktree ([staying current with main](how-work-gets-done-main.md)). Then **it runs the `check` of the repo**. A
   failure goes back to the agent at once, for three tries.
2. **It pushes the branch and opens a draft pull request**, if there is an `origin` remote and `gh`. See
   [the pull request](#the-pull-request).
3. **It has a second agent review the work**, adversarially. See [the review](#the-review).
4. **It waits for your approval** (`Ctrl+L`).

### The pull request

- If the agent already opened a pull request for the branch of the task, the board takes on that pull request. It
  goes through the review and the merge as usual.
- A branch that brings nothing to main gets no pull request. Either no change was necessary, or the work is on main
  already: the agent merged its own pull request, or pushed to main itself.
- Such a task is done, and its row says why: "No change needed", "Already on main" or "Already merged in #133".
- The board pushes nothing then. Thus the board never pushes to a repo that the task archived on GitHub.
- Work that is not on main is never done. If the repo is archived, the task stops and says so.

### The review

The reviewer works in its own tab in the same Herdr space ([a task's tabs](../reference/tabs.md)). It tells what it
found, with evidence. The board puts back the changes that the reviewer made.

- When the repo has a `check`, the board tells the reviewer the command, and that it passed on this commit. The
  reviewer must run tests only through it.
- If the reviewer ends its turn with no verdict, the board starts it once more, as the same review. If it again gives
  no verdict, the review fails, with its last message.
- Only the end of the reviewer's own turn counts. See [helpers of an agent](#helpers-of-an-agent).
- If the provider cut off the turn of a reviewer, the board tries it again
  ([a turn cut off by the provider](provider-errors.md)).
- The board decides from the findings ([the gate](gate.md)). A review that fails goes back to the agent at once.
- The row shows the result and a tally (`Pass · 2 minor`). Its tag `R#2` tells which review it was. The colour of the
  tag tells how it went. The task panel has the summary of the review, and every earlier review of the task.

### Helpers of an agent

An agent can give part of its work to a helper: the sub-agents of Codex, or the background agents of Claude Code.
When a helper finishes, the review does not end. While a helper still works, the review continues. The same is true
for a working agent and its task.

A helper is back when one of these is true:

- its answer is in the conversation (a message of its own, or a queued message that a turn in progress took in);
- its own saved conversation ends in its finished reply.

## Approving and merging

Your yes makes one last sync with main. If the sync changed something, the board checks the work again. It also
reviews it again, by the same rules. Then it merges in one of two ways:

- **Through the pull request**, only at the commit that the reviewer saw. If someone pushes a commit to the pull
  request in the meantime, the approval is withdrawn. On a repo with a merge queue, the task waits until GitHub says
  that the pull request is merged.
- **Locally**. The default is a squash merge.

If the board finds a conflict here, the conflict goes back to the agent in the same way. After the repair, the board
tries the merge that you approved again. The board never forces a checkout that has unsaved files.

With `merge.checks: github`, the board merges a pull request only after its checks on GitHub pass. See
[GitHub's checks](how-work-gets-done-github-checks.md). To merge without your yes, see
[merging by itself](gate.md#merging-by-itself).

## Stepping in

You step in when you type in the agent's own terminal, or reply from the board. Then all of the finish line starts
again:

- your approval is withdrawn;
- a pending auto-merge is cancelled (if the cancellation fails, the board saves it as a visible failure);
- the old review is marked stale.

A chat with work that was reviewed is the exception. What a chat starts again counts only after the code changed. See
[chatting with a finished task](how-work-gets-done-chat.md).

## Related pages

- [Delegate a task and merge its work](../how-to/delegate-and-merge.md)
- [Your first task](../tutorials/first-task.md)
- [Staying current with main](how-work-gets-done-main.md)
- [GitHub's checks](how-work-gets-done-github-checks.md)
- [Chatting with a finished task](how-work-gets-done-chat.md)
- [When a task is done](how-work-gets-done-clean-up.md)
- [The gate](gate.md)
- [An agent that waits for you is not finished](waiting-for-you.md)
- [Handing text to an agent](handing-over.md)
