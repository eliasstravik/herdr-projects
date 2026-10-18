# Your first look at the web board

In this lesson, you open the board in the Herdr web UI for the first time. You find your tasks, open one, and send a
message to the Coordinator from your browser.

## Before you start

- You have a board with some tasks ([your first board](first-board.md)).
- You have the [Herdr web UI](https://github.com/devswha/herdr-web-ui) plugin beside the herdr-projects plugin.
- You shared the web UI with the `tailscale serve` command that its Settings → Phone shows.

If one of these is not true, do the steps in [set up the web board](../how-to/set-up-the-web-ui.md) first.

## Step 1: Show the board in the web UI

1. Open the board in the terminal.
2. When the board asks *Show this board in the herdr web UI?*, select **Yes**.

The board now points the shared link of the web UI at its front door. You do this one time only.

## Step 2: Find the Boards section

1. Open the shared link of the web UI in a browser, on your phone or your desktop.
2. Look in the sidebar for the **Boards** section, under **Agents**.

**Note:** On a phone, the sidebar is a drawer, and the **Boards** section starts folded.

3. If the section is folded, tap its heading to unfold it.

The section has an entry for each machine, and **All machines**. Each entry has a dot in the status colours of the
board, and a short second line, for example `1 needs you · 2 working · 3 to do`.

## Step 3: Open the board of this machine

1. Pick the entry of this machine.

The board of this machine opens over the main area, with its tasks and its Coordinator. On a phone, the drawer closes.

## Step 4: Open a task

1. Tap a task in the list.

The task is selected, with a bar in its gutter. Nothing opens yet.

2. Tap **Open task**.

The task shows whole: its status, what it needs from you, its settings and the other parts of the task panel. The
buttons at the top of the open task are the actions that you can do now.

3. If you use a phone, tap the back arrow of the open task to go back to the list.

## Step 5: Send a message to the Coordinator

The switch over the box at the foot of the page names who gets your words. The Coordinator turns what you type into
tasks.

**Note:** If the switch says `New task`, there is no Coordinator. The board then adds what you type as a task, word
for word.

1. Make sure that the switch says `Coordinator`. If it names the agent of a task, tap it.
2. Tap the box.
3. Type a message for the Coordinator, for example a short description of a new task.
4. Press `Enter`, or tap the round Send.

The board shows a short notice above the chat. The answer of the Coordinator shows in the panel over the box, under
the line `Coordinator:`. The new task shows in the list.

## What you learned

- The **Boards** section in the sidebar of the web UI opens the board of each machine.
- A tap on a task selects it, and **Open task** shows it whole.
- The switch over the box tells who gets your words, and `Enter` sends them.

## Next steps

- Learn what each part of the web board does: [the web board](../reference/web-ui.md).
- Learn the chat box: [the web board's chat box](../reference/web-chat-box.md).
- Show the board of another machine: [set up the web board](../how-to/set-up-the-web-ui.md#show-the-board-of-another-machine).

## Related pages

- [Set up the web board](../how-to/set-up-the-web-ui.md)
- [The web board](../reference/web-ui.md)
- [The web board's chat box](../reference/web-chat-box.md)
- [How the web board works](../explanation/web-ui.md)
- [Your first board](first-board.md)
