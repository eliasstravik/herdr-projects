# Chat-only tasks: a conversation that ends when you say

**Some tasks are a conversation, not work**: a consulting session, a discussion, thinking out loud with an agent. A
chat-only task never finishes by itself, is never reviewed and never merged. It stays an ongoing conversation until you
say it is done.

```
herdr-projects add "Talk the pricing model through" --chat
```

## What its agent is told

- It is a conversation with you, not work to deliver.
- Change nothing in the repo: add, edit and delete no file, commit nothing ([like research only](research.md)).
- Talk, read, look things up, answer. It may write notes or a report into the task's own folder among the board's
  reports (`~/.herdr-projects/reports/<repo>/<number>/`) when that helps. It never has to.
- Work that should follow from the conversation it suggests in words, in its answer. It adds no task for it.
- Every stop of its own waits for you. It needs the report command only for a permission or a login, never to say
  that it waits.

## Every turn end waits for you

When its agent stops, the task waits for your next message ([waiting for you](waiting-for-you.md)). Its row says the last
line the agent wrote, with the question mark of a task that asks you something.

- **Nothing follows a stop**: no check, no review (whatever `reviewer.*` and `review.*` say), no push, no pull request,
  no merge, no "done".
- Reply from the board, or type in the agent's own terminal: its next turn starts, and its end waits again.
- The board's pushes towards finishing do not apply: the agent is never told to finish, sent back or reviewed.
- Its limits (`limit.time`, `limit.tokens`) still apply as for any task ([limits](limits.md)).
- A report it saved has its link in the task panel while the task waits.

## It ends when you say

- On the board and on the web Board: **yes** (`Ctrl+L`, `end the chat, as it is`), while the task waits for your answer.
- On the command line: `herdr-projects finish <id>`.

The task is then **done**, with the agent's last message as its summary, or `Chat ended. Report: <link>` when it wrote a
report. Nothing is checked, reviewed, pushed or merged, and its space is cleaned up like a done task's.

If its agent changed the repo anyway, nothing of that is merged either, and the line says so:
`Chat only, but its agent changed the repo (1 change in the repo): nothing was pushed, opened or merged.` What it
changed is kept (the clean-up keeps work that never merged).

A stop (`Ctrl+X`) and an archive work as on every task. Approving a chat-only task is refused: there is nothing to
merge.

## Making a task chat only

It is a [setting](settings.md) like every other: `chat.only`, `true` or `false`, `false` built in. It is looked up as every
setting is, nearest answer first: **the task → your entry for that repo → you → the repo's own file → built in.**

| For | How |
|---|---|
| one task, when it is added | `herdr-projects add "<title>" --chat` (short for `--chat.only true`) |
| one task, later | `herdr-projects delegate <id> --chat` (`--no-chat` turns it off), `herdr-projects set chat.only true --task <id>`, or its row on the task's settings page (`Chat only`): on the terminal board, and on the web Board under the open task (**All settings**) |
| a whole repo, or everything of yours | `herdr-projects set chat.only true --repo <name>`, or without `--repo`. The repo's own `.herdr-projects.json` may say it too: `{ "chat": { "only": true } }` |
| an [automation](automations.md) | `--chat` when it is made or changed (`--no-chat` turns it off), or `set chat.only true --automation <name>`. Each task it makes has it as its own |

- **The Coordinator does it by itself** when you plainly ask for a discussion or a consulting session with nothing to
  build ("I want to think the roadmap through with an agent"). Not for work, and not for research you want delivered as a
  report: that is [research only](research.md).
- It is read when the task's agent starts, as research only is: changed on a task under way, it counts from the task's
  next start, and `show <id>` says so beside the value (`counts from its next start: this run is not chat only`).

## Chat only and research only together

**Chat wins.** A task with both is chat only: it is research that never finishes by itself. Its agent gets the chat's
rules, its turn ends wait for you, and its row says `[chat]`, not `[research]`.

## On the board

- The row says `[chat]` after the task's name: on the board, in Herdr's sidebar, on the web Board, and for a task on
  another machine. The label comes from the setting, never from the title.
- The task panel has a part **Chat**: `No change to the repo · ends when you say.`, with the report's link when there is
  one. Its settings say `Chat  no change to the repo · ends when you say` where an ordinary task says how it merges, and
  leave out the review and the check, which never run.
