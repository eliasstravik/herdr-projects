use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};

use crate::coordinator::{self, OpenOptions};
use crate::paths::{self, Ctx, Env, SessionFlags};
use crate::project::{self, Project, Status};
use crate::runner::RealRunner;
use crate::threads::{self, ResolveArgs, StartArgs};
use crate::{actions, adopt, doctor, inbox, lifecycle, overview, routine, ticker};

#[derive(Parser)]
#[command(name = "herdr-projects", version = crate::VERSION, about = "Projects for herdr")]
struct Cli {
    /// Projects root (default: $HERDR_PROJECTS_ROOT, then config.toml, then ~/.herdr-projects)
    #[arg(long, global = true, value_name = "DIR")]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Args, Clone, Default)]
pub struct SessionArgs {
    /// herdr session name
    #[arg(long, value_name = "NAME", conflicts_with = "socket")]
    session: Option<String>,
    /// herdr socket path
    #[arg(long, value_name = "PATH")]
    socket: Option<PathBuf>,
}

impl From<SessionArgs> for SessionFlags {
    fn from(args: SessionArgs) -> Self {
        SessionFlags {
            session: args.session,
            socket: args.socket,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Create a project folder with its skeleton files
    New {
        name: String,
        #[arg(long, default_value = "")]
        goal: String,
        /// A repository, as PATH or PATH@MACHINE; repeatable
        #[arg(long = "repo", value_name = "PATH[@MACHINE]")]
        repos: Vec<String>,
    },
    /// List projects
    List {
        /// Include archived projects
        #[arg(long)]
        all: bool,
    },
    /// Open a project: its workspace and a coordinator agent in its folder
    Open {
        slug: String,
        /// Herdr agent kind for the coordinator (default: coordinator_agent in PROJECT.md)
        #[arg(long, value_name = "KIND")]
        agent: Option<String>,
        /// Extra argument for the agent CLI, repeatable (for example a model flag)
        #[arg(long = "agent-arg", value_name = "ARG", allow_hyphen_values = true)]
        agent_args: Vec<String>,
        /// Start another coordinator even though one is running
        #[arg(long)]
        new: bool,
        /// Move the project to this session when its recorded socket no longer exists
        #[arg(long)]
        rebind: bool,
        #[command(flatten)]
        session: SessionArgs,
    },
    /// The coordinator: send it a sentence
    Coordinator {
        #[command(subcommand)]
        command: CoordinatorCommand,
    },
    /// Print the digest the coordinator reads at the start of every turn
    Context {
        slug: String,
        /// Print without recording the inbox items as seen
        #[arg(long)]
        peek: bool,
    },
    /// Print threads grouped by what needs you
    Overview {
        slug: Option<String>,
        /// Wait for Enter before exiting (only when on a terminal; used by the popup)
        #[arg(long)]
        wait: bool,
    },
    /// Show only one project's panes in the sidebar, sorted by attention
    Focus { slug: Option<String> },
    /// Clear the sidebar view (herdr holds one, so this clears any tool's view)
    Unfocus {
        #[command(flatten)]
        session: SessionArgs,
    },
    /// Inbox items
    Inbox {
        #[command(subcommand)]
        command: InboxCommand,
    },
    /// Threads: the project's worker agents
    Thread {
        #[command(subcommand)]
        command: ThreadCommand,
    },
    /// Routines: scheduled prompts and watched commands
    Routine {
        #[command(subcommand)]
        command: RoutineCommand,
    },
    /// Pause a project: the ticker skips it and `thread start` is refused
    Pause { slug: String },
    /// Make a paused project active again
    Resume { slug: String },
    /// Archive a project: paused, hidden, tokens cleared, `open` refused
    Archive { slug: String },
    /// Make an archived project active again
    Unarchive { slug: String },
    /// Move a project folder to the trash (no worktree, branch or PR is touched)
    Delete {
        slug: String,
        /// Delete even though coordinator or thread panes are alive
        #[arg(long)]
        force: bool,
    },
    /// Continue the current workspace's agent pane as a new project
    AdoptWorkspace {
        /// Project name (default: the workspace label herdr passes to the action)
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "")]
        goal: String,
        /// The agent pane to adopt
        #[arg(long)]
        pane: String,
        /// The workspace's directory (the project's repo when it is a git repository)
        #[arg(long, default_value = "")]
        workspace_cwd: String,
        #[command(flatten)]
        session: SessionArgs,
    },
    /// Run by herdr's action menu
    #[command(hide = true)]
    Action { id: String },
    /// Run inside a plugin popup pane
    #[command(hide = true)]
    Pane { id: String },
    /// Safety settings
    Safety {
        #[command(subcommand)]
        command: SafetyCommand,
    },
    /// Print the coordinator skill
    Skill,
    /// Check the setup: versions, tools, root, ticker and each project's files and session
    Doctor {
        /// Repair what can be repaired: priming files, uploads/, stale binary paths
        #[arg(long)]
        fix: bool,
        #[command(flatten)]
        session: SessionArgs,
    },
    /// The background ticker
    Ticker {
        #[command(subcommand)]
        command: TickerCommand,
    },
}

#[derive(Subcommand)]
enum InboxCommand {
    /// Move handled items to inbox/done/
    Done {
        slug: String,
        #[arg(value_name = "ITEM_ID", required_unless_present = "all")]
        ids: Vec<String>,
        #[arg(long, conflicts_with = "ids")]
        all: bool,
    },
}

#[derive(Subcommand)]
enum CoordinatorCommand {
    /// Send a sentence to the project's coordinator (the popup's task keys use this)
    Prompt {
        slug: String,
        /// The text; `-` reads standard input
        #[arg(long, value_name = "FILE")]
        text_file: String,
    },
}

#[derive(Subcommand)]
enum ThreadCommand {
    /// Start a thread: a worktree workspace for --repo, else a tab in the project workspace
    Start {
        slug: String,
        #[arg(long)]
        title: String,
        #[arg(long, value_name = "PATH")]
        repo: Option<String>,
        #[arg(long, value_name = "LABEL")]
        machine: Option<String>,
        /// Herdr agent kind (default: thread_agent in PROJECT.md)
        #[arg(long, value_name = "KIND")]
        agent: Option<String>,
        /// Placement: worktree (default with --repo), tab (default without), or checkout (a tab on the repo's main checkout)
        #[arg(long, value_name = "worktree|tab|checkout")]
        kind: Option<String>,
        /// Extra argument for the agent CLI, repeatable (for example --agent-arg --model --agent-arg opus)
        #[arg(long = "agent-arg", value_name = "ARG", allow_hyphen_values = true)]
        agent_args: Vec<String>,
        #[arg(long, value_name = "REF")]
        base: Option<String>,
        /// The task; `-` reads standard input
        #[arg(long, value_name = "FILE")]
        task_file: String,
    },
    /// Bring back a thread whose pane is gone or whose start failed
    Restart {
        slug: String,
        id: String,
        /// Restart with another Herdr agent kind
        #[arg(long, value_name = "KIND")]
        agent: Option<String>,
        /// Replace the extra agent CLI arguments (repeatable; none given keeps the old ones, unless the kind changes)
        #[arg(long = "agent-arg", value_name = "ARG", allow_hyphen_values = true)]
        agent_args: Vec<String>,
    },
    /// Send a follow-up to a thread's agent (recorded in its task file)
    Prompt {
        slug: String,
        id: String,
        /// The text; `-` reads standard input
        #[arg(long, value_name = "FILE")]
        text_file: String,
    },
    /// A thread's Next list: print it, forward line N as a prompt, or add a line
    Next {
        slug: String,
        id: String,
        /// Forward this line (1-based) to the thread as a prompt
        #[arg(long, value_name = "N", conflicts_with = "add")]
        line: Option<usize>,
        /// Add a line to the list
        #[arg(long, value_name = "TEXT")]
        add: Option<String>,
    },
    /// Send Escape to a thread's pane (the harness's own interrupt)
    Stop { slug: String, id: String },
    /// List threads with live state and group
    List {
        slug: String,
        #[arg(long)]
        json: bool,
    },
    /// Show one thread's record, group, note and Next list
    Show {
        slug: String,
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Record an existing local agent pane as a thread of this project
    Adopt {
        slug: String,
        #[arg(long, value_name = "ID")]
        pane: String,
        #[arg(long)]
        title: String,
        /// Optional task; `-` reads standard input
        #[arg(long, value_name = "FILE")]
        task_file: Option<String>,
    },
    /// Record that the user has seen the current report
    Ack { slug: String, id: String },
    /// Resolve a thread (final copy first), or reopen a resolved one
    Resolve {
        slug: String,
        id: String,
        #[arg(long, conflicts_with_all = ["remove_worktree", "skip_copy", "discard_uncopied"])]
        reopen: bool,
        /// Also remove the worktree (never forced; the branch is kept)
        #[arg(long)]
        remove_worktree: bool,
        /// Resolve even though the final copy cannot be made
        #[arg(long)]
        skip_copy: bool,
        /// With --remove-worktree: accept losing what could not be copied
        #[arg(long, requires = "remove_worktree")]
        discard_uncopied: bool,
    },
}

/// `-` is standard input; a relative path is relative to the caller's directory.
fn read_text(file: &str) -> Result<String> {
    use std::io::Read;
    if file == "-" {
        let mut text = String::new();
        std::io::stdin().read_to_string(&mut text)?;
        Ok(text)
    } else {
        std::fs::read_to_string(file).map_err(|e| anyhow::anyhow!("could not read {file}: {e}"))
    }
}

#[derive(Subcommand)]
enum RoutineCommand {
    /// Approve a routine's command (a person at a terminal only)
    Approve { slug: String, name: String },
    /// List routines with their approval status
    List { slug: String },
}

#[derive(Subcommand)]
enum SafetyCommand {
    /// Print the effective safety settings and the config.toml table to edit
    Show { slug: String },
}

#[derive(Subcommand)]
enum TickerCommand {
    /// Start the ticker if it is not running (does nothing when there are no projects)
    Start,
    /// Run the ticker loop in the foreground
    Run,
    /// Ask the running ticker to exit and wait for it
    Stop,
    /// Show the running ticker's version, root and tool resolution
    Status,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let env = Env::from_process()?;
    let config_dir = env.config_dir();
    let root = paths::resolve_root(cli.root.as_deref(), &env, &config_dir)?;
    let runner = RealRunner;
    let ctx = Ctx {
        env: &env,
        root,
        config_dir,
        runner: &runner,
        detached_ticker: true,
    };

    match cli.command {
        Command::New { name, goal, repos } => {
            let repos = repos.iter().map(|arg| project::parse_repo_arg(arg)).collect();
            let project = project::create(&ctx.root, &name, &goal, repos)?;
            let prefix = coordinator::current_prefix(&ctx.root)?;
            project::write_priming(&project, &prefix)?;
            println!("created `{}` at {}", project.slug, project.dir().display());
            println!("next: {prefix} open {}", project.slug);
            Ok(())
        }
        Command::List { all } => {
            for slug in project::list_slugs(&ctx.root) {
                let project = Project::load(&ctx.root, &slug)?;
                let status = project.status();
                if status == Status::Archived && !all {
                    continue;
                }
                let mut counts = std::collections::BTreeMap::new();
                for row in threads::rows(&ctx, &project) {
                    *counts.entry(row.group.rank()).or_insert((row.group.label(), 0)) = (row.group.label(), counts.get(&row.group.rank()).map_or(0, |c: &(&str, usize)| c.1) + 1);
                }
                let summary: Vec<String> = counts.values().map(|(label, n)| format!("{label}: {n}")).collect();
                println!("{slug}\t{status}\t{}", if summary.is_empty() { "no threads".to_string() } else { summary.join(", ") });
            }
            Ok(())
        }
        Command::Open { slug, agent, agent_args, new, rebind, session } => coordinator::open(
            &ctx,
            &slug,
            &OpenOptions {
                session: session.into(),
                rebind,
                agent,
                agent_args,
                new,
            },
        ),
        Command::Coordinator { command } => match command {
            CoordinatorCommand::Prompt { slug, text_file } => {
                let text = read_text(&text_file)?;
                coordinator::prompt(&ctx, &slug, &text)
            }
        },
        Command::Context { slug, peek } => coordinator::context(&ctx, &slug, peek),
        Command::Overview { slug, wait } => overview::run(&ctx, slug.as_deref(), wait),
        Command::Focus { slug } => overview::focus(&ctx, slug.as_deref()),
        Command::Unfocus { session } => overview::unfocus(&ctx, &session.into()),
        Command::Inbox { command } => match command {
            InboxCommand::Done { slug, ids, all } => {
                let project = Project::load(&ctx.root, &slug)?;
                let moved = inbox::done(&project, &ids, all)?;
                println!("{moved} item(s) moved to inbox/done");
                Ok(())
            }
        },
        Command::Thread { command } => match command {
            ThreadCommand::Start { slug, title, repo, machine, agent, kind, agent_args, base, task_file } => {
                let task = read_text(&task_file)?;
                let kind = kind.as_deref().map(crate::thread::Kind::parse).transpose()?;
                let thread = threads::start(&ctx, &slug, StartArgs { title, repo, machine, agent, kind, agent_args, base, task })?;
                println!("{}", serde_json::json!({ "id": thread.id, "kind": thread.kind, "agent": thread.agent, "branch": thread.branch, "pane_id": thread.pane_id }));
                Ok(())
            }
            ThreadCommand::Restart { slug, id, agent, agent_args } => {
                let args = (!agent_args.is_empty()).then_some(agent_args);
                let thread = threads::restart(&ctx, &slug, &id, agent.as_deref(), args)?;
                println!("{} is back in pane {}; the ticker launches its {} agent", thread.id, thread.pane_id, thread.agent);
                Ok(())
            }
            ThreadCommand::Next { slug, id, line, add } => threads::next(&ctx, &slug, &id, line, add.as_deref()),
            ThreadCommand::Stop { slug, id } => threads::stop(&ctx, &slug, &id),
            ThreadCommand::Prompt { slug, id, text_file } => {
                let text = read_text(&text_file)?;
                let state = threads::prompt(&ctx, &slug, &id, &text)?;
                println!("sent to {id} (agent was {state})");
                Ok(())
            }
            ThreadCommand::Adopt { slug, pane, title, task_file } => {
                let task = task_file.map(|file| read_text(&file)).transpose()?;
                let thread = adopt::adopt(&ctx, &slug, &pane, &title, task)?;
                println!("{}", serde_json::json!({ "id": thread.id, "kind": thread.kind, "pane_id": thread.pane_id, "prompt_pending": thread.prompt_pending }));
                Ok(())
            }
            ThreadCommand::List { slug, json } => threads::print_list(&ctx, &slug, json),
            ThreadCommand::Show { slug, id, json } => threads::print_show(&ctx, &slug, &id, json),
            ThreadCommand::Ack { slug, id } => threads::ack(&ctx, &slug, &id),
            ThreadCommand::Resolve { slug, id, reopen, remove_worktree, skip_copy, discard_uncopied } => {
                threads::resolve(&ctx, &slug, &id, &ResolveArgs { reopen, remove_worktree, skip_copy, discard_uncopied })
            }
        },
        Command::Routine { command } => match command {
            RoutineCommand::Approve { slug, name } => {
                let project = Project::load(&ctx.root, &slug)?;
                routine::approve(&ctx.config_dir, &project, &name)
            }
            RoutineCommand::List { slug } => {
                let project = Project::load(&ctx.root, &slug)?;
                let commands = project.safety(&ctx.config_dir)?.routine_commands;
                routine::print_list(&ctx.config_dir, &project, commands);
                Ok(())
            }
        },
        Command::Pause { slug } => lifecycle::set_status(&ctx, &slug, Status::Paused),
        Command::Resume { slug } => {
            if Project::load(&ctx.root, &slug)?.status() == Status::Archived {
                bail!("`{slug}` is archived; use `unarchive`");
            }
            lifecycle::set_status(&ctx, &slug, Status::Active)
        }
        Command::Archive { slug } => lifecycle::set_status(&ctx, &slug, Status::Archived),
        Command::Unarchive { slug } => lifecycle::set_status(&ctx, &slug, Status::Active),
        Command::Delete { slug, force } => lifecycle::delete(&ctx, &slug, force),
        Command::AdoptWorkspace { name, goal, pane, workspace_cwd, session } => {
            adopt::adopt_workspace(&ctx, &adopt::AdoptWorkspace { name, goal, pane, workspace_cwd, session: session.into() })
        }
        Command::Action { id } => actions::run_action(&ctx, &id),
        Command::Pane { id } => actions::run_pane(&ctx, &id),
        Command::Safety { command } => match command {
            SafetyCommand::Show { slug } => {
                let project = Project::load(&ctx.root, &slug)?;
                let safety = project.safety(&ctx.config_dir)?;
                println!("Effective safety settings for `{slug}`:");
                println!("  start_threads = {:?}", safety.start_threads);
                println!("  coordinator_agent_args = {:?}", safety.coordinator_agent_args);
                println!("  thread_agent_args = {:?}", safety.thread_agent_args);
                println!("  routine_commands = {}", safety.routine_commands);
                println!();
                println!("To change one, edit {} by hand and add:", ctx.config_dir.join("config.toml").display());
                println!();
                println!("[safety.{:?}]", project.canonical_dir().to_string_lossy());
                Ok(())
            }
        },
        Command::Skill => {
            print!("{}", include_str!("../skill/COORDINATOR.md"));
            Ok(())
        }
        Command::Doctor { fix, session } => {
            if !doctor::run(&ctx, &session.into(), fix)? {
                bail!("some checks failed");
            }
            Ok(())
        }
        Command::Ticker { command } => match command {
            TickerCommand::Start => ticker::start(&ctx),
            TickerCommand::Run => ticker::run(&ctx),
            TickerCommand::Stop => ticker::stop(&ctx.root),
            TickerCommand::Status => ticker::status(&ctx.root),
        },
    }
}
