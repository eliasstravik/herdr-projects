# When a task is done

This page tells how the board cleans up after a task, and why it never loses work. It also tells what the keeper
does while the board is closed.

## The clean-up

A task is done when it merged here or on GitHub, when it finished with nothing to merge, or when you called it done.
Then its space leaves the sidebar of Herdr by itself, and the board removes its worktree and its branch.
`workspace.cleanup false` keeps all of it.

## Work is never lost

The clean-up never removes work. These rules make sure of that:

- A checkout that has unsaved files, or work that was never merged, stays on disk with its branch. The task tells
  where it is, and a reply continues the work there.
- The board does not close a space that you use again. Examples are a turn in the agent's own terminal, or a command
  that runs in one of its panes. This includes a command that a finished agent left running in the background.
- The board touches nothing of the task if Herdr could not answer for the space, or if its Herdr session does not run.
- The board tries the clean-up again by itself, about every half minute, with the board open or closed. The clean-up
  happens when the space is free. After a turn of the agent, it happens only after ten quiet minutes. The board keeps
  work that appeared in the meantime.
- A reply to work that the board cleaned up starts a new round in a new worktree.

### What counts as a running command

An agent program keeps some things running as part of itself. These are not commands of the kind above:

- its MCP servers;
- its own program under an npm wrapper (`codex` installed with npm).

They are in the process group of the agent. The input and output of an MCP server go to the agent itself, through a
pipe or a socket of which the agent holds the other end. Thus the board closes a finished agent with MCP servers like
any other agent.

A command whose streams go to another place (the terminal, a file, `/dev/null`) keeps the space open. This is true in
the group of the agent and outside it. A command in a group of its own also keeps the space open. If the board cannot
tell, it counts the process as a command.

### An agent of unknown status

Sometimes Herdr cannot place an agent (status "unknown"). Such an agent counts as at work until all of these are true:

- its screen stood still for two minutes;
- its saved conversation ends in a finished turn;
- nothing runs under the agent program but its own helpers (above).

Thus the board never closes by itself an agent of unknown status that keeps a command of its own running. Archive or
remove its task.

### The space of the repo

Herdr also opens the repo itself as a space when it makes the first worktree of a repo. The board closes that space
with the last task space of the repo. It does not close it if it was open before. It also does not close it if you use
it: a second pane or tab, an agent, a running command, or you look at it.

## With the board closed: the keeper

The keeper (`herdr-projects keeper`) is a small background process. It does what the tick of the open board does:

- it polls pull requests;
- it starts tasks in the queue;
- it runs the Watcher;
- it keeps the sidebar of Herdr current.

### The sidebar

An entry of an agent in the sidebar is a small copy of the row of its task on the board
([Herdr's sidebar](../reference/sidebar.md)). The entry shows these:

- the number and the title of the task;
- who works on it and where it is, in the board's own words (one list: `crates/core/src/status_words.rs`), with the
  review round and the pull request;
- the percent and the time to go of the row (`8% 16m`).

The title of the pane names the task in the same way, so the sidebar of the web UI reads the same. A review that runs
has two entries, one for the agent and one for the reviewer: they are two agents. The sidebar shows only tasks with a
live agent pane. The other tasks (in the queue, or your own) are only on the board.

### No notifications

The keeper shows no pop-up and makes no sound. What needs you is on the board and in the sidebar of Herdr.

### The life of the keeper

- Herdr starts the keeper with its server.
- If the keeper stopped, any Herdr event or opening the board starts it again.
- The keeper replaces itself when the plugin is updated.
- It stops when Herdr is gone.
- It looks after the tasks of this machine only.

An open board does the same. After an update, it loads the new version by itself, in place, when nothing is half
typed.

## Related pages

- [How work gets done](how-work-gets-done.md)
- [Herdr's sidebar](../reference/sidebar.md)
- [The Watcher](watcher.md)
- [The home folder](../reference/home-folder.md)
