# Marks and colours on a row

This page lists the marks that show the state of a task on the board, and the colours of a row: the mark, the review
tag, the pull request and the time.

## The marks

Each state has a mark of its own. Thus colour is never the only difference between two states.

| Mark | It means |
| --- | --- |
| `◐` | The agent works. |
| `◑` | The reviewer works. |
| `◇` | A step of the board itself. |
| `◌` | Queued: the task waits for a free slot, or for the tasks that it is tabbed to start after ([one task after another](../explanation/after.md)). |
| `✗` | Something went wrong. |
| `◆` | Ready to approve. |
| `✓` | Done. |
| `○` | To do. |

The `?` menu also has this list.

- Only a moon (`◐` or `◑`) shows that someone works: an agent or its reviewer.
- Every mark is one column wide. Every mark is a character that every monospace font has.
- The web board draws the same marks.
- The Herdr setting `status_indicators = "dots"` brings back the dots of Herdr for working (the agent and the
  reviewer) and for done. With dots, these keep their marks, because no dot has their meaning:
  - the `◇` of the board;
  - the `◌` of a wait for a slot;
  - the marks of what a waiting task needs.

### A task that needs you

A task that waits for you did not fail, so its mark is never a cross. The mark shows what the task needs. It is the
`kind` of the terminal program-status spec ([OSC 7501](https://mitchellh.com/writing/program-status-osc7501)).

| Mark | The task needs | Its State line |
| --- | --- | --- |
| `?` | An answer: the agent asks something and waits. | `asks you` |
| `!` | A yes or a no: a permission dialog, or a merge that did not go through. | `asks for your OK` |
| `!` | Your decision on finished work whose last review failed: send the review back, or merge the work all the same. | `review failed` |
| `@` | A sign-in: the program of the agent is logged out. | `needs a login` |
| `!` | You, but the board cannot tell for what (the kind is not known). | `needs you` |
| `✗` | None of these: something went wrong. | `needs you` |

- All of these tasks are under `needs you`, in the colour of that group: red.
- If colour is off (`NO_COLOR`), the mark alone shows the difference between a task that waits and a task that went
  wrong.
- Finished work that passed and waits for your yes is not one of these. Its mark is `◆`, in green, under
  `ready to approve` ([why](../explanation/board-design.md#marks-and-not-only-colours)).
- The words for the state of a task are in one list, `crates/core/src/status_words.rs`. The rows, the State line, the
  sidebar of Herdr and the web page all read that list.
- The records that the board writes to the terminal have no mark. They name the state and the kind
  (`state=blocked:kind=question`), and the terminal draws its own mark.

## The colours of a row

Each colour has one meaning:

| Colour | It means |
| --- | --- |
| Amber | The agent works. |
| Blue | The reviewer works. |
| Green | The task passed and is ready for you. |
| Red | The task failed, or it needs you. |
| Pink | Queued. |
| Grey | A step of the board itself, or to do. |

Which colours come from Herdr, which colours come from the board, and how the board follows the theme of Herdr:
[colours](colours.md).

- **While the reviewer works**, these parts are blue: the mark, the bar, the percent, the minutes left and the `R#`
  tag of the row. The same parts are blue in the task panel, on the details screen and on the chat line.
- The mark also shows the reviewer without colour: the half moon faces the other way, `◑`. When the task goes back to
  its agent, the mark is the `◐` of the agent again.
- **The steps of the board itself** have no moon but a `◇`. These steps are: starting, setting up, running the check,
  pushing and merging. They also include the waits on the way: for the turn of the check, for the turn to merge, and
  for the GitHub checks on the pull request (`Waiting for GitHub checks · 2 of 3 passed`).
- **A task that waits for a free slot** has an empty, dotted ring, `◌`, in a pink of its own
  ([a task that waits for a free slot](board-rows.md#a-task-that-waits-for-a-free-slot)).
- **An agent or a reviewer whose turn was cut off**, and that waits to try again, keeps its moon. It is still the one
  on the task.
- **The heading of a group keeps the colour of its group.** A task under review is in the `working` group.
- **To do** (its heading and its `○`) is a dim grey of its own, never the plain text colour. It is always readable:
  every grey on the board has a minimum contrast with the background.

In short:

| What you see | It means |
| --- | --- |
| A blue `◑` with a bar | A review runs. |
| An amber `◐` | The agent works. |
| A `◇` | The board works on the task. |
| A pink `◌` | The task did not start yet. |
| A green `◆` | The task is ready for your yes. |

### The blue

- The board uses the `blue` of the theme if that blue is different from all the colours next to it. These colours are:
  the green, red and amber of the theme, the light blue of the dot of Herdr for done and idle (its `teal`), muted text,
  plain text and the background.
- In many themes, the blue is not different. Examples: the blue of Dracula is its teal, the blue of Rosé Pine is its
  green, and the blue of Catppuccin is pale and near its muted text.
- In those themes, the board uses a blue of its own, `#3b82f6`. It is readable on dark and light backgrounds.
- Colours that you set yourself (`[theme.custom]`) obey the same rule. The board does not use a `blue` that is your red,
  or a `blue` that is not a blue (a green, a purple, a grey).
- If the blue of the board is too near one of your colours, the board uses a different blue.
- With the own colours of the terminal, the blue is the blue of the terminal.

## The review tag

The `R#` tag (example: `R#2`) belongs to the pull request, not to the progress. It is one cell to the left of the `PR#`
tag. The `R#` tags end under each other down the list.

The colour of the tag shows how the review went. No mark is next to it.

| Colour | The review |
| --- | --- |
| Green | It passed. |
| Red | It failed. The tag stays red while the agent repairs what the review found. |
| Blue | The reviewer works on it now, whatever the review before it said. |

- The three colours are different in light themes and in dark themes.
- The colour of a review from before the gate follows its advice.
- After the task is merged (through its pull request, or here with no pull request), the tag is quiet: the same grey
  as its merged `PR#` tag ([why](../explanation/board-design.md#quiet-tags-on-done-tasks)).
- One merged tag stays red: the tag of a task that was merged although its last review failed (you approved it all the
  same).
- The task panel, the details page and the archive use the same colours.
- A task that was never reviewed has no tag.
- Every row that had a review keeps its tag: under review, back with the agent, waiting for your yes, and done.

## The pull request

The colour of the `PR#` tag (example: `PR#18`) shows how its checks went. It uses the same rule, and no mark.

| Colour | The checks |
| --- | --- |
| Green | They pass. |
| Red | They fail. |
| Plain | No result yet: they run, or there are none. |

- A merged pull request is quiet: grey, as the time next to it.
- The greens and reds are those of the theme, the same ones that the `R#` tag uses.
- The task panel and the details page use the same colours.

## The time

- The time of a finished task is quiet, in the same grey as its merged tags. Thus all of the right side of a done task
  (the review tag, the pull request and the time) is quiet.
- That grey is the grey of the theme for greyed-out words. If the grey of the theme is too near the background, the
  board makes it readable ([colours](colours.md)).
- The board draws the time of a task that is not done as it always did.

## Related pages

- [The rows of the board](board-rows.md)
- [The header and the list](board-header-and-list.md)
- [Colours](colours.md)
- [How the colours work](../explanation/colours.md)
- [The gate](../explanation/gate.md)
- [Waiting for you](../explanation/waiting-for-you.md)
- [Why the board looks as it does](../explanation/board-design.md)
