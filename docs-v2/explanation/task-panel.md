# How the task panel is made

This page tells how the task panel fits a short window, what occurs when a part cannot show, and why the panel looks
as it does. The parts of the panel are in [the task panel](../reference/task-panel.md).

## A short window

In a short window, the panel removes full lines, the least necessary lines first. The state and the `Now` text stay
complete. **The settings stay longer than every text.** The panel shows them in full whenever it can.

The panel removes lines in this order:

1. The empty line on top goes. Then what the agent said and the review lose lines, each down to a few lines. The review
   loses its summary first, then `History` and `Evidence`. A text that lost lines ends in ` …`.
2. What the agent said, the attempts, the plan and the report go. The review keeps its one row
   (`Review 3  ✓ Pass · 2 minor`).
3. The description loses lines, down to one line that tells where to read it (`Description · alt+t shows it`). What
   the task needs from you goes down to its first two lines. Then the one line of the description goes.
4. Only now do the settings lose lines:
   - First, the rows that are not settings go: `Project`, `Machine`, `Note`.
   - Then the rows go one at a time. Each row that stays is complete.
   - The rows that stay longest are: a row with a value that somebody chose (a value that names its place), then
     `Agent`, `Reviewer`, `Review when`, `Send back` and `Merge`, then the other rows.
   - The last line under the rows tells how many settings have no space and the key that shows them
     (`13 more settings · ctrl+e shows them`).
5. If there is no space for one setting, the headline of the settings goes too. The other parts get their lines back.
   They then lose lines as in steps 1 and 2. The description is the last of the texts to go.
6. These go next: the facts that the panel shows elsewhere (`Task`, `On it`, `Made by`), the review down to its one row,
   the empty lines between the parts, the `Status` headline, and the second line of the title. Then `Change`, the row of
   the review and the title go.

After this, the state, the progress and the `Now` text are left. If there is no space for these and the last line
(`alt+t shows the whole task`), that line goes. Then the progress goes, then the state. The text itself stays last. The
panel cuts only a text that is longer than the full panel. The last line of that text ends in `…`.

## A part that cannot show

If a part of the panel cannot show, only that part fails. Its headline stays, with one red line under it:
`✗ This part could not be shown: …` and the error. All other parts of the panel show, and every key works.

The same is true for each section of the screen: the list, the panel, the task details, the menu, the chat, the text
box and the hints. A section that fails shows that red line in its place, and the other sections stay. Only a failure
outside all of these sections replaces the board with `view error`.

## Why the panel looks like this

The panel shows one issue and the history that comes under it. An issue page or a pull request page shows the same
thing. Thus the form comes from tools that already do this well. It is changed to fit the text cells of a terminal.

| From | What the panel uses | What the panel does not use, and why |
|---|---|---|
| The pull request page of GitHub | A status first. Then the checks and reviews, summed up as rows of a mark, a name and a result. What stops the merge, named immediately under the status. The timeline, with the newest relevant item first. | Avatars, badges as coloured pills, and a column on the side: there is no space and there are no pixels. |
| `gh pr view`, `gh pr status`, `gh pr checks` | Rows of a label and a value for the facts. One line for each run, with its mark, result and age. A result in words, with the colour only repeating it. | Its one-line form with dots between items (`Checks passing - Approved`): that was the old top of the panel. |
| lazygit, gh-dash | One accent colour for each state, quiet labels, and the detail of the selected item beside the list and in full on its own screen. | Boxes around every pane, and tabs in the detail: a box uses two columns and two lines of a panel that is 38 columns wide. |
| CodeRabbit, Greptile, Cursor Bugbot | Findings by severity, in a column of their own. A verdict line before the detail. What changed since the last round, as a history line. | Emoji badges, a score out of five for new reviews (the pass or fail of the gate tells more), and folded sections. |
| The guidelines of the `/ui` skill | Labels quieter than values in a list of facts. The lightest separation that works: space, then a rule, never a card by default. Numbers that align. | All rules about pixels, radii and shadows. |
| The `tui-design` skills on skills.sh | The same place for the same fact in every state. A mark or a word beside every colour. A test at narrow, usual and wide widths. Lines that go away by priority, not text that is cut. | Borders for emphasis, and their colours: the board keeps the colours of Herdr. |

## Related pages

- [The task panel](../reference/task-panel.md)
- [Colours](colours.md)
- [Board design](board-design.md)
- [The gate](gate.md)
