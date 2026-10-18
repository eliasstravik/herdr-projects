# Chatting with a finished task

This page tells what happens when you talk to the agent of a task that the board already reviewed. In short, the
board reviews again only what changed.

## Which chats this is about

This is about a task that is under review, that waits for your yes, or that is done. A chat is your own words to the
agent: from the board, with `tell`, or typed in its own pane. The keys that send a review or a failure back are not
a chat.

The board keeps what the last review covered: the commit that the reviewer saw, with nothing uncommitted on top. When
the turn of the agent ends, the board compares the work with that commit.

## No code changed

The agent answered a question or explained something, and it committed or edited nothing. Then the task goes back to
the stage it was in, with the same review. No check, review or merge starts.

- The row says so after the result of the review: `Pass · 1 minor · Chatting · no changes, review kept`.
- The task panel lists it under that review: `a chat changed no code, review 2 still stands`, with when and how many
  times.

## A review was under way when you wrote

The review continues. The box says `Send message to Agent: the review goes on unless the code changes`. The board
does not put back what the reviewer changed while the agent can be at work in the same folder.

- If the turn of the agent changed nothing, the result of the review stays when the review ends.
- If the turn changed the code, the board stops the review. If the review was already complete, its result no longer
  covers the work. The change then waits, as below.

## The code changed

The board does not review after each message. The row says `Changes made · review starts when you're done`. The
board waits until you are done. You are done when one of these is true:

- You left the chat. You selected another task on the board, or, for a chat in its own pane, Herdr shows a space that
  is not the agent's.
- `review.afterChat` passed (60 seconds) with no new message from you, and the agent is at rest.

Then one review runs on the final state, through all of the finish line: commit, main brought in, check, push and
review. Another message before then starts the quiet time again. If a turn puts the code back as the reviewer saw it,
it puts the task back too.

## What does not change

- The board still reviews what you ask for in a chat before any merge.
- A task that merges by itself never merges while a chat is open, or with changes that were not reviewed.
- When the code changes, the board withdraws an approval that you gave.
- Any chat cancels a merge that waits for the checks on GitHub. Its change waits until you are done, as for any
  chat. After the review, the task waits for your new yes.

## Related pages

- [How work gets done](how-work-gets-done.md)
- [The gate](gate.md)
- [Merge by itself](../how-to/turn-on-auto-merge.md)
