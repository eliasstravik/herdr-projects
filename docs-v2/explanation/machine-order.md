# Machine order

This page tells how the board puts machines in order, and which Herdr sidebar it follows. The board lists machines
in the same order as the machines sidebar of Herdr: the **machines** panel on the left of the Herdr window.

## The board follows the sidebar

If you change the order in the sidebar, the board follows. The order of a different machine changes at its next
answer, in a few seconds. The order of this machine changes within a minute.

The board uses that order everywhere it lists machines:

- the machine headings in the list;
- the `all machines:` line of the header;
- the round of `Alt+M`: this machine, every machine, then each other machine;
- the same items on the web board, and the Boards section of the web UI, whatever order the web UI keeps its PCs in.
  See [The web UI](web-ui.md).

## Where Herdr keeps the order

The Herdr sidebar shows **Local** first. Then it shows the saved machines that are switched on, in the order of its
list of saved machines.

- `herdr machine list --json` prints that list in order. It also says which machine the window shows now
  (`selected`).
- Herdr has no command to change the order. A new machine goes to the end. A plugin such as Reorder Machines, or an
  edit of the Herdr list, moves it.
- The list is on the computer where the Herdr window runs. This is often not the computer where the board runs. For
  example, a Herdr window on a laptop can show the board of a Mac mini. Then the order of the laptop is the one that
  counts. The list of the Mac mini itself, in the order that its machines were added, is a different list.

## Which sidebar the board follows

Each machine sends its Herdr list with its tasks. The board picks a sidebar in this order:

1. **The sidebar of a machine whose Herdr window shows this machine now.** That machine is its own Local, so it comes
   first. This machine is where that sidebar puts it.
2. **The sidebar that the board followed before**, if that sidebar still has this machine. This applies also if its
   window has since moved to a different machine. The board saves the choice with its view, so the choice stays after
   the board closes. Until that machine answers again, the board uses the list of this machine.
3. **The Herdr list of this machine**, with this machine (its Local) first.
4. **None**, if the board can read no list. This occurs with an older Herdr, or with machines that you named by hand
   in `board.machines`. Then this machine is first, and the other machines follow by name.

### How the board finds this machine in the list of another machine

This machine is the saved machine that connects to one of its names, in its own Herdr session. The names are:

- its host name (a `.local` name, as a Mac names itself, also without `.local`);
- an address of one of its network interfaces (also a Tailscale address);
- an SSH alias. The board reads it as the host that it stands for in the SSH config of that machine (`ssh -G`). That
  machine reads the alias before it sends its list.

The board does not follow a sidebar where it does not find this machine in this way. It also does not follow a
sidebar where this machine is switched off. The board never takes a machine that it does not know for this machine.

### How the board matches the other machines

The board matches the other machines to its own machines by where they connect: the same host, port and Herdr
session. If that does not match, it matches them by name.

- The board compares host names as whole names. `build.prod.example` is never `build.dev.example`.
- Only a bare name matches the first part of a whole name. For example, `mac-mini` matches
  `mac-mini.tail1234.ts.net`.
- The bare names of this machine are only the names that it really has. The board never cuts them from its whole
  name.

## Machines that the sidebar does not show

A machine can be known to the board but not shown in the followed sidebar. It can be switched off there, or not
saved there. Such machines come after the others: this machine first, then the rest by name.

A machine that does not answer stays where Herdr puts it. Herdr also keeps it in its sidebar.

## A machine on an older program

A machine with a program that is older than this feature (for example, herdr-superwork 0.3.0) sends no Herdr list.
The board then asks the Herdr of that machine itself, over the same SSH, at each of its answers
(`herdr machine list --json`). It follows that sidebar in the same way.

- Only an SSH alias in that list is not resolved there. Thus the entry for this machine in that list must use one of
  the names or addresses of this machine. A newer program there resolves aliases.
- The board never follows a machine whose Herdr gives no list. This occurs if a login shell there finds no Herdr, or
  if the Herdr there is older than saved machines. The board places such a machine like any other.

## Related pages

- [Other machines and several boards](other-machines.md)
- [Set up another machine](../how-to/set-up-another-machine.md)
- [List headings](../reference/list-headings.md)
- [The board header and list](../reference/board-header-and-list.md)
- [The web UI](web-ui.md)
