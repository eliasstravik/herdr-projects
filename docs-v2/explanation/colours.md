# Why the board uses these colours

This page tells why the board takes Herdr's theme, why some states have colours of their own, and why the greys have
a floor. For the list of colours, see [colours](../reference/colours.md).

## The board follows Herdr's theme

The board draws everything in the theme that Herdr shows. Thus the board and Herdr never look like two different
programs.

Herdr does not give its theme to a plugin. Thus the board reads the same config file that Herdr reads, and resolves it
in the same way. If no theme is set, the board uses Herdr's default, Catppuccin. It does not use the colours of the
terminal, because Herdr next to it shows its default.

### When the two cannot match

With `herdr --remote`, the client of Herdr paints Herdr's screen. The client uses the config of the machine where the
client runs. Herdr does not send that config, or its theme, to the server where the board runs. Thus the board cannot
know the theme of the client. To make them match, see
[choose the board's theme](../how-to/colours-choose-a-theme.md).

The Herdr web UI does not read Herdr's theme. It has its own colours. The web board uses the colours of the web UI, so
that it matches Herdr on the web, not Herdr in a terminal.

## A status uses Herdr's own colour

The board draws a status in the colour that Herdr's sidebar and agent list use for that state, whatever else the theme
does. Thus "working" has the same colour on the board and in Herdr. The board has its own colours only for states that
Herdr does not have: under review, ready to approve, queued and to do.

## Why queued is pink

Pink is the colour furthest from all the other colours: amber works, red needs you, green is ready, teal is done, blue
is under review, grey is to do, purple is an automation.

Herdr's themes have no pink of their own. Only the `mauve` of Dracula is a pink, and in Dracula that is the colour of
automations. Thus the pink is one of the board's own pinks. The nearest colour to pink is red, the colour of "needs
you". The measure keeps the pink well away from the theme's red. A mark needs a contrast of at least 3 to 1, so the
pink must have at least that much against the background.

## Why automations are purple

Automations are not tasks. Nothing of theirs must look like a status. Herdr's `mauve` is the one colour of Herdr's
theme that no status uses. Thus the automations use it, and it changes with the theme as all other colours do.

Some themes have a `mauve` that does not work:

- The `mauve` of Vesper is its amber, the colour of "working".
- The `mauve` of Nord, Kanagawa and Rosé Pine is a dusty colour near the theme's greys.
- The `mauve` of Kanagawa Lotus is too near its text colour.

In these themes, the board uses a purple of its own. In the `terminal` theme, the board uses the terminal's own
magenta, so that it follows the terminal as the rest of that theme does. The web UI has no purple among its status
colours, so the web board uses a purple of its own. It is moved towards the text colour of the page, so that it is
easy to read on light and dark pages.

## Why the greys have a floor

The greys are Herdr's own (`overlay0`, `subtext0`), but not all of Herdr's themes keep them readable. The grey of Nord
for greyed-out words has a contrast of 1.7 to 1 against its background. Nine more themes are under 3 to 1.

Thus each grey has a floor. A grey under its floor moves towards the plain text colour only as far as the floor, so
that it keeps the tint of the theme. 4.5 to 1 is what WCAG asks of text, and 3 to 1 is what WCAG asks of a mark.

The background is the real background where the board knows it, because the board paints no background of its own.

The mark of a finished task (`✓`) used the faint attribute in an earlier version. On a light background, its contrast
fell to 1.7 to 1. Now the board makes the colour quieter by a measured amount.

## Why the `terminal` theme does not use the terminal's greys

In the `terminal` theme, the board does not know the colours behind the sixteen colours of the terminal. None of the
terminal's own ways to grey a word is safe:

- **"Bright black"** is nearly the background on many dark schemes. On a dark navy scheme, it made "to do" unreadable
  (1.3 to 1).
- **"White"** is nearly the background on light schemes.
- **The faint attribute** halves the text colour. A quiet scheme has no contrast to spare. On Solarized light (text
  4.1 to 1), faint text is 1.9 to 1.

If the terminal does not give its text colour and its background, the board uses the plain text colour for greyed
words. The terminal's plain text colour is the one colour that is known to be readable. A grey that must be dimmer than
text needs the text colour. A grey measured against the background alone was stronger than the real text on Solarized
light.

The selected row is the one place where the board cannot control the result. Its background in this theme is the
terminal's "bright black", as in Herdr's own sidebar. The board cannot know what is readable on that background.

## Why done is teal on the web board

The web UI has one colour for done, and that colour is the green of "ready to approve". If the web board used it, the
two states would have one colour. Thus the web board uses a teal of its own for done.

## Why the box has the colour of the recipient

The frame of the text box shows who gets your message, at a glance. An agent's box has the status colour of its task.
Thus the box also shows how the task is doing.

The box and the mark never have two different colours. Thus while the board runs its own check on a task, the box of
that agent is the plain light grey of the mark. That grey is near the plain text colour. On the web board, and with the
colours of the terminal on a dark terminal, it is the same as the plain text colour. For that time, only the words tell
it from the Coordinator's box.

A machine's coordinator and a repo's coordinator have the colour of their headings. The Coordinator keeps its plain
look. Thus you can tell the three apart at a glance.

## Colour is never the only clue

Every state has its own mark and its own words. With `NO_COLOR`, the board draws no colour at all, and it is still
easy to read. The tests measure that the colours are different from each other in every built-in theme.

## Related pages

- [Colours](../reference/colours.md)
- [Choose the board's theme](../how-to/colours-choose-a-theme.md)
- [The colour of the text box](../reference/colours-text-box.md)
- [Check progress](check-progress.md)
- [The web UI](web-ui.md)
