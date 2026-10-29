# Plugins

Everything is a plugin: places, coordinators, tasks, chat, reactions, artifacts, terminal, worktree, browser, voice,
health, alerts, keyboard, autopilot are the base ones, in `plugins/<name>/` and named in [plugins.json](../plugins.json).
A task's side panel takes its items from plugins too (`taskPanel`: Terminal from terminal; Files, Diff and Pull request
from worktree; Browser from browser). A plugin has a `client.tsx` (screen parts it adds to slots) and/or a `server.ts`.

## Your own plugins

Your own plugins live outside the repo, on the main machine, in `~/.shepherdr/plugins/<name>/` (under
`SHEPHERDR_HOME`), the same shape as a base plugin. Updates never touch that folder, and every release takes them in:
save a file there and Shepherdr rebuilds and restarts within seconds; open screens reload themselves.

- **Add one**: a new folder with a `client.tsx` (and a `server.ts` if you like). Its name: lowercase letters, digits
  and dashes, the same as `name` in its `definePlugin`.
- **Change a built-in one**: copy `plugins/<name>/` to `~/.shepherdr/plugins/<name>/` and edit it. Yours replaces the
  built-in one of that name, in its place; the server's own behaviour for that name stays.
- **Turn any off**: Settings → Plugins has a switch for each, built in or yours. They go in `~/.shepherdr/plugins.json`
  (`{ "off": ["voice"] }`), never in the repo's list, so a merge never undoes them.
- **When yours fails**: one that doesn't build is set aside and the rest updates as usual: its last good version runs
  if it had one, else it is left out (a built-in one it replaced comes back). One that throws as the server loads it is
  skipped there; one that throws on the screen stops only its own parts. A line at the top of the screen says so, and
  Settings → Plugins shows the error. Fix the file and it comes back with the next rebuild.

A hello world, `~/.shepherdr/plugins/hello/client.tsx`:

```tsx
import { definePlugin, useSnap } from '@client/kernel.ts'

function Hello() {
  const snap = useSnap()
  return <div className="bg-bubble px-4 py-1.5 text-center text-[0.8125rem]">Hello: {snap?.tasks.length ?? 0} tasks</div>
}

export default definePlugin({
  name: 'hello',
  setup(reg) {
    reg.topBars.push(Hello)
  },
})
```

and, if it has a server part, `~/.shepherdr/plugins/hello/server.ts`:

```ts
import { definePlugin } from '../../server/plugin-api.ts'

export default definePlugin({ name: 'hello', about: 'Says hello at the top of the screen' })
```

Imports: `@client/…` (the screen's kernel and pieces, `src/`), `@shared/…` (types), `@plugins/<name>/…` (another
plugin's parts: yours where you replaced it, so every plugin gets the same one), and the packages Shepherdr has
(React, clsx, Heroicons). A plugin folder sits where a base one does, so a base plugin's relative imports
(`../../server/plugin-api.ts`) work in a copy too.

**A theme.** There is no theme slot: Shepherdr's colours are CSS variables on `:root` (light) and
`:root[data-theme=dark]` in `src/styles.css` (`--bg`, `--fg`, `--accent` and the rest), so a plugin sets its own by
rendering a `<style>` from the `overlays` slot:

```tsx
import { definePlugin } from '@client/kernel.ts'

const css = ':root{--accent:#2563eb} :root[data-theme=dark]{--accent:#60a5fa}'

export default definePlugin({
  name: 'blue',
  setup(reg) {
    reg.overlays.push(() => <style>{css}</style>)
  },
})
```

**The contract.** `definePlugin` (the screen's in `src/kernel.ts`, the server's in `server/plugin-api.ts`) and the
slots below are public: plugins outside the repo rely on them, so they change only with care.

- `setup(reg)` gets the registry of slots (`Registry` in `src/kernel.ts`, each one documented there):
  - views: `reg.views.places`, `placesMenu` (the sidebar header's buttons), `chat`, `tasks`, `artifacts`, `artifact` (one
    in a split view's pane), `tasksMenu`, `settings`, `search` (one each; the last plugin to set one wins)
  - chats: `messageParts`, `messageEdge` (under and on a message's bubble), `chatTail` (after the messages),
    `chatBody` (replaces a chat's body while its `when` holds)
  - headers and menus: `placeHeader`, `placeMark`, `placeMenu` (a coordinator's), `agentHeader`, `agentMenu` (a task's)
  - the composer: `composerAbove`, `composerLive`, `composerSettings`, `composerIdle`, `composerPlus`, `composerMic`
    (unchanged). The base chat shows launch controls inside its + popup; attachment menu contributions still
    use `composerSettings`. Model/effort changes use `restart-info` and `restart`; legacy `model`/`effort` actions
    return a restart-required error rather than claim an unapplied change. `Chat.agent` is optional launch metadata,
    not telemetry of terminal changes.
  - the screen: `taskPanel` (side panel items), `overlays`, `topBars`, `tasksFooter`, `keys` (keyboard handlers)
  - the lists show what their chips hold, `listView(ui, snap)`: `tasks`, `artifacts` and `tasksMenu` still get the
    place picked (`placeId`, `place`), and `tasksFooter` also gets the `list` shown. `ListView.attention`
    optionally marks global Home’s default actionable view; explicit chips, locks and searches keep normal lists.
    `counts` keeps its legacy totals and adds `need`, `ready`, `ownNeed` and `ownReady` for separate categories. Optional `Snapshot.attentionRequests` lists original unanswered request IDs
    by chat; older machines fall back to one per waiting task/coordinator. Forwarded copies never add to counts
  - Settings: `settingsPages` (a page under This device), `settingActions`
  - items with an `order` are sorted by it; base plugins space theirs out (10, 20, …), so yours can fit between.
- Hooks and actions from `@client/kernel.ts`: `useSnap()` (the server's data: places, tasks, artifacts, settings),
  `useUI()` and `setUI()` (what is on screen), `useStore()`, `getState()`, `act()` (send an action to the server),
  `has(name)` (is a plugin on), and helpers such as `openTask`, `goPlace`, `placeOf`, `taskOf`.
- The server part: `definePlugin({ name, about })`; it runs when the server starts.
