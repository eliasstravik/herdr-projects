//! What `herdr-projects help` prints: every command, in a few words each.
pub const HELP: &str = r#"herdr-projects: a work board for herdr

  herdr-projects                     open the board here (in herdr: bind a key to the plugin action "open")
  herdr-projects add <title> [--desc TEXT] [--repo NAME|PATH] [--delegate] [--after ID[,ID…]] [--<setting> VALUE ...] [--research|--no-research]
                                      (--research: research only, short for --research.only true. Its agent changes
                                       nothing in the repo; the result is a report. --no-research: not research only,
                                       where a default says it is)
  herdr-projects list [--archived] [--all] [--query TEXT] [--json]
  herdr-projects report <id> --state idle|working|blocked|done|error [--kind question|permission|auth] [--msg TEXT] [--title TEXT]
                         [--progress 0-100] [--eta MINUTES]   (your own task, with --state working: how far, and minutes left)
  herdr-projects describe <id> <text>          write or replace a task's description
  (a title is at most 60 characters: a longer one is shortened at a word, its full text kept in the description)
  (an <id> is repo/134; every repo has a short key, made from its name, and HSW-134 names the same task, in any
   letter case. Another key: herdr-projects set repo.key HSW --repo NAME; the old one still names its tasks)
  herdr-projects delegate <id> [--after ID[,ID…]|--now] [--<setting> VALUE ...] [--research|--no-research]
                                      any work setting, for this task only: --reviewer.agent claude --merge.by itself
                                      (a refused option changes nothing on the task)
                                      --after: start only once those tasks (of any repo here, or of another machine:
                                       vm-str:ELA-3) are done, by itself;
                                      --now: start a task that waits without waiting
  herdr-projects after <id> [ID[,ID…] | --clear]   what a task waits for: shown, changed, or dropped (it starts then)
  herdr-projects tell <id> <text>              send a message to the task's agent
  herdr-projects stop <id>                     stop its agent
  herdr-projects approve <id> [--anyway]       merge finished work that waits for approval (--anyway: also when its review
                                                or its pull request's checks on GitHub failed);
                                                on a task whose plan is ready (plan.approve you): yes to the plan, the build starts.
                                                tell <id> <text> there sends the plan back with those words
  herdr-projects pick <id> <letter>            keep one of a task's several attempts (attempts.by); the others' tabs close
  herdr-projects review <id>                   send the reviewer's findings back to the agent
  herdr-projects retry <id>                    send the saved error back to the agent; on a task a limit stopped: continue
  herdr-projects fix <id>                      send failure details to the agent (never approves a merge)
  herdr-projects finish <id>                   finish anyway, when the agent stopped without a summary or at a limit,
                                                or when the board missed it finishing (its agent at rest, the row still "Finishing")
  herdr-projects details <id>                  failure details, review, next steps, and what it used against its limits
  herdr-projects pin|archive|restore|clear <id>
  herdr-projects set <name> <value> [--repo NAME | --task ID | --automation NAME]
                                      change a setting: for you, for your entry for one repo, for one task, or for an automation
  herdr-projects unset <name> [--repo NAME | --task ID]
                                      back to what it would be without that answer
  herdr-projects show [<id>] [--repo NAME]     every setting and the place it came from; without an id, your own
                                      (a program setting, such as worker.agent, takes claude, codex, pi or another
                                       program herdr can start, as agy/gemini-3-pro; one that cannot do that role is
                                       refused with the reason)
  herdr-projects models [PROGRAM]              the model names each program takes, written as a setting writes them
                                      (codex/<model>); a name it does not take is refused when it is set
  herdr-projects automations [add|show|change|on|off|run|firings|remove] …
                                      tasks made on a schedule or when something happens on the board; each firing adds
                                      a task and puts an agent on it. `herdr-projects automations help` says how
  herdr-projects coordinator                   the Coordinator (the agent behind the board's box): who it is, how it stands
  herdr-projects coordinator tell <text> [--delegate]
                                                say something to it (it is started when it is not running); --delegate is
                                                the board's tab: what it adds from this message gets an agent at once
  herdr-projects coordinator where             where the human was when they last wrote to it from the board (for the Coordinator itself)
  herdr-projects coordinator restart           start its program again, in the same conversation
  herdr-projects coordinator new [--agent KIND[/MODEL]] [--effort E]
                                                a fresh one; with options, that choice is for this one only
  herdr-projects coordinator stop              close it; the next message starts it again where it left off (words still waiting go in then)
  herdr-projects --skill                       instructions an agent can load
  herdr-projects machines                      other machines: whether each answers, its version, or why not and what to do;
                                                and whether another machine could ask this one
  herdr-projects web-ui [link|unlink]          the board inside the herdr web UI: how it stands; point the web UI's shared link at it, or back
  herdr-projects version                       this program's version, and where it is
  herdr-projects install                       write the launcher in the data folder (agents and hooks run it) for this program
  herdr-projects doctor                       older installs on this machine (herdr-superwork, herdr-projects 0.2): take over
                                                what is left of them, and say where things stand
  herdr-projects update [VERSION]              this program, in place, from the newest release (or that version)
  herdr-projects remove-extras                 take out what the plugin added outside its own folders (the how-to
                                                files, its blocks in herdr's config); run it before unlinking
"#;
