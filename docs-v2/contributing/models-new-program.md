# Add a model list for a new program

This page is for contributors. It tells where the board finds how to list the models and effort levels of a program.

## The models of a program

How the board lists and reads the models of a program is one entry in the row of that program in the table of
programs: `crates/core/src/agents/kinds/`, field `models`. The entry gives one of these:

- the command that lists the models, and its format (and, for Codex, its app-server: `server`)
- the short names and full names of the models

The board does not check the models of a program that has neither.

## The effort levels of a program

The board reads the effort levels of a program from its `--help`, unless its row tells otherwise with `efforts_read`.
Examples of `efforts_read` are the catalog of Codex, or the table only. The `efforts` of the row are the levels that
the board uses when it cannot read the levels.

## Related pages

- [Models and effort](../reference/models.md)
- [Agent programs](../reference/agent-programs.md)
- [Crates](crates.md)
