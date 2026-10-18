# The code of the agent programs

This page tells where the code of the agent programs is, and how to add a program.

## The table of programs

The table of programs comes from the code in `crates/core/src/agents/kinds/`. `herdr-projects docs` writes it between
the `made:agents` lines in `docs/agents.md`. Do not edit that table by hand.

To add a program, add one more entry in `crates/core/src/agents/kinds/`. A program that Herdr starts but has no
integration for (Gemini CLI, Cline, Kiro, Amp, Maki, Muse) is also one more entry.

## Helpers (subagents)

The rules of the board name helpers as allowed. A test keeps both parts of this behaviour (the helpers are allowed,
and the work is not given to another task or agent on the board): `tests/agent_helpers.rs`.

## Related pages

- [Agent programs](../reference/agent-programs.md)
- [How the board uses agent programs](../explanation/agent-programs.md)
- [The code of the retries and the fallback](fallback.md)
- [The crates](crates.md)
- [Testing](testing.md)
