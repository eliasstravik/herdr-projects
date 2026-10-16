# Command Code

Command Code (`commandcode`, also `command-code`, `cmd` and `cmdc`) can be the worker, reviewer, Coordinator,
planner or Watcher, and can take part in fallbacks and attempts. Install Command Code 1.82.5 or newer and start it
once yourself to log in. Herdr 0.9.3 works: no separate herdr integration or change to herdr is needed.

```
herdr-projects set worker.agent commandcode
herdr-projects set reviewer.agent commandcode/anthropic/claude-sonnet-4-6
herdr-projects models commandcode
```

The model names come from `commandcode --list-models`. A model's effort goes through `--effort`; the board passes
the requested level to Command Code, which checks what that model supports. Its CLI help only gives example levels,
so the board does not treat those examples as an exhaustive list. Fast service is not a separate Command Code flag.

## Starting and resuming

The board starts the program in the task's pane. Command Code's built-in herdr mod reports `cmd` as working, at rest,
or waiting for a permission or answer. Keep that built-in mod enabled. The board checks the pane's terminal identity
and waits for the program to report ready before sending instructions. The agent gets a unique board name like
every other program.

Full access uses `--yolo`; asking first uses `--permission-mode standard`. Both skip project trust and onboarding,
and disable automatic updates during that run. The board controls its own worktree, so it does not ask Command Code
to create another one. Plans use the board's read-only mount and approval flow, as Codex does.

Every launch gets a private mod that records its actual session id, scoped to the herdr socket and terminal. A new
chat updates that id; a restart reopens that exact conversation with `--resume`, never the folder's last chat. This
also lets project coordinators share a folder with the human's own chats without resuming one of those instead.

## What the board reads

The board reads Command Code's native JSONL files under `~/.commandcode/projects/`: the active branch's messages,
tools still awaiting results, model and usage. Rewound messages stay out of the chat. Tool output, thinking and
injected prompts are not shown as things the human said.

Per-launch mods add the standing rules to the system prompt and report the end of a whole run. They preserve the
user's hooks and settings. Provider errors are reported even when the provider failed before the first assistant
message was written: passing errors use the usual retry policy, and lasting ones use the usual fallback policy.
An interrupted run or a denied permission is not a completed task. Background helpers keep the task open until
they return. The usual closing note remains the fallback if the callback is unavailable.

Command Code's own taste learning can create `.commandcode/taste/` in a new worktree. Those files count as changes
on a research-only task too. Initialize the intended taste files beforehand or turn taste learning off in Command
Code's own project settings when that is unwanted; the board does not silently delete or ignore them.

Token limits count input, output, cached reads and cached writes once per recorded model response, including responses
on abandoned branches (those tokens were still spent). The Watcher uses a headless, read-only, nonpersistent run.
