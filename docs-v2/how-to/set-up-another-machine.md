# Set up another machine

Use this page to show the tasks of another machine on your board, to check each machine, and to repair a machine
that does not answer. For how the board works with other machines, see
[Other machines](../explanation/other-machines.md).

## Before you start

The board finds machines only through Herdr. There is no Tailscale discovery. The board uses one of these:

- the machines that you added with `herdr machine add`;
- the machines that you name in `board.machines`.

If there are no machines, `Alt+M` tells you so. The board lists the machines in the order of the machines sidebar of
Herdr. See [Machine order](../explanation/machine-order.md).

## Install herdr-projects on the other machine

Do these steps one time on each other machine.

1. Install `git`, `gh` and the agent programs that the machine will run.
2. Run these commands:

   ```sh
   # git, gh, the agent programs it will run, then:
   git clone https://github.com/eliasstravik/herdr-projects && cd herdr-projects
   herdr plugin link "$PWD"
   sh bin/run.sh install             # gets the program and writes its launcher
   ```

3. Put `herdr-projects` on the PATH of a login shell (see the next section).

How the program is fetched and updated is in [Install and update](install-and-update.md) and
[Install details](../reference/install-details.md).

## Put `herdr-projects` on the PATH of a login shell

The board asks the other machine with `$SHELL -lc 'herdr-projects remote list'`. This is a login shell that nobody
types into, which is what SSH starts. It is not an interactive shell. A PATH that you set only for interactive shells
does not count.

**Note:** On many systems, `~/.bashrc` stops at its "if not running interactively, return" line. A PATH that you set
below that line is not found. A PATH set only in `~/.bashrc` is often not sufficient.

1. Do one of these two things on the other machine:
   - Set the PATH in the profile file: `~/.profile` or `~/.bash_profile` for bash, `~/.zprofile` for zsh.
   - Link the launcher into a folder that a login shell already has:
     `ln -s ~/.herdr-projects/data/bin/herdr-projects ~/.local/bin/`.
2. On the machine that the board is on, run this test:

   ```sh
   ssh <machine> 'exec $SHELL -lc "herdr-projects remote list"' | tail -c 200
   ```

3. Make sure that the output ends with a line of JSON.

If the output ends with a line of JSON, all is well. The start-up files of a shell can print text around that line:
a banner, a goodbye, or text on stderr. This does no harm, however much text it is, and also if that text is JSON.
The board reads the last line that is its own answer.

## Add the machine to Herdr

1. Add the machine with `herdr machine add`, or name it in `board.machines`.
2. On the board, press `Alt+M`.

Each press goes to the next view: this machine, every machine, then each other machine by itself.

## Check every machine

1. Run `herdr-projects machines`.

The command lists this machine and each other machine. For each one, it shows if the machine answers, its plugin
version and how many tasks it has. If a machine does not answer, it shows the reason and what to do. It reads the
answers with the same code as the board. For example:

```
vm-home  this machine: a login shell (/bin/bash) finds herdr-projects 0.2.0
vm-a     answers · herdr-projects 0.2.0 · 136 tasks  (exedev@vm-a)
vm-b     does not answer: herdr-projects is not found there  (exedev@vm-b)
    A login shell on vm-b does not find `herdr-projects`.
    …
```

For this machine, the command runs the same login shell with a bare environment: the account, its shell and a short
PATH. This is what SSH gives. Thus you can see on the machine itself if a different machine can ask it.

To get the same output as data, add `--json`.

## Repair a machine that does not answer

If a machine does not answer, the board shows one line in place of its tasks, with the reason. For example:

```
vm-a: its tasks cannot be read: herdr-projects is not found there.
```

To see more, press `Alt+M` until the view of that machine shows. The view shows what came back, what it means and
what to do. On the web board, this text is behind "Why, and what to do".

1. Find the text of the line in this table.
2. Do what the last column says.

| The line says | What happened | What to do |
|---|---|---|
| cannot connect (its name is not known, connecting timed out, it refused the connection, there is no way to it from here), ssh could not connect | SSH never reached the machine. | Make sure that the machine is on and on the network. Check its name and port in `herdr machine list` or `board.machines`. |
| its host key changed | The machine shows a different key than the key that the board saved for it. | If the machine was set up again, run `ssh-keygen -R <host> -f ~/.herdr-projects/data/known_hosts`. The board keeps its own list. With a port other than 22, the name is `'[<host>]:<port>'`, and the line of the board gives it in that form. |
| it does not let the board in | There is no key that works, or the machine asks for a password. The board runs SSH in batch mode and cannot type a password. | Run `ssh-copy-id <machine>`, or load your key into the SSH agent that the board runs under. |
| herdr-projects is not found there, node is not found there, its node is too old | A login shell there does not find the program (exit 127). Or it finds an older version that uses Node, and there is no Node or the Node is older than 22. | Repair the PATH of the login shell, as above. This version needs no Node. |
| it answered something else than the board's answer, it answered nothing, it answered with an error, it answered, but not with its tasks | The command ran but did not print the list of the board. The cause can be an older plugin, a start-up file that prints and stops, or an error. | Update herdr-projects there. Run the test above to see all of the output. |
| it took too long | No answer came within 20 seconds. | The board asks again by itself. Look for a start-up file that waits for something. |

If a machine on an older plugin answers, the board shows "update herdr-projects on <machine>". To repair this,
update herdr-projects on that machine.

## Related pages

- [Other machines](../explanation/other-machines.md)
- [Machine order](../explanation/machine-order.md)
- [Install and update](install-and-update.md)
- [Install details](../reference/install-details.md)
- [Settings](../reference/settings.md)
- [Troubleshoot](troubleshoot.md)
