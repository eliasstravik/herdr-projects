# Colours

**The board uses Herdr's own theme, and colour means status only.** A mark's colour says where a task or a coordinator
stands, and the text box's edge takes the status colour of whoever you type to. Names are never coloured: machines,
projects and tasks are all in the base colour. Colour is never the only clue: every state has its own mark and words
too, and with `NO_COLOR` the board still reads.

| Colour | Means |
|---|---|
| amber (Herdr's `yellow`) | an agent works |
| red | it needs you (blocked or failed) |
| green | ready to approve |
| blue | under review |
| teal | done |
| pink | queued: waits for a free slot, or for other tasks |
| dim grey | to do |
| light grey | the board runs its own check |
| purple | an automation |
| orange | an autopilot's wakes in the chat |
| the base colour | every name: a machine's, a project's, a task's title |
| the fainter grey | what goes with a name: counts (`2 machines · 5 tasks`, `7 tasks`), keys (`HP`, `HOME`), times, and tags after a title (`[auto]`, `[research]`, `[chat]`, `×3`) |

## The board is in Herdr's theme

Everything the board draws is in the theme Herdr shows, so the two never look like different programs. Herdr does not
hand its theme to a plugin, so the board reads what Herdr reads: the `[theme]` and `[ui]` tables of this machine's
`~/.config/herdr/config.toml`, resolved as Herdr resolves them (the theme's name and its other names, `auto_switch` with
`dark_name` and `light_name`, `[theme.custom]`, the older `ui.accent`).

- **No theme set means Herdr's default**, Catppuccin, exactly as Herdr shows it. The board never falls back to the
  terminal's own colours while Herdr beside it shows its default.
- **`auto_switch`**: the board asks its terminal whether it is light or dark, as Herdr does, and takes the theme's light
  or dark one.
- **The `terminal` theme** paints with the terminal's own sixteen colours, in Herdr and on the board alike. Its greys are
  the exception ([below](#the-greys-are-quiet-but-always-readable)).
- **A theme changed in Herdr** shows the next time the board opens. The popup, the task panel, the reply view and the
  review page take the same theme. Another machine's tasks are drawn by this board, in this theme.

To choose by hand, set [`board.theme`](settings.md): `herdr` (the default, all of the above), `dark` or `light` (that
theme's dark or light one), or one of Herdr's themes by name: `catppuccin`, `catppuccin-latte`, `terminal`,
`tokyo-night`, `tokyo-night-day`, `dracula`, `nord`, `gruvbox`, `gruvbox-light`, `one-dark`, `one-light`, `solarized`,
`solarized-light`, `kanagawa`, `kanagawa-lotus`, `rose-pine`, `rose-pine-dawn`, `vesper`.

### When the board cannot know the theme

- **Herdr opened from another machine (`herdr --remote`).** Herdr's screen is then painted by its client, with the config
  of the machine the client runs on, and Herdr passes neither that config nor its theme to the server the board runs on.
  If that machine's config names another theme, Herdr shows that theme around a board in this machine's. Set
  `board.theme` to that theme's name (`herdr-projects set board.theme nord`), or give both machines the same `[theme]`.
  A plain `ssh` to this machine with Herdr started here is not this case: client and server are on one machine, and the
  board matches.
- **The web Board.** The Herdr web UI does not read Herdr's theme; it has colours of its own (its CSS variables, light and
  dark and its palettes). The Board there uses those, so it matches what Herdr looks like on the web, not in a terminal
  ([the board in the web UI](web-ui.md)).

## A status is in Herdr's own colour

A task's status is drawn in the colour Herdr's sidebar and agent list use for that state, whatever else the theme does:
working is Herdr's `yellow` (amber), blocked or failed its `red`, done its `teal`. The mark, the group's heading, the bar,
the percent and the minutes left take it.

The board has colours of its own only for what Herdr has no state for:

| State | Colour | Which one |
|---|---|---|
| under review | blue | the theme's `blue` where that is told apart from all it sits beside, else the board's `#3b82f6` ([the board](board.md)) |
| ready to approve | green | the theme's `green` |
| queued: waits for a free slot, or tabbed and waits for other tasks to be done | pink | one of the board's own pinks, the first told apart from every other status colour, the automations' purple, the greys, text and background ([below](#a-queued-task-is-pink)) |
| to do | dim grey | the theme's grey for greyed-out words (`overlay0`), made readable as said below, where that is told apart from the green and the blue; else another grey. With the terminal's own colours it is a grey mixed from the terminal's own background and text colour, measured |

- They are told apart from each other in every built-in theme (the tests measure it).
- "To do" is a dim grey in the list (the heading, the `○` mark, the task's number) and in the box alike: dimmer than plain
  text, and always readable.
- While the board runs its own check, the mark is a plain light grey, the theme's `subtext0`
  ([the check's progress](check-progress.md)).

## A queued task is pink

A task started while every slot of `board.agents` is taken has a colour no other state has: pink. It is used for its `◌`,
its place in the queue (`Q#2`), the State line in the task panel, and its agent's text box.

A task tabbed with `--after` while the tasks it waits for are not all done is queued too, in the same pink: its `◌`, its
State line, its box, and its line `queued after SKL-6` ([one task after another](after.md)). A task that only waits for
others, never tabbed, stays the dim grey of "to do", and its row says nothing of the wait.

Pink is the colour furthest from all the others: amber works, red needs you, green is ready, teal is done, blue is under
review, grey is to do, purple is an automation.

Herdr's themes have no pink of their own (only Dracula's `mauve` is one, and that is the automations' colour there). So
it is one of the board's: the first of its pinks that is told apart from every colour above, from plain text, both greys
and the background, and stands at least 3 to 1 against the background, what a mark needs. Its nearest neighbour is red,
the colour of "needs you", so the measure keeps it well away from the theme's red.

| Theme | The pink |
|---|---|
| Nord, One Dark, Solarized, Kanagawa, Vesper | `#f472b6`, the first of the pinks for a dark background |
| Tokyo Night, Gruvbox, Rosé Pine | `#ff6ec7`: `#f472b6` is too near one of their colours |
| Catppuccin (Herdr's default), Dracula | `#ec4899`: the two brighter ones are too near their red, a pink-ish one |
| Catppuccin Latte, Gruvbox light, One Light, Rosé Pine Dawn | `#db2777`, darker, for a light background |
| Tokyo Night Day, Kanagawa Lotus | `#c2188b` |
| Solarized Light | `#9d174d` |
| `terminal` | `#ec4899`: the terminal's sixteen colours have no pink, and their magenta is the automations'. A middle pink reads on a dark terminal and a light one alike |

**Colour is not the only clue**: the `◌` is a dotted ring no other state has, and `Q#2`, `Waiting for a free slot` and
`queued after SKL-6` are words. With `NO_COLOR` they still say it.

On the web Board it is the board's own `#ec4899`, pulled towards the page's text colour, as the purple is.

## An automation is purple

Automations are not tasks, so nothing of theirs may be mistaken for a status. They have one colour of their own, used
for the heading `automations` in the list, the mark of each of their rows (`⟳`, dimmed while it is off), the `[auto]`
mark of an automation's line in the `?` menu. The `[auto]` tag of a task one made is a tag like any other: dimmed.

It is **Herdr's own `mauve`**, the one colour of Herdr's theme no status uses, so it changes with the theme like
everything else. It is used where it is a purple (from violet to pink) and told apart from the status colours (red,
green, amber, teal, blue, grey), from plain text, from both greys of greyed words and from the background. The tests
measure that in every built-in theme.

| Theme | The automations' colour |
|---|---|
| Catppuccin (Herdr's default), Latte, Tokyo Night and Day, Dracula, Gruvbox dark and light, One Dark and Light, Solarized dark and light, Rosé Pine Dawn | the theme's `mauve` |
| Nord, Kanagawa, Rosé Pine, Vesper | the board's `#a855f7`: Vesper's `mauve` is its amber, the colour of "working", and the other three's are dusty ones that sit on the theme's greys |
| Kanagawa Lotus (light) | the board's darker `#7c3aed`: its `mauve` sits on its text colour |
| `terminal` | the terminal's own magenta, so it follows the terminal as the rest of that theme does |

A `mauve` of your own (`[theme.custom]`) is held to the same rule.

**Colour is not the only clue**: the heading is a heading by its place and its word, the mark is an arrow going round
where every task's mark is another shape, and `on` or `off` is a word on every row. With `NO_COLOR` all of that still
stands. One that is off is quiet as a done task is: its mark dimmed, its words greyed.

On the web Board it is a purple of the board's own (`#a855f7`, pulled towards the page's text colour so it reads on light
and dark), since the web UI has no purple among its status colours.

## A coordinator on autopilot has the theme's colours; its wakes are orange

A coordinator on [autopilot](autopilot.md) has no colour of its own on the list: its machine's or project's row has the
mark `∞` in the theme's own colours, as a task's mark is: red at `needs you`, yellow while it works, quiet while asleep
and plain text else. The top row's `∞` is the coordinator of all machines' (or the only machine's).

**Colour is not the only clue**: `∞` is a shape no task's mark has, its panel's state word says where it stands, and
nothing of autopilot shows while it is off. With `NO_COLOR` that still stands.

Its wakes in the chat, under the name `Autopilot`, are an orange, so they are never read as your words:

- It is **the theme's own `peach`** where that is an orange told apart from every status colour, the automations'
  purple, the queue's pink, both greys, plain text and the background. The tests measure that in every built-in theme.
- Else it is the first of the board's oranges that is: `#fb923c`, `#f97316`, `#ff8c42`, `#ea580c`, `#c2410c`, the darker
  ones first on a light theme.

On the web Board it is the board's `#f97316`, pulled towards the page's text colour, as the purple and the pink are.

## The greys are quiet but always readable

The board has three greys:

- the one for greyed-out words: a task's number, a time, a count in the header, "… more done", the words between two
  hints, a label;
- a brighter one for words that must be read: what a row says, the keys on the hint line;
- the dim grey of "to do".

They are Herdr's own (`overlay0`, `subtext0`), but Herdr's themes do not all keep them readable. Nord's grey for
greyed-out words is 1.7 to 1 against its background, and nine more themes are under 3 to 1. So each grey is held to a
floor:

- **The measure** is the WCAG 2 contrast ratio between the grey and its background: 1 to 1 is the same colour, 21 to 1
  is black on white.
- **The floor is 4.5 to 1**, what WCAG asks of text. Where a theme's own plain text is so quiet that 4.5 would leave the
  grey less than 1.5 times dimmer than it, the floor comes down with the text, but **never under 3 to 1** (One Dark 4.4,
  Kanagawa Lotus 4.1, Solarized 3.7, Tokyo Night Day and Solarized Light 3.0).
- **A grey under its floor is pulled towards the theme's plain text colour, only as far as the floor.** So it keeps the
  theme's tint, and a grey that already reads is left alone.
- **"To do" is also dimmer than plain text by the same measure**: plain text stands out from the background at least 1.3
  times as much. In every built-in theme it is between 1.4 and 3.7 times.
- **The background is the real one where it is known.** The board paints no background: it draws on the terminal's own.
  When it opens, it asks the terminal for its text colour and background (Herdr answers for its panes with those of the
  terminal you sit at) and measures against that background. With no answer, it measures against the theme's.
- **On the selected row**, whose background is the theme's, a grey keeps 3 to 1, or as much of it as that row's own text
  has to give (Tokyo Night Day and Solarized Light: 2.4 and 2.3, where plain text itself has 3.5 and 3.1).
- **A finished task's mark** (`✓`) is its colour made quieter: halfway to the background, or as near to that as keeps it
  3 to 1 (what WCAG asks of a mark), on the selected row too. It used the faint attribute once, which fell to 1.7 to 1 on
  a light background.

### In the `terminal` theme

There the board does not know the colours behind the terminal's sixteen, and none of the terminal's own ways to grey a
word can be trusted:

- **"bright black"** is nearly the background on many dark schemes: on a dark navy one it made "to do" unreadable (1.3
  to 1);
- **"white"** is nearly the background on light ones;
- **the faint attribute** halves the text colour, and a quiet scheme has no contrast to spare: on Solarized light (text
  4.1 to 1) faint text is 1.9 to 1.

So the board uses none of the three for words:

- **If the terminal told its text colour and background** (most do, and Herdr does for its panes), the greys are mixed
  from those two and held to the same floor as any other, dimmer than the text they were mixed from. They are the only
  true colours the board draws in this theme.
- **If it did not tell both**, greyed-out words and "to do" are the terminal's plain text colour, the one colour there
  known to be readable. A grey that must be dimmer than text cannot be chosen without knowing the text: one measured
  against the background alone came out stronger than the real text on Solarized light. Nothing is dimmer than text then,
  and nothing is unreadable: "to do" is told from a task's own words by its heading and its `○`, as with `NO_COLOR`.

The selected row is the one place left to the terminal: its background in this theme is the terminal's "bright black",
as in Herdr's own sidebar, and what reads on that cannot be known.

A colour you name yourself in `[theme.custom]` from the terminal's sixteen (`overlay0 = "gray"`) is kept as it is: it
cannot be measured.

### On the web Board

The web Board uses the web UI's own colours. Its "to do" and its greyed-out words are the web UI's dim text
(`--text-dim`), which is 4.5 to 1 or more against the page, its panels and a hovered row on every palette of the web UI
(measured on herdr-web-ui 0.4.8: 4.5 to 9.9). The board's own teal for done is 3.6 to 1 or more there.

The rules below hold there too: names in the base colour, the hierarchy by weight and how far in a row stands, the
filter row's lit filter in its own colour (on a tint of it, as the web UI marks a status), and the chat's frame and the
name of whoever it talks to in that one's status colour. Inside the board's own conversation your words are the web
UI's strong text, theirs its dim text.

## Names have no colour; the hierarchy is weight and indent

- **Machines** are in bold, in the base colour (`All machines` too). **Projects** are in normal weight, one step in.
  **Tasks** are in normal weight, another step in. No accent on a machine's name, no colour on a project's or a task's.
- **What goes with a name is dimmed**: counts, keys, times, and the tags after a title (`[auto]`, `[research]`,
  `[chat]`, `[archived]`, `×3`).
- **Status keeps its colours**: the marks, a status group's heading, the filter row's lit filter and the selected row
  are as they were.

## The box says who you type to

The text box's edge takes the status colour of whoever it talks to, as their mark has it; its ladder already says who
([machine and project coordinators](machine-and-project-coordinators.md#the-chat-box-names-who-it-talks-to)). Inside it,
your words are in the base colour you type in, theirs grey.

| Who the box talks to | Colour of its edge |
|---|---|
| idle, never started, to do, done | the base colour |
| working | amber |
| needs you (asks you, wants your OK) | red |
| failed, could not start | red |
| ready to approve, under review, queued | that status's colour |
| a coordinator on autopilot | its `∞` mark's colour: red at `needs you`, amber while it works, else base |
| nobody: a new task where no coordinator can run, find | the base colour |

- The lit step of the ladder is in the same colour, reversed. It is the same wherever there is such a box: under the
  list, in the reply view, under the task panel and full screen, in the popup, and on the web Board, where the frame of
  the text field takes the colour.
- **Colour is never the only clue.** The ladder names who it is (`vm-str › gtm`, `gtm › GTM-14`), the grey words in the
  empty box say who gets it (`Send message to gtm's coordinator`, `your message starts an agent on it`), and while you
  type the hint line does (`enter send to HSW-34's agent`). With `NO_COLOR` the board draws no colour at all.

On the web Board, done is a teal of the board's own: the web UI has one colour for done, and it is the green of "ready to
approve" there, so the two would be one colour.
