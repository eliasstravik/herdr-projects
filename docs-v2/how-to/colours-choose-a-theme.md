# Choose the board's theme

This page tells you how to select the theme of the board by hand, and how to make the board match Herdr when you open
Herdr from another machine.

By default, `board.theme` is `herdr`, and the board uses the theme that Herdr shows. You do not have to set a theme.

## Select a theme by hand

1. Select a value for `board.theme`: `herdr`, `dark`, `light`, or a theme name from
   [colours](../reference/colours.md#the-boardtheme-setting).
2. Set the value. For example: `herdr-projects set board.theme nord`.
3. Open the board again.

## Make the board match Herdr with `herdr --remote`

With `herdr --remote`, Herdr can show the theme of the client's machine around a board in the theme of this machine.
Do one of these procedures.

Procedure 1:

1. Find the theme name in the `[theme]` table of the client machine's `~/.config/herdr/config.toml`.
2. Set `board.theme` to that name: `herdr-projects set board.theme nord`.

Procedure 2:

1. Give both machines the same `[theme]` table in `~/.config/herdr/config.toml`.

**Note:** A plain `ssh` to this machine, with Herdr started on this machine, does not need this. The client and the
server are on one machine, and the board matches.

## Related pages

- [Colours](../reference/colours.md)
- [Why the board uses these colours](../explanation/colours.md)
- [Change a setting](change-a-setting.md)
- [Settings](../reference/settings.md)
