# Remove herdr-projects

Use this page to remove herdr-projects from Herdr. You must remove the extras of the plugin before you unlink the
plugin.

## Why you must remove the extras first

Herdr gives a plugin no uninstall hook. Thus Herdr cannot clean up after the plugin. The command
`herdr-projects remove-extras` does this work.

## Remove the plugin

**CAUTION:** Do step 1 before you unlink the plugin. If you unlink the plugin first, the marked blocks stay in your
Herdr `config.toml`.

1. Run this command:

   ```sh
   herdr-projects remove-extras
   ```

2. Unlink the plugin from Herdr.

`herdr-projects remove-extras` does these things:

- It removes the marked blocks (the key and the sidebar rows) from the Herdr `config.toml`.
- It removes the how-to files from the skills folder of each agent program. See [Roles](../reference/roles.md).
- If the Herdr web UI plugin is installed, it points the shared link of the web UI back at the web UI. It also stops
  the front door of the board. See [The web UI](../reference/web-ui.md).

## Point only the web UI link back

If you want to keep the plugin but remove the board from the Herdr web UI, run `herdr-projects web-ui unlink`. It
points the shared link of the web UI back at the web UI. See [The web UI](../reference/web-ui.md).

## What stays outside the home folder

Some files of the board are not in `~/.herdr-projects/`. The list is in
[The home folder](../reference/home-folder.md#what-stays-outside-the-home-folder).

**Note:** The test runner of this repo keeps a cache in `~/.cache/herdr-projects-check`. This cache is only for
work on the code of herdr-projects. You can delete it at any time.

## If you came from the Node version

If you installed the older Node version, run `npm unlink -g herdr-projects`. This removes the Node program from your
PATH. See [Move from an old install](move-from-an-old-install.md).

## Related pages

- [Install and update](install-and-update.md)
- [The home folder](../reference/home-folder.md)
- [Install details](../reference/install-details.md)
- [The web UI](../reference/web-ui.md)
- [Move from an old install](move-from-an-old-install.md)
