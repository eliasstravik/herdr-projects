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
- **The terminal board:** `Ctrl+T` (t for talk). The key again hangs up. The call runs on the computer you sit at, not on the
  machine the board runs on ([below](#from-the-terminal-board)).

The first call asks the browser for the microphone once. A call is private: it needs the board's private link
(Tailscale), and the page gets a token that is good for one call, never the API key.

## What it does on a call

- **It follows the chat.** Space, moving between tasks, or a tap on the chat's switch changes who the call talks to.
  You can also say it: "talk to HP-52", "the voice task", "back to the Coordinator".
- **Clear messages.** The voice agent writes each message for the coding agent itself: one self-contained
  instruction or question with just what you asked of that agent, in your intent and key wording, with names,
  numbers, file names and task tags kept exactly. Small talk, thinking aloud, what you said to the voice itself and
  what was meant for another agent are left out. Nothing is added to it: the agent gets, and the chat shows, that
  instruction as if you had typed it (with one exception, below). The voice's expression tags ("[neutral]" before a
  sentence) are only in what it speaks, never in the chat or in a message; brackets of your own text ("items[index]",
  "[dependencies] must stay") stay.
- **No action you did not ask for.** The voice agent says what each message asks for: approving, merging, stopping
  or removing a task, or none of those. One of those goes out only when you asked for it, the voice agent asked you
  back, naming the action and the task ("Merge HP-52, right?"), and the last thing you said is a plain yes. The board
  checks this itself, from what was said on the call: a question ("what does merge mean?"), a no ("no, don't do it"),
  a yes to something else ("yes, explain it") or a yes about another task is not enough, and neither is the voice
  agent's word that you said yes. A no takes back what you asked before it, and a question that offers something
  else ("would you like an explanation of merge?") does not count.
- **One action per message, in the board's words.** A confirmed action goes as the board writes it, the action and
  its task alone ("Approve HP-52."), never in the voice agent's words: nothing else it wrote goes along. One that
  names another action, also in other words ("approve HP-52 and ship it"), is not sent; the voice agent sends the
  approval alone and asks about the merge.
- **Everything else goes as asked.** The board does not try to read a sentence for what it orders (a question may
  name an action, an order may hide in many words): a message the voice agent sends with no action goes as written.
  Only when its words name or may order one of the four (merge, ship it, get rid of, kill …) does one short line
  follow it: "(Voice call: nothing was confirmed, so ask the human before you approve, merge, stop or delete.)". What was said is kept on the board for them, never pasted into the message. And
  nothing goes out on the voice agent's word alone: when none of your words since the last message reached the
  board, the message is not sent.
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
sometimes on the same one. Either way the call's sound is the computer you sit at: `Ctrl+T` (t for talk) rings it, and the **call
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
  which PipeWire also answers), through its WebRTC echo cancelling (`module-echo-cancel`, loaded for the call on the
  microphone and speaker in use, mono at 16 kHz with its gain control, and taken away after; it needs `pactl`). A quiet
  microphone is lifted a little, by your speech, never by the room's hum. On macOS a small program of Apple's voice processing, which cancels the echo;
  it is compiled once, the first time, with the Xcode Command Line Tools (`xcode-select --install`), and macOS asks
  once whether the terminal herdr runs in may use the microphone (System Settings, Privacy & Security, Microphone).
- **Headphones** are needed where nothing cancels the echo (no `pactl` on Linux, voice processing not available on
  macOS): the chat says so on the call's line (`on m1 · headphones needed: no echo cancelling there`). So does a
  microphone that stayed near silent for ten seconds (`your microphone is very quiet: the voice may not hear you`).
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
| `voice.llm` | the voice agent's language model at ElevenLabs, or `none` for the built-in one, DeepSeek V4.1 Flash (`deepseek-v41-flash`); speech stays `eleven_v4_turbo` |
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
