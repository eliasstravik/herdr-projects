# Machine order

**The board lists machines in the same order as Herdr's machines sidebar** (the **machines** panel on the left of the
Herdr window). Reorder the sidebar and the board follows: another machine's order at its next answer (a few seconds),
this machine's own within a minute.

That order is used everywhere machines are listed:

- the machine headings in the list;
- the header's `all machines:` line;
- `Alt+M`'s round (this machine, every machine, then each other one);
- the same on the web board, and the web UI's Boards section, whatever order the web UI keeps its PCs in
  ([web UI](web-ui.md)).

## Where Herdr keeps the order

Herdr's sidebar shows **Local** first, then the saved machines that are turned on, in the order of its list of saved
machines.

- `herdr machine list --json` prints that list in order, and says which machine the window shows now (`selected`).
- Herdr has no command to change the order. A new machine goes to the end. A plugin such as Reorder Machines, or an
  edit of Herdr's list, moves it.
- The list lives on the computer the Herdr window runs on, which is often not the one the board runs on. With the
  Herdr window on a laptop showing a Mac mini's board, the laptop's order is the one that counts. The Mac mini's own
  list (in the order its machines were added) is a different one.

## Which sidebar the board follows

Every machine sends its Herdr list along with its tasks. The board picks, in this order:

1. **The sidebar of a machine whose Herdr window shows this machine now.** That machine is its own Local, so it comes
   first; this machine stands where that sidebar puts it.
2. **The one it followed before**, as long as that sidebar still has this machine (even if its window has since moved
   to another machine). The choice is saved with the board's view, so it holds after the board closes. Until that
   machine answers again, the board uses this machine's own list.
3. **This machine's own Herdr list**, with this machine (its Local) first.
4. **None**, when no list can be read (an older Herdr, or machines named by hand in `board.machines`): this machine
   first, then the others by name.

### How the board finds itself in another machine's list

This machine is the saved machine that connects to one of its names, in its own Herdr session:

- its host name (a `.local` name, as a Mac names itself, also without `.local`);
- an address of one of its network interfaces (a Tailscale address too);
- an SSH alias, read as the host it stands for in that machine's SSH config (`ssh -G`, which that machine reads before
  it sends its list).

A sidebar where this machine is not found that way, or is turned off, is not followed. A machine the board does not
know is never taken for this one.

The other machines are matched to the board's machines by where they connect (same host, port and Herdr session), or
else by name. Host names are compared whole: `build.prod.example` is never `build.dev.example`. Only a bare name
matches the first part of a whole one (`mac-mini` and `mac-mini.tail1234.ts.net`). This machine's bare names are only
the ones it really has, never cut from its whole name.

## Machines the sidebar does not show

A machine the board knows but the followed sidebar does not show (turned off there, or not saved there) comes after
the rest: this machine first, then by name.

A machine that does not answer stays where Herdr puts it, as Herdr keeps it in its sidebar too.

## A machine on an older program

A machine whose program is older than this feature (herdr-superwork 0.3.0, say) sends no Herdr list. The board then
asks that machine's Herdr itself, over the same SSH, at each of its answers (`herdr machine list --json`), and follows
its sidebar all the same.

- Only an SSH alias in that list is not resolved there. So the entry for this machine must use one of its own names or
  addresses. A newer program there resolves aliases.
- A machine whose Herdr gives no list (none found by a login shell there, or a Herdr older than saved machines) is
  never followed, and is placed like any other.
