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

The same as the Herdr web UI lists, read from the same places:

| Program | What it lists |
|---|---|
| Claude Code | `/name`: your skills (`~/.claude/skills`, or `$CLAUDE_CONFIG_DIR`), the project's (`.claude/skills` in the folders up to the repo's top), your and the project's commands (`git/commit.md` is `/git:commit`), and those of the plugins turned on (`/plugin:name`). A skill marked `user-invocable: false` is left out |
| Codex | `$name`: the project's `.agents/skills`, yours in `~/.agents/skills` and `~/.codex/skills` (or `$CODEX_HOME`), the ones it comes with, and those of the plugins turned on in its `config.toml` (from its plugin cache). A skill turned off there (`[[skills.config]]`, `enabled = false`) is left out. Your prompts as `/prompts:name` |
| pi | `/skill:name`: yours (`~/.pi/agent/skills`, `~/.agents/skills`) and the project's (`.pi/skills`, `.agents/skills`); prompt templates as `/name` |
| OpenCode | `/name`: the commands of its `opencode.json(c)` and `commands` folders, the skill it comes with (`customize-opencode`), and the skills of its own folders, `~/.claude/skills`, `~/.agents/skills` and the project's |
| Devin | `/name` by the skill's folder: Devin's own skills folders, Windsurf's, Claude Code's and Copilot's (as its `config.json` lets it read them), and its plugins' as `/plugin:name`. A name two places have says the place: `/claude:review` |
| Grok | `/name`: the project's `.grok/skills` and `.claude/skills`, yours in `~/.grok/skills` (or `$GROK_HOME`) and `~/.claude/skills`, the paths of its `config.toml`, and its plugins'. What git or its `[skills] ignore` leaves out is left out |
| Hermes | `/name` in its own spelling (`Web Tool` is `/web-tool`): `~/.hermes/skills` (or `$HERMES_HOME`) and its other skill folders, a project's only where its `config.yaml` trusts it, its skill bundles and its quick commands |
| omp | `/skill:name` from every place it reads (its own folders, Claude Code's, `.agents`, Codex's, OpenCode's, `.github`, the Claude Code plugins turned on, the extensions its `settings.json` names and its npm plugins), as its `config.yml` lets it; its commands and prompts as `/name` |
| Antigravity (agy) | `/name`: the project's `.agents` skills (and those its `skills.json` names), yours in `~/.gemini/config/skills` and its plugins', and the ones it comes with |

- Each shows what it does in its first sentence: the rest of a skill's description says when to use it, which is for the
  agent.
- A program's own built-in commands (`/model`, `/compact`, …) are not listed. They work when typed.
- The other programs (Copilot, Cursor, Droid, Kilo, Kimi, Letta, Mastra Code, Qoder, Qwen) list nothing, as in the web
  UI. What you type still goes to them as typed.
- A skill added or changed shows within a few seconds.
