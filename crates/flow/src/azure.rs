//! Azure DevOps, the second host beside GitHub: a repo whose origin is at dev.azure.com (ssh.dev.azure.com, or the older
//! visualstudio.com) has its pull requests, their policies, the merge and the base branch's pipeline run through the
//! Azure CLI (`az` with its azure-devops extension), as gh is used for GitHub. Each part is read into the shape gh gives
//! (`pr view --json …`), so the board's own rules (land/, housekeeping/prs.rs) read both hosts the same way. One part
//! per file in azure/: the origin's URL, the `az` program, the pull request, its policies, a build's log and the main
//! pipeline's runs, and what a task shows of them. More: docs/azure-devops.md.
pub mod cli;
pub mod logs;
pub mod policies;
pub mod pr;
pub mod runs;
pub mod shown;
pub mod url;
pub use url::{host, parse, Azure};

/// The host's name, as a message says it: "Azure DevOps" for a pull request there, else "GitHub".
pub fn host_word(pr: &str) -> &'static str { if pr::is_azure(pr) { "Azure DevOps" } else { "GitHub" } }
/// How the host writes a pull request's number: `!1234` on Azure DevOps, `#12` on GitHub.
pub fn numbered(pr: &str, number: &str) -> String { format!("{}{number}", if pr::is_azure(pr) { "!" } else { "#" }) }
