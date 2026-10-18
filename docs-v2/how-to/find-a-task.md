# Find a task

This page tells you how to find a task by its words, its number or its tag, on the board, on the web board and on
the command line.

A task matches by its words, and also by its number and its tag. For the task `HSW-134`, `134`, `#134` and `hsw-134`
all match.

## Find a task on the board

1. Press `Ctrl+F`.
2. Type the words, the number or the tag of the task. The list filters while you type.
3. If you type exactly the number or the tag of one task, the selection goes to that task.

**Note:** The list filters while you type, so `13` also shows `HSW-13` and `HSW-135`.

**Note:** While find is on, every matching task shows, also a task under a folded heading
([list headings](../reference/list-headings.md)).

**Note:** For a task of another machine, type the tag alone, without the machine.

## Find a task on the web board

1. Type the words, the number or the tag of the task in the search box. The list filters in the same way as on the
   board.

## Find a task on the command line

1. Type `herdr-projects list --query`, followed by what you look for.

The query matches in the same way as find on the board.

## Related pages

- [Task numbers](../reference/task-numbers.md)
- [Fold list headings](fold-list-headings.md)
- [Commands](../reference/commands.md)
- [Keys](../reference/keys.md)
