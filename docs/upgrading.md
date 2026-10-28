# Upgrading and updates

## Upgrading from herdr-projects

Shepherdr 2.0 is the next version of herdr-projects: `herdr plugin update` takes you there.

1. Finish or stop the tasks in flight first: their agents keep running, but nothing follows them any more.
2. `herdr plugin update` needs herdr 0.9.4 or later: run `herdr update` first on an older one.
3. Run `herdr plugin update herdr-projects`, then press your old herdr-projects key (or run the action **Open
   Shepherdr**, or restart herdr). Shepherdr starts and shows its link.

On its first start Shepherdr brings nothing over from herdr-projects: `~/.herdr-projects` stays exactly as it was. It
copies what it is about to change into `~/.herdr-projects-backup-<date>/` (with a README.txt), then stops herdr-projects'
keeper, page server and open board, takes its agent skill out of `~/.claude/skills`, `~/.codex/skills` and
`~/.pi/agent/skills`, takes its sidebar rows out of herdr's `config.toml` (its key stays and opens Shepherdr), and
points a tailnet link its page server took over back where it was. It says so once in the home space's chat.

Later updates are `herdr plugin update` too: the next herdr start, or the next press of the open key, swaps the running
Shepherdr for the new version.

## Going back to herdr-projects

herdr-projects 1.2.27 stays on the branch `legacy-herdr-projects` and the tag `v1.2.27`, with its own README. To go back
with your old tasks and settings as they were:

```sh
herdr plugin action invoke herdr-projects.stop                     # stop Shepherdr
herdr plugin install eliasstravik/shepherdr --ref v1.2.27 --yes    # herdr-projects 1.2.27 in its place
cp ~/.herdr-projects-backup-<date>/config.toml ~/.config/herdr/config.toml   # its sidebar rows back
herdr server reload-config
rm ~/.shepherdr/took-over-herdr-projects.json                      # a later upgrade tidies up again
```

If you changed herdr's config since the upgrade, paste the `# >>> herdr-projects sidebar` block from the backup's
`config.toml` back in instead of copying the whole file. Then press the herdr-projects key: the board opens, and its
keeper and agent skill come back by themselves. Shepherdr's own data stays in `~/.shepherdr` for a later try; to
return to Shepherdr, `herdr plugin install eliasstravik/shepherdr --yes`.

## Updates

Shepherdr keeps itself up to date. When its own checkout moves to a new commit (a task merged into main), it makes a
release of that commit in `.releases/`: a frozen copy, with `bun install` run only when the lockfile changed, built and
checked there. Once no voice call is on, it stops new calls and restarts on the release. The server always runs from a
release, never from the checkout, so a later merge can't change what runs; a release that stops as it starts is taken
back. Every open screen then reloads itself to the new version: what you were typing and the files you attached come
back, and it waits until you stop typing, finish a voice call or stop talking to type. A screen reloads only when the
screen itself changed. If an update fails, Shepherdr keeps running the last version, says so at the top of every screen
(tap Why for the error) and tries again on the next commit. It follows the checkout herdr links, so a merge on GitHub
arrives once that checkout pulls it; `shepherdr merge` pulls main there itself after merging a pull request. A change to
the updater itself (`scripts/supervise.ts` and what it imports) starts it again on the new code, with the server, once
that release is past its trial and no voice call is on.

By hand: `bun run build && bun server/main.ts` serves on `http://127.0.0.1:7787` (`SHEPHERDR_PORT` to change it).
