# Titles

This page gives the length limit of a task title and what the board does with a title that is too long. For the
reasons, read [why titles are short](../explanation/titles.md).

## The limit

**A title is at most 60 characters.** The board shortens a longer title. It does not refuse it, and it keeps the full
text.

The limit is the same in every place where a title is made or changed:

- a task that you type in the box;
- `add`;
- `report --title`.

## A title that is too long

```
$ herdr-projects add "After the startup dialog handoff, finish jobs wrongly mark the task as failed"
proj/12: added The title was over 60 characters, so it was shortened to "After the startup dialog handoff, finish
jobs wrongly mark…" and the full text was kept in the description. Rename it if a better short title exists.
```

- The board cuts the title at the end of a word, and adds `…`:
  `After the startup dialog handoff, finish jobs wrongly mark…`.
- If the title is one long word with no place to cut, the board cuts it where the space ends.
- If only the final full stop makes the title too long, the board removes only the full stop.
- The full text becomes the first paragraph of the description, above the old description. If the description already
  holds the full text, the board does not change it.

## How the board counts characters

The board counts the space on the screen, as it does when it draws rows.

- Most letters count as one.
- A wide character (Chinese, Japanese, Korean, an emoji) takes two cells and counts as two. Thus 30 wide characters
  fill a title.
- A title at the limit is 60 cells, in all languages.

## Who knows the limit

The limit is given in these places, so that titles fit and the board does not have to cut them:

- The rules of the Coordinator give the limit where they tell how to add and rename a task.
- The how-to of the board (`SKILL.md`) gives the limit where it tells how to add a task.

## Titles from before the limit

The board does not change them: nothing rewrites saved tasks. The list cuts a long title with `…`, as before. The
limit applies the next time that you rename the task.

## For contributors

The number is `TITLE_MAX` in [`crates/core/src/store/titles.rs`](../../crates/core/src/store/titles.rs). No other code
knows the number.

## Related pages

- [Why titles are short](../explanation/titles.md)
- [Commands](commands.md)
- [Board rows](board-rows.md)
- [Herdr's sidebar](sidebar.md)
