//! Specific Herdr notifications: one per event that needs the user, titled
//! `<Project> · <thread>`, with a sound only when the user is needed.
//! `mute = true` in PROJECT.md silences everything but errors.

use crate::herdr::Herdr;
use crate::paths::Ctx;
use crate::project::{self, Project};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sound {
    None,
    /// A result: a new report, a merge.
    Done,
    /// The user is needed.
    Request,
}

impl Sound {
    fn arg(self) -> &'static str {
        match self {
            Sound::None => "none",
            Sound::Done => "done",
            Sound::Request => "request",
        }
    }
}

pub struct Notifier<'a> {
    herdr: Option<Herdr<'a>>,
    name: String,
    mute: bool,
}

impl<'a> Notifier<'a> {
    pub fn new(ctx: &'a Ctx, project: &Project) -> Notifier<'a> {
        let settings = project.read_project_md().map(|(s, _)| s).unwrap_or_default();
        let herdr = project.coordinator().filter(|c| !c.socket.is_empty() && std::path::Path::new(&c.socket).exists()).map(|c| Herdr::new(ctx.env.herdr_bin(), c.socket, ctx.runner));
        Notifier { herdr, name: project::display_name(&settings.name, &project.slug), mute: settings.mute }
    }

    /// `subject` is a thread id or another short name; `error` notifications
    /// go out even when the project is muted.
    pub fn send(&self, subject: &str, body: &str, sound: Sound, error: bool) {
        if self.mute && !error {
            return;
        }
        let Some(herdr) = &self.herdr else {
            return;
        };
        let title = if subject.is_empty() { self.name.clone() } else { format!("{} · {subject}", self.name) };
        let _ = herdr.call(&["notification", "show", &title, "--body", body, "--sound", sound.arg()], crate::herdr::CALL_TIMEOUT);
    }
}
