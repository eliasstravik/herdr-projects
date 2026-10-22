# Pictures pasted into the chat: `[Image 1]`

**A picture you paste into the board's text box stands there as `[Image 1]`, not as its long path.** The next one in
the same message is `[Image 2]`, and so on; the next message counts from 1 again. What is sent has the real path in its
place, so the agent opens the picture as before.

```
❯ what is wrong here [Image 1] and here [Image 2]
```

## On the terminal board

- **What becomes a chip:** every picture's path in a paste. That is the path herdr pastes for a picture on your
  clipboard (`…/herdr-clipboard-images-501/client-327-clipboard-…-0.png`), and a picture file dropped in the terminal
  (`/Users/you/Desktop/Screen\ Shot.png`, or in quotes). Several in one paste give several chips, and so does a paste
  with words around them (`<path> also look at <path> here` goes in as `[Image 1] also look at [Image 2] here`). A
  picture is a path that ends in `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp`, `.heic`, `.tif` or `.tiff` and is a
  file on this machine (herdr's own is taken as it is).
- **A path that comes as keys** (a terminal that does not mark a paste as one types it in, key by key): herdr's own is
  a chip the moment its name is whole; any other picture's path once you type a space after it.
- **What stays words:** a path that is no picture or no file here, and `[Image 1]` you type yourself while the
  message has no first picture. A picture whose chip would take a name that already stands as words (`[Image 1]`
  typed or pasted, in the box or in the paste) stays its path, so those words are sent as they are.
- **A space on either side:** the chip never touches the word or the chip before or after it, also when a picture is
  pasted or typed right after another chip.
- **One character:** the arrows step over a chip whole, backspace after it or delete before it takes it out whole, and
  `Ctrl+W` takes it as one word. The cursor never stands inside one. Undo (`Ctrl+-`) brings it back with its picture.
- **In the accent colour**, so it is told apart from your words.
- **Sent as the path:** when the message goes, each chip is the path again, exactly as it was pasted (its quotes and
  `\ ` too). The kept draft has the paths as well: a draft that comes back (the board opened again, words put away
  and brought back) shows its chips again.

## In the chat, once sent

Your messages in the chat, on the terminal board and on the web board, show their pictures as chips too, numbered
per message: `[Image 1] also look at [Image 2] here` (in the accent on the terminal). That holds for the path the board sent, wherever its file is (a picture sent to another
machine's agent is named where it landed), and for `[Image #39]`, which Claude Code writes in its conversation for a
picture it took in. Your words on their way and the same words once the conversation has them stand once.

## In the web board's own chat box

The box under the web board's list where the web UI has no chat of its own to show
([the web board's chat box](web-chat-box.md)) does the same with a picture you paste, drop or pick: it goes into the
message as `[Image 1] ` (its thumbnail stands above the message, as before), and what is sent names it as
`@<path>`, which is how the agent reads it. The chip is one character there too:

- **The caret never stands inside it.** A click, a tap or an arrow that would put it there puts it before or after
  the chip, the way it was going; a selection that starts or ends inside one takes the whole chip.
- **Backspace just after it, or delete just before it,** takes the picture out whole, its tile with it; the tile's
  cross takes the chip out too.
- **A chip changed all the same** (cut in part some other way) is no longer the picture: its picture is taken out,
  tile and all, and the band over the box says `[Image 1] was taken out of the message.` A tile always means its
  picture is sent.
- **Words you change while they are on their way** stay in the box, as before; a picture of theirs that was sent
  stands there as its `@<path>` again, so what stays still names it.

Any other file still goes in as `@<path> `. The draft kept in the browser has the paths, so after a reload it shows them (the tiles are gone by then).

The web UI's own chat, docked under the board most of the time ([the web board's chat](web-chat.md)), is the web UI's:
it puts the `@<path>` in its box itself.

## A picture for another machine

A picture's path is a file of the machine the board runs on. When the words are for an agent or a Coordinator on
another machine, the board sends the picture there first and the words name it where it landed, so the agent there can
open it ([files you attach go with your words](other-machines.md#files-you-attach-go-with-your-words)). This holds for
herdr's clipboard pictures too: herdr keeps them on the machine its server runs on, which is the board's.
