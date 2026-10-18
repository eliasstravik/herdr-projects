# Move the home folder

Use this page to keep the files of the board in a different place than `~/.herdr-projects/`. You can move the whole
folder, or only one part of it.

**Note:** `HERDR_PROJECTS_HOME` is not made to run a second board beside the first. The board key of Herdr and its
events use the folder that Herdr itself started with. There is no built-in way to keep two separate lists on one
machine. See [Several boards](../explanation/other-machines.md#several-boards).

## Move the whole folder

1. Set the variable `HERDR_PROJECTS_HOME` to the new folder.

## Move only one part

Each part has its own variable:

| To move | Set |
|---|---|
| only the data | `HERDR_PROJECTS_DATA_DIR` |
| only the settings | `HERDR_PROJECTS_CONFIG_DIR` |
| only the reports | `HERDR_PROJECTS_REPORTS_DIR` |
| only the folder the Coordinator starts in | `HERDR_PROJECTS_COORDINATOR_DIR` |

1. Find the variable of the part in the table.
2. Set that variable to the new folder.

If you set `HERDR_PROJECTS_DATA_DIR`, `HERDR_PROJECTS_CONFIG_DIR` or `HERDR_PROJECTS_REPORTS_DIR` to a place of your
own, the one-time move from older versions does not move that part.

**Note:** The `XDG_*` variables (`XDG_STATE_HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`) do not choose a place. The
board reads them only to find the files of an older version.

## Related pages

- [The home folder (reference)](../reference/home-folder.md)
- [The home folder (explanation)](../explanation/home-folder.md)
- [Move from an old install](move-from-an-old-install.md)
- [Several boards](../explanation/other-machines.md#several-boards)
