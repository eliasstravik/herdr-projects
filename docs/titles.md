# Titles

**A title is at most 60 characters.** A longer one is shortened, never refused, and its full text is kept.

The limit is the same everywhere a title is made or changed: a task you type in the box, `add`, and
`report --title`.

## A title that is too long

```
$ herdr-projects add "After the startup dialog handoff, finish jobs wrongly mark the task as failed"
proj/12: added The title was over 60 characters, so it was shortened to "After the startup dialog handoff, finish
jobs wrongly mark…" and the full text was kept in the description. Rename it if a better short title exists.
```

- It is cut at the end of a word and ends in `…`: `After the startup dialog handoff, finish jobs wrongly mark…`.
- One long word with nowhere to cut is cut where the room ends.
- A title that is too long only by its final full stop just loses the full stop.
- The full text becomes the first paragraph of the description, above what was there. If the description already
  says it, it is left alone.

Why shorten instead of refusing: a task an agent adds for you is never lost over its title, and the answer tells you
what to do.

## Why 60

That is about the room a title gets on a list row: in a 120-column window with the task panel open, the list is 72
cells wide and the mark and time take the rest. It is also what Herdr's sidebar gets for the task's space name, after
the task's number. A 60-character title is read whole in both places. A 100-character sentence filled the row and
pushed out everything else.

## How characters are counted

By room on screen, the same way rows are drawn. Most letters count as one. A wide character (Chinese, Japanese,
Korean, an emoji) takes two cells and counts as two, so 30 of them fill a title. A title at the limit is 60 cells,
whatever the language.

## Who is told

So that titles are written to fit and need no cutting:

- The Coordinator's rules give the limit where they say how to add and rename a task.
- The board's how-to (`SKILL.md`) gives it where it says how to add a task.

## Titles from before the limit

They stay as they are: nothing rewrites saved tasks. The list cuts a long title with `…` as it always did. The limit
applies the next time you rename one.

The number is `TITLE_MAX` in `crates/core/src/store/titles.rs`, and nothing else knows it.
