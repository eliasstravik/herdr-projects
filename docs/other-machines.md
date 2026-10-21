# Other machines

**Every machine keeps its own tasks, and `Alt+M` shows the others' tasks on your board.** Press it to go round: this
machine, every machine, then each other machine by itself. Actions on another machine's task run on that machine, over
SSH. Work never moves between machines.

Which machines you see is the first thing in the board's header, in the accent colour, and is never cut:
`all machines: otherbox`, or `only otherbox`.

## Set up another machine

Machines are found through Herdr only: the ones you added with `herdr machine add`, or the ones you name in
`board.machines`. There is no Tailscale discovery. They are listed in the order of Herdr's machines sidebar
([machine order](machine-order.md)).

With no other machine there is no "every machine" to show, on the terminal board and on the web: `Alt+M` asks Herdr
for its machines (`Looking for other machines…`), says when there are none, and shows every machine as soon as there
is another one. A board that showed every machine goes back to this machine's own when the others are gone.

Each other machine needs, once:

```sh
# git, gh, the agent programs it will run, then:
git clone https://github.com/eliasstravik/herdr-projects && cd herdr-projects
herdr plugin link "$PWD"
sh bin/run.sh install             # gets the program and writes its launcher
```

…and `herdr-projects` on the PATH of a login shell there (next section). How the program is fetched and updated is in
[installing](install.md).

Why SSH: the plugin's commands on another machine travel over the plugin's own SSH connection, because Herdr has no
command that runs a plugin's commands there.

### `herdr-projects` must be on a login shell's PATH

The board asks another machine with `$SHELL -lc 'herdr-projects remote list'`: a login shell that nobody types into, not
an interactive one. A PATH set only for interactive shells does not count. On many systems `~/.bashrc` stops at its "if
not running interactively, return" line, so a PATH set below that line is not found.

Fix it either way:

- set the PATH in the profile file: `~/.profile` or `~/.bash_profile` for bash, `~/.zprofile` for zsh,
  `~/.config/fish/config.fish` for fish (outside its `if status is-interactive` block);
- or link the launcher into a folder a login shell already has:
  `ln -s ~/.herdr-projects/data/bin/herdr-projects ~/.local/bin/`.

Any login shell does, fish too: the board hands it only words every shell reads alike, and lets `/bin/sh` start it
with the command.

Test it from the machine the board is on. All is well when it ends with a line of JSON:

```sh
ssh <machine> 'exec $SHELL -lc "herdr-projects remote list"' | tail -c 200
```

Anything a shell's start-up files print around that line (a banner, a goodbye, anything on stderr) does no harm, however
much and even if it is JSON itself: the board reads the last line that is its own answer.

## Check every machine: `herdr-projects machines`

It lists this machine and each other one: whether it answers, its plugin version and how many tasks it has, or the
reason and what to do. It reads the answers with the same code as the board.

```
vm-home  this machine: a login shell (/bin/bash) finds herdr-projects 0.2.0
vm-a     answers · herdr-projects 0.2.0 · 136 tasks  (exedev@vm-a)
vm-b     does not answer: herdr-projects is not found there  (exedev@vm-b)
    A login shell on vm-b does not find `herdr-projects`.
    …
```

For this machine it runs the same login shell with a bare environment (the account, its shell and a short PATH, which is
what SSH gives). So you can see on the machine itself whether another one could ask it. `--json` gives the same as data.

## When a machine does not answer

It is never shown as a machine with no tasks. Where its tasks would be, on the terminal board and the web board, one
line says so, with the reason:

```
vm-a: its tasks cannot be read: herdr-projects is not found there.
```

- In the every-machine view, the other machines' tasks still show under it.
- On that machine's own view (`Alt+M` goes round to it, even while it does not answer) you get the rest: what came back,
  what it means and what to do. The web board keeps that behind "Why, and what to do". If it is longer than the screen,
  the arrows scroll it and a line says there is more.
- The line stays put while the board asks again every half minute. It is never repeated as a message.
- An action on a task of a machine that stopped answering says why, in its one message.

| The line says | What happened | What to do |
|---|---|---|
| cannot connect (its name is not known, connecting timed out, it refused the connection, there is no way to it from here), ssh could not connect | SSH never reached the machine | is it on and on the network? Check its name and port in `herdr machine list` or `board.machines` |
| its host key changed | the machine shows another key than the board saved for it | if it was set up again: `ssh-keygen -R <host> -f ~/.herdr-projects/data/known_hosts` (the board keeps its own list; with a port other than 22 the name is `'[<host>]:<port>'`, and the board's line gives it that way) |
| it does not let the board in | no key that works, or it asks for a password: the board runs SSH in batch mode and cannot type one | `ssh-copy-id <machine>`, or load your key into the SSH agent the board runs under |
| herdr-projects is not found there, node is not found there, its node is too old | a login shell there does not find the program (exit 127), or finds an older Node-based version of it, with no Node there or one older than 22 | fix the login shell's PATH, as above; this version needs no Node |
| it answered something else than the board's answer, it answered nothing, it answered with an error, it answered, but not with its tasks | the command ran but did not print the board's list: an older plugin, a start-up file that prints and stops, an error | update herdr-projects there; run the test above to see all of it |
| it took too long | no answer within 20 seconds | the board asks again by itself; look for a start-up file that waits for something |

## What you see of another machine

- **Headings.** With every machine shown, each machine is a heading of its own, with its repos (or status groups) under
  it. A row has its plain tag (`#1`); the heading above names its machine ([list headings](list-headings.md)).
- **Automations.** A machine's [automations](automations.md) come with its tasks and stand in its part of the list, as
  its tasks do: a group of their own after its tasks' (by repo: under their repo), folding with its heading. Turning them
  on and off, firing, removing and changing their settings all happen on the machine that has them.
- **The chat.** For each task someone is on, its machine sends the last few things said in the conversation: its
  agent's, or its reviewer's during a review, read there from the program's saved conversation. The chat and task panel
  show them like this machine's own, under the same lines that name who said them ([the board](board.md)).
- **Its Coordinator.** While the board shows one other machine by itself, what you type goes to that machine's
  Coordinator, and the chat shows that Coordinator's conversation. The machine sends where its Coordinator stands
  (`idle`, `working`, how many of your messages wait) and the last dozen things said. The box shows them under the same
  lines (`Coordinator:`, `You:`) and scrolls back through them. A machine with no Coordinator says so (what you type
  there is added as a task).
- **A machine on an older plugin** is recognised by its missing version: its new fields stay blank and the board says
  "update herdr-projects on <machine>". It sends no chat: its agent's last message still shows, and for a reviewer or
  its Coordinator the box says the machine needs the update.

## Going into another machine's agent

`Enter` on another machine's task first has that machine reopen the task's space and restart an exited agent, in the
same conversation.

- **In a Herdr window**, that machine's Herdr puts the agent's pane in front, as `Enter` does for an agent on this
  machine. The board stays and says: `vm-a: its agent is in front there. Pick vm-a in herdr's sidebar to go to it: “<its title>”; Local there brings you back here.` Herdr has no command that switches a window to another machine, so
  that one click in its machines sidebar is yours. The agent is there when you do.
- **Coming back:** **Local** in the sidebar (or `prefix+w` and a space of this machine) brings you back to the board,
  as you left it. The board's own key does not, while that machine is shown: Herdr runs it there, so it opens that
  machine's board.
- **Outside a Herdr client** (plain SSH), or with a machine on an older plugin, your terminal attaches to that one
  agent instead (`herdr agent attach`). `ctrl+b q` returns to the board.

## A task's work settings travel with it

When you put an agent on another machine's task (or send that machine words that add one), your machine works out what
you set for it (you, and your entry for its repo) and sends that along. It is kept on the task as what it carried,
apart from anything else the task tracks. There it stands where that machine's own "you" would: under the task's own
settings and above the repo's file. `show <id>` there says `sent from <your machine>`.

Board settings (how many agents at once, the Watcher, the Coordinator) stay with each machine ([settings](settings.md)).

## Files you attach go with your words

Words for another machine's agent or Coordinator may name a file of this machine: one you attached (`@<path>`), or one
you dropped in the terminal (`/Users/you/Desktop/Screen\ Shot.png`, or in quotes), or a picture you pasted, which the
box shows as `[Image 1]` ([pictures pasted](image-paste.md)). Before the words go, the board sends
each such file over its own SSH link to that machine, and the words name it where it landed there. You copy nothing.

- **Where it lands:** for a task's agent, in that task's working folder there, under `.herdr-projects/files/`. For a
  Coordinator, in its own folder there, the same way. With no Coordinator there (the words become a task), in that
  machine's board folder, under `files/`. Git leaves the folder alone. A file kept there goes after 30 days.
- **What goes:** only a file that is on this machine. A folder, a path that is no file here, and a bare path into a
  system folder or a hidden one (`/etc/hosts`, `~/.zshrc`) are left as you typed them: they mean that machine's own.
- **Up to 8 MB a file.** A larger one is not sent.
- **When a file cannot go** (too large, the machine does not answer, it refuses), the board says why and sends none of
  your words: never a path the agent there cannot open.
- **A machine on an older version** cannot take files: the board says so. Update `herdr-projects` there.

## In the Herdr web UI

The web Board shows other machines as `Alt+M` does, acts on their tasks, and opens their agents in the web UI's own view
of that machine, when the web UI has that machine as one of its PCs. The terminal board works in the web UI's terminal
view too, on this machine and on another. What works how, and what the board says when something is missing:
[the board in the Herdr web UI](web-ui.md).
