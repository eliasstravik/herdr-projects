# Shared browser

- Every task has a shared browser, the Browser item of its side panel (a full-screen sheet on a phone). Its agent
  drives it with Shepherdr's `bin/agent-browser`, to check its app or sign in to a site, and you watch it live. Take
  over at any time (your mouse, keys, touches and the address bar go in; the agent is told to hold off) and hand it
  back. Fit sizes the page to the panel; Desktop goes back to 1280×720. Mark up: tap elements, write a note
  on each, and send them together into the task's chat; numbered pins mark them on the page, and the agent gets each
  element's selector, text, React components, box, key styles and a cropped shot. It runs on the task's machine: one
  [agent-browser](https://github.com/vercel-labs/agent-browser) session per task (named after it, in the `shepherdr`
  namespace, with a profile in `~/.shepherdr/browser/<task>` that keeps its sign-ins), started when either side first
  uses it. agent-browser streams it on 127.0.0.1 only; the server relays that stream to the app, so the browser is as
  private as the app itself. A task that is Ready keeps it, to review in; stopping or archiving the task closes it
  (until you write to the task again), and merging, closing or deleting it also forgets its sign-ins. One left with no
  agent command and no one watching it in the panel closes too, sign-ins kept: Settings → Browser → Close idle browser
  after (`browser.idleClose`: 10 min, or off; per machine or space). The agent's next command or opening the panel
  starts it again. Its pages see `prefers-reduced-motion: reduce` (Chrome draws them in software, so an animation
  costs a core); an agent whose page must move turns that off with `set media no-preference`. agent-browser is
  pinned in package.json (`bin/agent-browser` runs that version, for agents too: they are told its full path, since a
  shell's start-up files can put another version first on the PATH, and two versions on one session keep restarting
  its daemon); it needs Chrome on the main machine once (`bin/agent-browser install`); other machines get it with
  their helper. Without it the item is greyed, with a
  hint. A browser that won't start says why over the panel, with Try again.
