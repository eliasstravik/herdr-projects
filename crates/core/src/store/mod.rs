//! Tasks on disk, locks and settings. No herdr calls here: this part only knows files. One folder per task:
//! <data>/tasks/<repo>/<n>/ holding task.json, inbox.jsonl and artifacts/. task.json is the whole truth the board owns
//! about a task; what herdr knows (agent status) is read live.
pub mod archived_repos;
pub mod attached;
pub mod attempts;
pub mod automations;
pub mod check_runs;
pub mod failures;
pub mod fetch;
pub mod files;
pub mod handovers;
pub mod inbox;
pub mod key_gen;
pub mod keys;
pub mod keys_given;
pub mod marks;
pub mod move_home;
pub mod never_real;
pub mod old_copy;
pub mod old_installs;
pub mod old_research;
pub mod old_suggestions;
pub mod repos;
pub mod retries;
pub mod reviews;
pub mod runs;
pub mod said_by;
pub mod settings;
pub mod settings_list;
pub mod task_cache;
pub mod tasks;
pub mod titles;
