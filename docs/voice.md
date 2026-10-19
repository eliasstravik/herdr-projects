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

The terminal board often runs on another machine than the one you sit at (you see it through herdr's own window). The
call's sound has to be on your computer, so `Ctrl+O` rings it:

- **The call helper** on your computer picks it up and opens the call in a small browser window of its own (a Chrome,
  Chromium, Brave or Edge window, with a profile of its own that may use the microphone; without one, your default
  browser). The window closes when the call ends. It has Mute and Hang up too.
- The helper is part of herdr-projects. It runs by itself on a computer where herdr-projects is installed and herdr has
  saved machines (the ones in herdr's sidebar), and listens on each of them over SSH, as the board reaches other
  machines ([other machines](other-machines.md)). `voice.helper none` turns it off.
- **Several computers listening:** the one you are at takes the call (herdr's window, or a terminal, in front; on
  macOS also the last input).
- **No helper picked it up** after a few seconds (or the one that took it could not open it): the chat shows a line to
  Ctrl-click. herdr opens it in the browser of the computer its window runs on, and a tap starts the call there.
- That link holds the call's ticket: good once, for two minutes, and seen by whoever sees the terminal board (also a
  device the web UI lets only watch, in its terminal view). Hang up with the key if a call you did not take goes live.

The browser's echo cancelling works with the computer's own speakers. In a loud room, or where the voice agent seems
to hear itself, use headphones.

## Setting it up

- **The ElevenLabs API key** stays on the machine the board runs on. It is looked for, in this order: the
  `ELEVENLABS_API_KEY` variable, `~/.herdr-projects/settings/elevenlabs.json` (`{"api_key": "…"}`, the place for it),
  herdr-call's own config, your agent-tokens file, a reachy-voice `.env`. The key needs the ElevenAgents write
  permission. Nothing else is set up by hand: the voice agent, its prompt and its tools are made in your ElevenLabs
  account before the first call and kept in step with this program after that.
- **The terminal board's calls** open their page from the board's front door, at the herdr web UI's private link
  ([the board in the web UI](web-ui.md)).
- `herdr-projects voice` says how it stands: where the key was found (never the key), the call page, the helper, a call
  that is on. `herdr-projects voice end` hangs up.

| Setting | What it does |
|---|---|
| `voice.language` | `auto` (Swedish and English, as you speak), `sv` or `en` |
| `voice.voice` | an ElevenLabs voice id that agents can use (not one with live moderation), or `none` for the built-in one. A voice ElevenLabs refuses is said in the call, with how to pick another |
| `voice.helper` | `auto`: the call helper runs on a computer with saved machines; `none`: never |

## How it fits together

- **The page** holds the call: the ElevenLabs client in the browser over WebRTC, built into the program
  (`crates/web/src/webui/page/vendor/`, made by `scripts/voice-sdk.sh`), with its own audio worklet so the web UI's
  rules on scripts stay as they are. The web UI's page may connect to ElevenLabs' call servers and nothing else new.
- **The front door** (`/herdr-projects/api/voice/`) starts a call (a token for a page the web UI lets drive, or for the
  page that brings the terminal board's one-time ticket), runs the voice agent's tools on the board the call belongs
  to, and says what to read back.
- **The call record** (`data/voice/call.json`): one call at a time on a machine, who it talks to, what was said. Its
  ticket and its key are kept only as hashes.
- Cost: ElevenLabs bills the minutes a call is open, on your plan.
