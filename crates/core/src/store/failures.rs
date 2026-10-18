//! Failures: workflow diagnostics that live beside the task, not in its one-line message.
use crate::clock::now;
use crate::json::*;
use std::sync::LazyLock;

/// A login or credential diagnostic, not filenames, variable names or status numbers alone.
static AUTH: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)^\s*(?:(?:fatal|error|api error|graphql):\s*)?(?:HTTP(?:/\d(?:\.\d)?)?\s+(?:401|403)\s*:?\s*)?(?:unauthori[sz]ed|forbidden)[.!]?\s*$|\b(?:not (?:logged|signed) in|log ?in required|authentication (?:failed|required|error)|failed to authenticate|could not read Username|terminal prompts disabled|401 unauthori[sz]ed)\b|\b(?:missing|invalid|expired|bad)\s+(?:credentials?|token|api[ _-]?key)\b|\b(?:credentials?|token|api[ _-]?key)\s+(?:(?:is|has|was)\s+)?(?:missing|invalid|expired|required)\b").unwrap());
static REPORTER: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"^\s*(?:#\s*(?:Subtest|Suite):|(?:ok|not ok)\s+\d+\b|[✔✖✓✗▶ℹ])").unwrap());
static ASSERTION: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)\bAssertionError\b|\bexpected\b.*\b(?:received|got|actual)\b|^\s*(?:expected|actual|received|FAIL|not ok)\b").unwrap());
static CODE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r#"^\s*(?:[+-](?:\s|$)|at\s|['"`]|//|/\*|(?:const|let|var|function|class|return|throw|assert|expect)\b)"#).unwrap());
static PLACE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"^\s*(?:\[[^\]]+\]|.*\.[cm]?[jt]sx?):\d+(?::\d+)?\s*$").unwrap());
static ANSI: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)").unwrap());

pub fn strip_ansi(s: &str) -> String { ANSI.replace_all(s, "").into_owned() }
pub fn is_auth_failure(t: &str) -> bool {
    strip_ansi(t).split('\n').map(|l| l.trim_end_matches('\r')).any(|line| {
        if REPORTER.is_match(line) || ASSERTION.is_match(line) || CODE.is_match(line) || PLACE.is_match(line) { return false; }
        match AUTH.find(line) { Some(m) => !line[..m.start()].contains(['\'', '"', '`']), None => false }
    })
}
pub fn bound_detail(detail: &str) -> String {
    if detail.len() <= 65536 { return detail.to_string(); }
    let mut a = 32768; while !detail.is_char_boundary(a) { a -= 1; }
    let mut b = detail.len() - 32768; while !detail.is_char_boundary(b) { b += 1; }
    format!("{}\n… output truncated …\n{}", &detail[..a], &detail[b..])
}
/// A patch that records a failure on a task. `auth` failures are the human's to fix.
pub fn failure_patch(failure: &Obj) -> Obj {
    let auth = failure.at("auth") == &json!(true) || (failure.at("auth") != &json!(false) && is_auth_failure(failure.s("detail")));
    let mut f = failure.clone();
    f.put("auth", auth);
    f.put("detail", bound_detail(&failure.t("detail")));
    f.put("agentCanFix", if auth { false } else { failure.at("agentCanFix") != &json!(false) });
    if auth { f.put("next", "Fix the login or credential, then tell the agent to continue."); }
    f.put("at", now());
    f.put("sentAt", Value::Null);
    obj(json!({ "failure": f }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn auth() {
        assert!(is_auth_failure("fatal: could not read Username for 'https://github.com'"));
        assert!(is_auth_failure("Error: not logged in"));
        assert!(!is_auth_failure("  expected 'not logged in' to equal x"));
        assert!(!is_auth_failure("const token = missing"));
    }
}
