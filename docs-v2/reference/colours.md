# Colours

This page lists the colours that the board uses and what each colour means. For the reasons behind these colours,
read [why the board uses these colours](../explanation/colours.md).

## What each colour means

The board uses the theme of Herdr. The colour of a task shows its state. The frame of the text box has the colour of
the agent or Coordinator that you type to.

| Colour | Meaning |
|---|---|
| amber (Herdr's `yellow`) | An agent works. |
| red | The task needs you (blocked or failed). |
| green | Ready to approve. |
| blue | Under review. |
| teal | Done. |
| pink | Queued: the task waits for a free slot, or for other tasks. |
| dim grey | To do. |
| light grey | The board runs its own check. |
| purple | An automation. |
| plain text colour (the box only) | You type to the Coordinator. |
| the colour of a machine's heading (the accent of the theme) | A machine's coordinator, in the chat. |
| the grey of a repo's heading | A repo's coordinator, in the chat. |

Colour is never the only clue. Each state also has its own mark and its own words. With `NO_COLOR`, the board is still
easy to read.

## Where the theme comes from

The board reads the `[theme]` and `[ui]` tables of `~/.config/herdr/config.toml` on this machine. It resolves them as
Herdr resolves them:

- the name of the theme, and its other names;
- `auto_switch`, with `dark_name` and `light_name`;
- `[theme.custom]`;
- the older `ui.accent`.

| Case | What the board does |
|---|---|
| No theme is set | It uses Herdr's default theme, Catppuccin, as Herdr shows it. It does not use the colours of the terminal. |
| `auto_switch` is set | It asks its terminal if the terminal is light or dark, as Herdr does. Then it uses the light or the dark theme. |
| The `terminal` theme | It uses the sixteen colours of the terminal, as Herdr does. The greys are different (see [the `terminal` theme](#the-terminal-theme)). |
| You change the theme in Herdr | The board shows the new theme the next time it opens. |

The popup, the task panel, the reply view and the review page use the same theme. This board draws the tasks of
another machine in the theme of this machine.

### The `board.theme` setting

The setting [`board.theme`](settings.md) selects the theme by hand. To change it, see
[choose the board's theme](../how-to/colours-choose-a-theme.md).

| Value | Theme |
|---|---|
| `herdr` | The default. The board follows Herdr, as this page says. |
| `dark` | The dark theme of the current theme. |
| `light` | The light theme of the current theme. |
| a theme name | One of Herdr's themes, by name. |

The theme names are: `catppuccin`, `catppuccin-latte`, `terminal`, `tokyo-night`, `tokyo-night-day`, `dracula`,
`nord`, `gruvbox`, `gruvbox-light`, `one-dark`, `one-light`, `solarized`, `solarized-light`, `kanagawa`,
`kanagawa-lotus`, `rose-pine`, `rose-pine-dawn`, `vesper`.

### When the board cannot know the theme

- **Herdr opened from another machine (`herdr --remote`).** The client of Herdr paints Herdr's screen with the config
  of the client's machine. If that config names a different theme, Herdr shows that theme around a board in the theme
  of this machine. A plain `ssh` to this machine, with Herdr started on this machine, is not this case. The board then
  has the same theme as Herdr.
- **The web board.** The Herdr web UI does not read Herdr's theme. The web board uses the colours of the web UI (its
  CSS variables, light and dark, and its palettes). See [the web UI](web-ui.md).

## Status colours

A status has the colour that Herdr's sidebar and agent list use for that state:

| State | Colour |
|---|---|
| working | Herdr's `yellow` (amber) |
| blocked or failed | Herdr's `red` |
| done | Herdr's `teal` |

The mark, the heading of the group, the bar, the percent and the minutes left use this colour.

The board has its own colours only for states that Herdr does not have:

| State | Colour | Which colour |
|---|---|---|
| under review | blue | The theme's `blue`, if it is different from all the colours near it. If not, the board's `#3b82f6` ([marks and colours on a row](board-marks-and-colours.md#the-blue)). |
| ready to approve | green | The theme's `green`. |
| queued: waits for a free slot, or tabbed and waits for other tasks to be done | pink | One of the board's own pinks (see [the queued pink](#the-queued-pink)). |
| to do | dim grey | The theme's grey for greyed-out words (`overlay0`), made readable (see [the greys](#the-greys)), if it is different from the green and the blue. If not, another grey. With the colours of the terminal, a grey that the board mixes from the background and the text colour of the terminal, and measures. |

- In every built-in theme, these colours are different from each other. The tests measure this.
- "To do" is dim grey in the list (the heading, the `○` mark, the task number) and in the box. It is dimmer than plain
  text, and always readable.
- While the board runs its own check, the mark is a plain light grey, the theme's `subtext0`
  ([check progress](../explanation/check-progress.md)).

## The queued pink

Pink is used only for a queued task. A task is queued when you start it while every slot of `board.agents` is taken.
The pink is used for:

- its `◌` mark;
- its place in the queue (`Q#2`);
- the State line in the task panel;
- the text box of its agent.

A task tabbed with `--after` is also queued, in the same pink, while the tasks that it waits for are not all done. Its
`◌`, its State line, its box and its line `queued after SKL-6` are pink ([one task after another](../explanation/after.md)).
A task that waits for other tasks, but that you did not tab, stays the dim grey of "to do". Its row does not show the
wait.

The pink is the first of the board's pinks that meets these rules:

- It is different from every other status colour, the purple of automations, plain text, both greys and the background.
- Its contrast against the background is at least 3 to 1.
- It is well away from the theme's red.

| Theme | The pink |
|---|---|
| Nord, One Dark, Solarized, Kanagawa, Vesper | `#f472b6`, the first pink for a dark background |
| Tokyo Night, Gruvbox, Rosé Pine | `#ff6ec7`: `#f472b6` is too near one of their colours |
| Catppuccin (Herdr's default), Dracula | `#ec4899`: the two brighter pinks are too near their red, which is pink-ish |
| Catppuccin Latte, Gruvbox light, One Light, Rosé Pine Dawn | `#db2777`, darker, for a light background |
| Tokyo Night Day, Kanagawa Lotus | `#c2188b` |
| Solarized Light | `#9d174d` |
| `terminal` | `#ec4899`: the sixteen colours of the terminal have no pink, and their magenta is for automations. A middle pink is easy to read on a dark terminal and on a light terminal. |

On the web board, the pink is the board's own `#ec4899`, moved towards the text colour of the page, as the purple is.

Other clues: the `◌` is a dotted ring that no other state has. `Q#2`, `Waiting for a free slot` and
`queued after SKL-6` are words. With `NO_COLOR`, they still show the state.

## The automation purple

Automations have one colour of their own. It is used for:

- the heading `automations` in the list;
- the mark of each automation row (`⟳`, dimmed while the automation is off);
- the `[auto]` tag of a task that an automation made;
- the line of the automation in the `?` menu.

The colour is Herdr's own `mauve` where it is a purple (from violet to pink) and different from the status colours
(red, green, amber, teal, blue, grey), from plain text, from both greys of greyed words and from the background. The
tests measure this in every built-in theme.

| Theme | The automations' colour |
|---|---|
| Catppuccin (Herdr's default), Latte, Tokyo Night and Day, Dracula, Gruvbox dark and light, One Dark and Light, Solarized dark and light, Rosé Pine Dawn | the theme's `mauve` |
| Nord, Kanagawa, Rosé Pine, Vesper | the board's `#a855f7`: the `mauve` of Vesper is its amber, the colour of "working". The `mauve` of the other three is a dusty colour near the theme's greys. |
| Kanagawa Lotus (light) | the board's darker `#7c3aed`: its `mauve` is too near its text colour |
| `terminal` | the terminal's own magenta |

The same rule applies to a `mauve` of your own in `[theme.custom]`.

Other clues: the heading is a heading by its place and its word. The mark is a circular arrow, and no task mark has that
shape. Every row shows the word `on` or `off`. With `NO_COLOR`, all of this stays. An automation that is
off is quiet, as a done task is: its mark is dimmed and its words are greyed.

On the web board, the colour is the board's own purple, `#a855f7`, moved towards the text colour of the page.

## The greys

The board has three greys:

| Grey | Used for |
|---|---|
| the grey for greyed-out words (Herdr's `overlay0`) | a task number, a time, a count in the header, "… more done", the words between two hints, a label |
| a brighter grey (Herdr's `subtext0`) | words that you must read: what a row says, the keys on the hint line |
| the dim grey of "to do" | the "to do" state |

Each grey has a minimum contrast (a floor):

- **The measure** is the WCAG 2 contrast ratio between the grey and its background. 1 to 1 is the same colour. 21 to 1
  is black on white.
- **The floor is 4.5 to 1.** If the theme's plain text is so quiet that 4.5 makes the grey less than 1.5 times dimmer
  than the text, the floor is lower. It is never less than 3 to 1. The lower floors are: One Dark 4.4, Kanagawa Lotus
  4.1, Solarized 3.7, Tokyo Night Day and Solarized Light 3.0.
- **A grey under its floor moves towards the theme's plain text colour**, only as far as the floor. A grey that is
  already readable does not change.
- **"To do" is dimmer than plain text.** Plain text has at least 1.3 times the contrast of "to do". In every built-in
  theme, it has between 1.4 and 3.7 times.
- **The background.** The board paints no background. It draws on the background of the terminal. When it opens, it
  asks the terminal for its text colour and its background. Herdr answers for its panes with the colours of the
  terminal that you use. If there is no answer, the board measures against the theme's background.
- **On the selected row**, which has the theme's background, a grey keeps 3 to 1, or as much as the text of that row
  has. On Tokyo Night Day and Solarized Light, this is 2.4 and 2.3, where plain text has 3.5 and 3.1.
- **The mark of a finished task** (`✓`) is its colour made quieter: halfway to the background, or as near to that as
  keeps 3 to 1. This also applies on the selected row.

### The `terminal` theme

In the `terminal` theme, the board does not use "bright black", "white" or the faint attribute for words.

- **If the terminal gives its text colour and its background**, the board mixes the greys from those two colours.
  They have the same floor as other greys, and they are dimmer than the text. They are the only true colours that the
  board draws in this theme. Most terminals give these colours, and Herdr gives them for its panes.
- **If the terminal does not give both colours**, greyed-out words and "to do" have the plain text colour of the
  terminal. Nothing is dimmer than text, and nothing is unreadable. The heading and the `○` mark tell "to do" from the
  words of a task, as with `NO_COLOR`.
- **The selected row** has the terminal's "bright black" as its background, as in Herdr's own sidebar.
- **A colour that you name in `[theme.custom]`** from the sixteen colours of the terminal (for example,
  `overlay0 = "gray"`) does not change. The board cannot measure it.

### The web board

The web board uses the colours of the web UI. Its "to do" and its greyed-out words use the dim text of the web UI
(`--text-dim`). On every palette of the web UI, `--text-dim` has a contrast of 4.5 to 1 or more against the page, its
panels and a row under the pointer. The measured values on herdr-web-ui 0.4.8 are 4.5 to 9.9. The board's own teal for
done has a contrast of 3.6 to 1 or more there.

## The colour of the text box

The frame of the text box has the colour of the recipient of your message. For the details, see
[the colour of the text box](colours-text-box.md).

## Related pages

- [Why the board uses these colours](../explanation/colours.md)
- [Choose the board's theme](../how-to/colours-choose-a-theme.md)
- [The colour of the text box](colours-text-box.md)
- [Marks and colours on a row](board-marks-and-colours.md)
- [Settings](settings.md)
- [The web UI](web-ui.md)
- [Check progress](../explanation/check-progress.md)
- [One task after another](../explanation/after.md)
