# Add your own keys

Use this page to move a key of the board to a different chord. Do this if a key never gets to the board.

Some window managers and terminals use `Alt` for themselves. The `Alt` keys of the board are the keys that you use
most frequently. If such a key does not get to the board, you can give it a chord of your own with the setting
`board.keys`.

## Give a key your own chord

1. Find the name of the key. The name is in `crates/flow/src/board/keys.rs`. The error message also shows it if you
   type a wrong name.
2. Write one pair for each key: the name, then `=`, then the chord.
3. Put commas between the pairs.
4. Set `board.keys` to the pairs:

   ```
   herdr-projects set board.keys "chatMessage=ctrl+o, chatScrollUp=ctrl+up, chatScrollDown=ctrl+down"
   ```

After this, the key answers to your chord only. Its old chord does nothing. The `?` menu, the hint line and the
messages of the board name your chord. This is also true on the web board.

## If the board refuses the chord

The board refuses a chord that a different key uses already. It tells you which key uses it:

```
chatMessage=ctrl+l: ctrl+l is yes's key already
```

To use that chord all the same, move the other key too, in the same value:

```
undo=ctrl+c, close=ctrl+q
```

The board also refuses these chords:

- The key that opens the board.
- The prefix key of Herdr, if the key that opens the board starts with it.

## Rules for your own keys

- You can give a plain character only to a key that is a plain character already: `talk`, `menu`, `sayYes`, `sayNo`.
  The text box types any other plain character.
- You cannot move the keys of the text box itself (`newline`).
- `board.keys` is a setting for you only. You cannot set it for one repo. A file in a repo cannot set it.
- If you make a mistake when you edit your settings file by hand, the mistake has no effect. That key keeps its own
  chord. `herdr-projects show` tells you why.

## Put back every key

To give every key its own chord again, type this command:

```
herdr-projects unset board.keys
```

## Related pages

- [Keys](../reference/keys.md)
- [The hint line](../reference/keys-hint-line.md)
- [Settings](../reference/settings.md)
- [Change a setting](change-a-setting.md)
