# Voice calls

**A call is a live conversation with the board's voice agent, about whoever the chat talks to.** You talk, it talks
back, and you can talk over it. When you ask for something, it sends your request to the agent the chat is pointed at
(the Coordinator, the selected task's agent, or a machine's or a repo's coordinator) and says it did. When that agent
answers, it tells you the gist, in a sentence or two. It speaks Swedish and English, as you do.

The voice agent runs at [ElevenLabs](https://elevenlabs.io/agents). It never acts on the board itself: everything goes to
the board's own agents, as if you had typed it in the chat.

## Starting a call

- **The web board:** the **Call** button in the chat, beside Hide. Tap it again (**Hang up**) to end the call. It uses
  the microphone and speaker of the phone or computer the page is open on, with the browser's echo cancelling.
- **The terminal board:** `Ctrl+O`. The key again hangs up. The call runs on the computer you sit at, not on the
  machine the board runs on ([below](#from-the-terminal-board)).

The first call asks the browser for the microphone once. A call is private: it needs the board's private link
(Tailscale), and the page gets a token that is good for one call, never the API key.

## What it does on a call

- **It follows the chat.** Space, moving between tasks, or a tap on the chat's switch changes who the call talks to.
  You can also say it: "talk to HP-52", "the voice task", "back to the Coordinator".
- **Your own words go along.** A message it sends starts with everything you said since the last message, word for
  word (the last 40 things said, if a call runs that long without sending), then one line that says it was spoken, what the voice agent made of it, and that the agent should answer in one
  or two short sentences first. Where your words name an action that was not confirmed ("why did the merge fail?"),
  that line says they are no request to do it.
- **No action you did not ask for.** Approving, merging, stopping and removing go out only when you asked for them,
  the voice agent asked you back, naming the action and the task ("Merge HP-52, right?"), and the last thing you said
  is a plain yes. The board checks this itself, from what was said on the call: a question ("what does merge mean?"),
  a no ("no, don't do it"), a yes to something else ("yes, explain it") or a yes about another task is not enough, and
  neither is the voice agent's word that you said yes. A no takes back what you asked before it, and a question that
  offers something else ("would you like an explanation of merge?") does not count. And nothing goes out on the voice
  agent's word alone: when none of your words since the last message reached the board, the message is not sent.
- **Answers are read back.** When the agent the call talks to has answered and its words stood still for a moment, the
  voice agent says the gist. Code, paths and links are left out.
- **Updates in pauses only.** A task that came to need you, became ready to approve or failed since the call began is
  said once, the way `board.notify` tells you. Never while you talk, never over the voice agent: the next pause.
  "Be quiet" stops those updates for the rest of the call; answers to what you sent still come.
- **Quick questions** ("what needs me?") are answered from the board at once, without asking an agent.
- **The chat shows the call:** a line under who you talk to says the call is on, with the last thing said. What was
  sent is in the agent's chat, as if you had typed it.
- A call with nobody saying anything for two minutes hangs up by itself. None is longer than an hour.

## From the terminal board

The terminal board often runs on another machine than the one you sit at (you see it through herdr's own window), and
sometimes on the same one. Either way the call's sound is the computer you sit at: `Ctrl+O` rings it, and the **call
helper** there holds the call right in the terminal's computer, with its microphone and speaker. No browser, no link,
no window.

- The helper is part of herdr-projects. It runs by itself on a computer where herdr-projects is installed and herdr has
  saved machines (the ones in herdr's sidebar), and listens on each of them over SSH, as the board reaches other
  machines ([other machines](other-machines.md)), and on this computer itself. `voice.helper none` turns it off.
- **Which computer takes it.** A board whose herdr window is on the same computer (herdr's Local, in use: on macOS
  input in the last two minutes, on Hyprland a terminal in front) is answered there, by that computer's own helper,
  and no other computer hears of the call. Otherwise every computer listening says how surely you are at it, and the
  best one takes it: herdr's window showing the board's machine in front on Hyprland, then a terminal in front with
  recent input, then recent input. A Mac counts only with input in the last two minutes, never for a terminal left in
  front.
- **The API key stays on the board's machine.** The board's machine starts the call and hands the helper a signed
  address for one conversation over ElevenLabs' WebSocket; the voice agent's tools, what was heard and what to read
  back go between the helper and the board over the helper's SSH connection.
- **The sound.** On Linux the sound server's own tools (PipeWire's `pw-record` and `pw-play`, or `parec` and `pacat`,
  which PipeWire also answers), through its WebRTC echo cancelling (`module-echo-cancel`, loaded for the call and
  taken away after; it needs `pactl`). On macOS a small program of Apple's voice processing, which cancels the echo;
  it is compiled once, the first time, with the Xcode Command Line Tools (`xcode-select --install`), and macOS asks
  once whether the terminal herdr runs in may use the microphone (System Settings, Privacy & Security, Microphone).
- **Headphones** are needed where nothing cancels the echo (no `pactl` on Linux, voice processing not available on
  macOS): the chat says so on the call's line (`on m1 · headphones needed: no echo cancelling there`).
- `herdr-projects voice test`, on the computer you sit at, tries its sound for a call: it plays a short tone, listens
  to the microphone for a moment, and says whether the echo is cancelled.
- **No helper picked it up** after a few seconds: the chat says so. A call the helper could not hold (no sound tools,
  no connection) ends, and the board says why.

## Setting it up

- **The ElevenLabs API key** stays on the machine the board runs on. It is looked for, in this order: the
  `ELEVENLABS_API_KEY` variable, `~/.herdr-projects/settings/elevenlabs.json` (`{"api_key": "…"}`, the place for it),
  herdr-call's own config, your agent-tokens file, a reachy-voice `.env`. The key needs the ElevenAgents write
  permission. Nothing else is set up by hand: the voice agent, its prompt and its tools are made in your ElevenLabs
  account before the first call and kept in step with this program after that.
- **The terminal board's calls** need herdr-projects on the computer you sit at (its call helper), and SSH from there
  to the board's machine with `herdr-projects` on its path, as other machines need.
- `herdr-projects voice` says how it stands: where the key was found (never the key), the helper, how the last call
  sounded, a call that is on. `herdr-projects voice end` hangs up.

| Setting | What it does |
|---|---|
| `voice.language` | `auto` (Swedish and English, as you speak), `sv` or `en` |
| `voice.voice` | an ElevenLabs voice id that agents can use (not one with live moderation), or `none` for the built-in one. A voice ElevenLabs refuses is said in the call, with how to pick another |
| `voice.helper` | `auto`: the call helper runs on a computer with saved machines; `none`: never |

## How it fits together

- **The web board's page** holds its call: the ElevenLabs client in the browser over WebRTC, built into the program
  (`crates/web/src/webui/page/vendor/`, made by `scripts/voice-sdk.sh`), with its own audio worklet so the web UI's
  rules on scripts stay as they are. The web UI's page may connect to ElevenLabs' call servers and nothing else new.
- **The front door** (`/herdr-projects/api/voice/`) starts the web board's call (a token for a page the web UI lets
  drive), runs the voice agent's tools on the board the call belongs to, and says what to read back. The terminal
  board's call runs the same code in the listener on the board's machine (`herdr-projects voice listen`, which the
  helper starts over SSH): it starts the call with its one-time ticket and keeps the call's key.
- **The helper** (`herdr-projects voice helper`) holds the terminal board's call: its sound, its WebSocket to
  ElevenLabs (TLS in Rust, nothing to install), its requests to the board.
- **The call record** (`data/voice/call.json`): one call at a time on a machine, who it talks to, what was said. Its
  ticket and its key are kept only as hashes.
- Cost: ElevenLabs bills the minutes a call is open, on your plan.
