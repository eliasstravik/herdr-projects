//! Where each agent program keeps its own settings and files.
use crate::store::files::user_home;
use std::path::PathBuf;

fn env(k: &str) -> Option<PathBuf> { super::account::variable(k).unwrap_or_else(|| std::env::var(k).ok().filter(|v| !v.is_empty()).map(PathBuf::from)) }
pub fn claude_file() -> PathBuf { super::account::trust_file().or_else(|| env("HERDR_PROJECTS_CLAUDE_JSON")).or_else(|| env("CLAUDE_CONFIG_DIR").map(|p| p.join(".claude.json"))).unwrap_or_else(|| user_home().join(".claude.json")) }
pub fn codex_file() -> PathBuf { env("HERDR_PROJECTS_CODEX_TOML").unwrap_or_else(|| codex_home().join("config.toml")) }
pub fn claude_home() -> PathBuf { super::account::home().or_else(|| env("CLAUDE_CONFIG_DIR")).unwrap_or_else(|| user_home().join(".claude")) }
pub fn codex_home() -> PathBuf { env("CODEX_HOME").unwrap_or_else(|| user_home().join(".codex")) }
pub fn pi_home() -> PathBuf { env("PI_CODING_AGENT_DIR").unwrap_or_else(|| user_home().join(".pi").join("agent")) }
