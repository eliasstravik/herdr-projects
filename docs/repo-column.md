# The repo column

On the board of every repo, a row can say which repo its task is in. Turn it on with the setting
`board.repoColumn` (default off; "Repo name on each row" in the settings, see [settings](settings.md)).

- The repo's name stands right-aligned on the row, just before the review, the pull request and the time.
- It is text, never colour alone: with `NO_COLOR` the name still reads. Its colour only tells repos apart: the same
  repo always has the same one, taken from its name.
- It is not shown when the view is one repo (`--repo`, [saved views](saved-views.md)): the header already says which.
- It takes only the room that is left. The title, the columns and what the row says keep theirs first; the name is
  cut to what is left (at least four cells), and dropped on a row where less is left.
