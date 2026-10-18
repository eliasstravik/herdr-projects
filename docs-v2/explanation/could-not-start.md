# When an agent could not start

This page tells why an agent can fail to start, what the board does to prevent it, and why. To fix a failed start,
read [fix an agent that could not start](../how-to/could-not-start-fix.md).

## The row tells the reason

If the agent of a task could not start, the row of the task tells why, in plain words, and what to do. You fix the
cause, then you delegate the task again. For an agent that the board starts again, you reply in place of a new
delegation.

The board also keeps the full account of the failed start. It is in the task panel and in the log of the task. For a
reviewer, it is in the review. For the Coordinator, it is in its own log
([where to look further](../how-to/could-not-start-fix.md#look-further)).

## Long rules and long messages

Two limits once caused long text to stop the start of an agent, with no reason. The board handles both limits now.

### Long rules go in a file

Herdr starts a program: it types the command of the program into the shell of the pane. The shell can still be in its
start (its line editor is not ready yet, because of a slow `.bashrc` or `.zshrc`). Then the terminal takes a maximum of
4096 bytes on one line. On macOS, the maximum is 1024 bytes.

Codex got the rules of a task in the command line, in `-c developer_instructions=…`. With your own
`worker.instructions`, the command became longer than the limit. The command arrived cut off. The shell waited at its
`>` continuation prompt, and the start timed out.

Now, rules longer than 300 bytes go in a file in the folder of the task (`rules.md`). The board tells Codex to read that
file and obey it for the full session. Claude Code, pi, Droid and the other programs always take their rules as a file
name. Thus their start command stays short. Only Codex takes its rules as text.

### The board waits for a shell that is not at its prompt yet

Before the shell shows its prompt, Herdr 0.9.5 refuses a line that is longer than the terminal takes ("the terminal is
in canonical mode with a 1024-byte limit; wait for the shell prompt and retry"). The first start of a new task failed in
this way on macOS.

Now the board waits until the pane shows its prompt, and tries again. It tries a maximum of 3 more times in
approximately 15 seconds. It does this for every start: the agent of a task, a reviewer, the Coordinator, and the picker
of a comparison. Before each try, the board looks for an agent in the pane. Thus it never starts a second agent.

### Very long messages go in a file

A message is one argument of `herdr agent prompt`. Linux passes a maximum of 128 KiB in one argument.

Thus the board writes a message longer than 100,000 bytes to a file in the folder of the board (`messages/`). Examples
are the first prompt of a task, a reply, and a message to the Coordinator. Then the board types a line that names that
file.

The board types a shorter message as it is. Messages of 4,000 and 100,000 characters went in complete, into Claude Code
and into Codex.

How the project found these limits: [facts checked on Herdr 0.9.3](../contributing/herdr-0.9.3-facts.md).

## Related pages

- [Fix an agent that could not start](../how-to/could-not-start-fix.md)
- [Troubleshoot the board](../how-to/troubleshoot.md)
- [How the board uses agent programs](agent-programs.md)
- [Agent programs](../reference/agent-programs.md)
- [Facts checked on Herdr 0.9.3](../contributing/herdr-0.9.3-facts.md)
