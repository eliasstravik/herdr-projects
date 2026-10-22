# Skills in the chat

**Type `/` in the board's text box to use a skill**, as in the Herdr web UI's chat box. A list over the box shows the
skills and commands of whoever the chat talks to, with what each does. Pick one and send: its program runs it.

```
 ◐ PRJ-34 Fix the login  working
 3 skills and commands
▌  /plan-day   Plan the day
   /review-pr  Review a pull request
   /ship       Ship the release
────────────────────────────────────────────────────────────────────────────────
❯ /
────────────────────────────────────────────────────────────────────────────────
  ↑↓ pick a skill · tab enter put it in the box · esc close the list
```

## Using it

- **Type `/`** at the start of your message. The list shows what that program offers. Keep typing to narrow it: `/rev`
  lists `/review-pr`. Names that start with what you typed come first, then names with a word that does, then names that
  hold it. A plugin's name (`/tools:lint`) is found by its own part too (`/lint`).
- **`↑` `↓`** pick one. The picked one is highlighted.
- **`Tab`**, or **`Enter`**, puts it in the box with a space after it. Type what it should work on, if anything, and
  press `Enter` to send.
- **`Enter` on a word that already is the picked one** (`/ship`, typed whole) sends it at once.
- **`Esc`** closes the list and keeps what you typed. Change the word and it lists again. A second `Esc` puts the draft
  away, as always.
- The list is only for a message that starts with `/` and is still one word. A `/` further on (`look at /tmp`) lists
  nothing.

What you send goes to the agent word for word, so `/review-pr the login branch` reaches it as you typed it. A skill or
command you know works without the list too: type it and send.

## Whose skills

The list is of the program the words go to, read in the folder it runs in:

| You talk to | Its list |
|---|---|
| a task's agent (`Space` on the task) | its program's skills, read in the task's worktree (and the repo's main folder, for skills not committed yet) |
| the Coordinator | its program's skills, in the Coordinator's own folder (no repo's) |
| a repo's coordinator (`Space` on a repo's heading) | its program's skills, with that repo's |
| a machine's coordinator | its program's skills, in its own folder |

No list shows for words that go to another machine (its skills are on that machine), for a task with no agent yet, or
with no Coordinator (your words are added as a task). What you type still goes as typed.

## What each program offers

As the Herdr web UI lists them:

| Program | Skills | Commands |
|---|---|---|
| Claude Code | `/name`: yours (`~/.claude/skills`, or `$CLAUDE_CONFIG_DIR`), the project's (`.claude/skills` in the folders up to the repo's top), and those of the plugins turned on (`/plugin:name`). A skill marked `user-invocable: false` is left out | `/name` from `~/.claude/commands` and the project's `.claude/commands` (`git/commit.md` is `/git:commit`), and the plugins' |
| Codex | `$name`: the project's `.agents/skills`, yours in `~/.agents/skills` and `~/.codex/skills` (or `$CODEX_HOME`), and the ones it comes with | your prompts as `/prompts:name` |
| pi | `/skill:name`: yours (`~/.pi/agent/skills`, `~/.agents/skills`) and the project's (`.pi/skills`, `.agents/skills`) | prompt templates as `/name` |

- Each shows what it does in its first sentence: the rest of a skill's description says when to use it, which is for the
  agent.
- A program's own built-in commands (`/model`, `/compact`, …) are not listed. They work when typed.
- Other programs list nothing. What you type still goes to them as typed.
- A skill added or changed shows within a few seconds.
