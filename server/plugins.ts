// Server parts of the plugins: the base ones (plugins/) and the human's own (~/.shepherdr/plugins). The lists decide
// which are on; a plugin turned off turns its behaviour off on the server as well as on the screen. One of the human's
// own that throws as it loads is skipped, and Settings → Plugins says why.
import plugins, { catalog, enabled, user } from '../generated/server-plugins.ts'
import type { PluginEntry } from '../scripts/build-plugins.ts'
import { readOff } from '../scripts/build-plugins.ts'
import type { SettingSection } from '../shared/types.ts'
import type { ServerPlugin } from './plugin-api.ts'
import { HOME, writeJson } from './store.ts'

export const has = (name: string) => enabled.includes(name)

/** every plugin this build knows, on or off */
export const pluginCatalog: PluginEntry[] = catalog

/** the human's own server parts that threw as they loaded, by name */
const failed: Record<string, string> = {}

export const serverPlugins: ServerPlugin[] = [...plugins]
for (const [name, load] of user)
  try {
    serverPlugins.push((await load()).default)
  } catch (e) {
    failed[name] = e instanceof Error ? e.message : String(e)
    console.log(`plugin ${name} didn't load on the server, so it is skipped there: ${failed[name]}`)
  }

/** Turn a plugin on or off (the human's list, ~/.shepherdr/plugins.json): Shepherdr restarts with it in seconds. */
export function setPluginOn(name: string, on: boolean): { ok: boolean; error?: string } {
  if (!catalog.some((p) => p.name === name)) return { ok: false, error: `No plugin ${name}.` }
  const off = readOff(HOME).filter((n) => n !== name)
  writeJson('plugins.json', { off: on ? off : [...off, name] })
  return { ok: true }
}

/** Settings → Plugins: each plugin, base and the human's own, with its switch, from this build and the list now */
export const pluginSection = () => pluginsPage(catalog, readOff(HOME), failed)

/** a key of the Plugins page: its switch */
export const PLUGIN_KEY = 'plugin.'

export function pluginsPage(list: PluginEntry[], off: string[], failed: Record<string, string> = {}): SettingSection {
  const lines = (e: string) => e.split('\n').filter(Boolean).slice(0, 3).join('\n')
  return {
    name: 'Plugins',
    about: 'Built in, and your own from ~/.shepherdr/plugins (one with a built-in name replaces it). Shepherdr restarts with a change in seconds.',
    // the built-in ones first, then yours (one that replaces a built-in one among them)
    rows: [...list.filter((p) => p.from === 'base'), ...list.filter((p) => p.from === 'user')].map((p) => {
      const want = !off.includes(p.name)
      const warn = p.broken
        ? p.kept
          ? `Your latest change doesn't build, so its last good version runs:\n${lines(p.broken)}`
          : `${p.from === 'base' ? "Yours doesn't build, so the built-in one runs" : "Skipped: it doesn't build"}:\n${lines(p.broken)}`
        : failed[p.name]
          ? `Skipped on the server: it threw as it loaded:\n${lines(failed[p.name])}`
          : undefined
      return {
        label: p.name,
        key: `${PLUGIN_KEY}${p.name}`,
        value: want ? 'on' : 'off',
        options: ['on', 'off'],
        group: p.from === 'user' ? 'Yours' : 'Built in',
        ...(p.overrides ? { hint: 'Replaces the built-in one' } : {}),
        ...(want !== p.on && !p.broken ? { hint: want ? 'Turning on: restarting' : 'Turning off: restarting' } : {}),
        ...(warn ? { warn } : {}),
      }
    }),
  }
}
