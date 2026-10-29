# Get started

Install Shepherdr on the machine that runs herdr, share it on your tailnet, and open it on your phone, tablet or desktop.

Shepherdr stands alone: it talks to herdr and needs nothing else of yours. Agents keep running in herdr terminals; any
CLI herdr can start works (chats are read for Claude Code, Codex and pi).

## Install

Needs herdr 0.9 or later, on Linux or macOS. It runs on [Bun](https://bun.sh): with none found, it gets one pinned,
checksum-checked Bun into `~/.shepherdr/bun/` by itself. Tasks' shared browser also needs Chrome for agent-browser,
once on the main machine: `bin/agent-browser install` (on Linux, `install --with-deps` adds what Chrome needs). Other
machines need nothing installed by hand ([Other machines](machines.md)).

```sh
herdr plugin install eliasstravik/shepherdr     # or link a checkout: herdr plugin link /path/to/shepherdr
```

Its plugin id is `herdr-projects`, the plugin it replaced ([Upgrading](upgrading.md)), so herdr lists it as `herdr-projects (Shepherdr)`
and its actions are `herdr-projects.<action>`. herdr builds it (`bun install`, `bun run build`) and starts it at every
herdr start; the action **Open Shepherdr** starts it too and shows its link. Then run the plugin action
**Share Shepherdr on my tailnet**: it serves the app privately on your tailnet (`tailscale serve`, never public) and
prints the link, `https://<machine>.<tailnet>.ts.net/shepherdr/`. On an iPhone, open it in Safari and add it to the
home screen: alerts need that.

## Install it as an app, with alerts

Install the link as an app on each device, open it, and turn alerts on in Settings → Alerts on → **Turn on for this
device**.

- **iPhone or iPad**: in Safari, Share → **Add to Home Screen**, then open it from the home screen (alerts only work
  there).
- **Mac**: in Safari, File → **Add to Dock**; or in Chrome, the install icon at the right of the address bar (or ⋮ →
  Cast, save and share → **Install page as app**). Chrome alerts come while Chrome runs.
- **Omarchy** (or any Linux): in Chromium, ⋮ → Cast, save and share → **Install page as app**, or Omarchy's own
  Install → Web App with the link. Alerts show through mako while Chromium runs.

Push alerts use this server's existing VAPID keys and the public project URL as their signing contact.
Set `SHEPHERDR_PUSH_CONTACT` to a valid public `https://` URL or `mailto:` address to use your own;
invalid or localhost contacts fall back to the project URL. Changing the contact does not require adding devices again.
Settings → Alerts on shows failures per device, with more details in `shepherdr doctor`.
Warnings clear after that device's next accepted alert or renewed subscription; they are forgotten on restart.
Acceptance by a push service does not prove the device displayed the alert. Expired subscriptions are removed as before.


Tasks' alerts always come; what a space's coordinator says follows that space's Notify (All, Needs me or Off: [Spaces](spaces.md)). Each alert says the task and what it needs (`SHE-63 · Plan to approve`), and the question or the card's choices under
it (`Approve the plan · Send it back`), so you can decide from the lock screen. A tap opens the app on that very card,
lit up for a moment, its buttons ready (a done task: its card in the space's chat). One alert per task: its newer
alert takes the old one's place, and it goes once the task no longer waits on you (with the next alert, or when you
open the app or that card). The app icon counts what waits on you (on the home screen and the Mac's Dock); Linux has
no icon count, so the window title says it: `(3) Shepherdr`. On Android, Mac Chrome and Linux, alerts also carry the
card's first two choices as buttons that answer it at once (never one that needs a note). Alerts buzz only while the
app is closed or in the background.

## Voice

Calls go through an ElevenLabs voice agent (Shepherdr makes one for you, with no recording and no retention). Give it
your key once, on the main machine:

```sh
echo "<your ElevenLabs API key>" | shepherdr secret elevenlabs
```

## Next

- [Upgrading and updates](upgrading.md): coming from herdr-projects, going back, and how Shepherdr keeps itself up to date.
- [Other machines](machines.md): run spaces and tasks on more machines from the same app.
- [Spaces](spaces.md), [Chats](chat.md), [Tasks](tasks.md), [Autopilot](autopilot.md), [Artifacts](artifacts.md), [Shared browser](browser.md) and [Settings](settings.md): how it all works.
