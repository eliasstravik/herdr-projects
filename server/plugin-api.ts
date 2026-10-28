// What a base plugin's server part declares.
export interface ServerPlugin {
  name: string
  /** what it adds on the server, in one line */
  about: string
}

export const definePlugin = (p: ServerPlugin) => p
