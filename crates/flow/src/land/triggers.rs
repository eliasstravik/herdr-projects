//! Whether a GitHub workflow of the branch runs for a pull request, as GitHub decides it: its `on:` read as YAML (push,
//! pull_request, pull_request_target, each with its branches, paths and types filters) and its filters matched with
//! GitHub's own patterns (`*`, `**`, `?`, `+`, `[]`, `!`). Used by expected.rs.
use yaml_rust2::{Yaml, YamlLoader};

/// A workflow file: its name as GitHub shows it, and the events that start it, each with its filters.
#[derive(Debug, Default)]
pub struct Flow { pub name: String, pub on: Vec<(String, Vec<(String, Vec<String>)>)> }
/// What GitHub matches a workflow against: the branch the pull request merges into, its own branch, and the files it
/// changes (None: not known, so a paths filter is taken to match).
pub struct Pr<'a> { pub base: &'a str, pub branch: &'a str, pub files: Option<Vec<String>> }

fn words(v: &Yaml) -> Vec<String> {
    match v {
        Yaml::String(s) => vec![s.clone()],
        Yaml::Array(a) => a.iter().flat_map(words).collect(),
        Yaml::Integer(n) => vec![n.to_string()],
        Yaml::Real(r) => vec![r.clone()],
        Yaml::Boolean(b) => vec![b.to_string()],
        _ => vec![],
    }
}
/// A workflow's text, read as YAML (every form GitHub takes: `on: push`, `on: [push, pull_request]`, a mapping in
/// blocks or inline, `{pull_request: {paths: ['src/**']}}`). One that cannot be read runs for every pull request, so
/// that its checks are waited for rather than missed.
pub fn parse(file: &str, text: &str) -> Flow {
    let unread = Flow { name: file.to_string(), on: vec![("pull_request".into(), vec![])] };
    let Ok(docs) = YamlLoader::load_from_str(text) else { return unread };
    let Some(doc) = docs.into_iter().next() else { return unread };
    let name = doc["name"].as_str().filter(|n| !n.trim().is_empty()).map(String::from).unwrap_or_else(|| file.to_string());
    let on = match &doc["on"] {
        Yaml::Hash(h) => h.iter().filter_map(|(k, v)| {
            let event = k.as_str()?.to_string();
            let filters = match v { Yaml::Hash(f) => f.iter().filter_map(|(k, v)| Some((k.as_str()?.to_string(), words(v)))).collect(), _ => vec![] };
            Some((event, filters))
        }).collect(),
        v => words(v).into_iter().map(|e| (e, vec![])).collect(),
    };
    Flow { name, on }
}
/// One of GitHub's filter patterns against a branch name or a path.
pub fn glob(pattern: &str, s: &str) -> bool {
    let cs: Vec<char> = pattern.chars().collect();
    let mut re = String::from("^");
    let mut i = 0;
    while i < cs.len() {
        match cs[i] {
            '*' if cs.get(i + 1) == Some(&'*') => { if cs.get(i + 2) == Some(&'/') { re.push_str("(?:.*/)?"); i += 1; } else { re.push_str(".*"); } i += 1; }
            '*' => re.push_str("[^/]*"),
            '?' | '+' => re.push(cs[i]), // zero or one, one or more, of what is before it
            '[' => { while i < cs.len() && cs[i] != ']' { re.push(cs[i]); i += 1; } re.push(']'); }
            c => re.push_str(&regex::escape(&c.to_string())),
        }
        i += 1;
    }
    re.push('$');
    regex::Regex::new(&re).map(|r| r.is_match(s)).unwrap_or(true)
}
/// A list of patterns, `!` ones taking back what came before them: the last that matches decides.
pub fn matches(patterns: &[String], s: &str) -> bool {
    let mut hit = false;
    for p in patterns { if let Some(n) = p.strip_prefix('!') { if glob(n, s) { hit = false; } } else if glob(p, s) { hit = true; } }
    hit
}
fn get<'a>(f: &'a [(String, Vec<String>)], k: &str) -> Option<&'a Vec<String>> { f.iter().find(|(n, _)| n == k).map(|(_, v)| v) }
fn branch_ok(f: &[(String, Vec<String>)], b: &str) -> bool {
    if let Some(p) = get(f, "branches") { return matches(p, b); }
    if let Some(p) = get(f, "branches-ignore") { return !matches(p, b); }
    true
}
fn paths_ok(f: &[(String, Vec<String>)], files: &Option<Vec<String>>) -> bool {
    let Some(files) = files.as_ref().filter(|f| !f.is_empty()) else { return true };
    if let Some(p) = get(f, "paths") { return files.iter().any(|x| matches(p, x)); }
    if let Some(p) = get(f, "paths-ignore") { return files.iter().any(|x| !matches(p, x)); }
    true
}
/// Whether a workflow runs on the pull request's head: never, maybe (its `types` hold only some of the events that bring
/// a head, `opened` for the first, `synchronize` for each one pushed after: GitHub starts no run of it on a head that
/// came the other way, #420), or always.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Runs { Never, Maybe, Always }
fn on_head(types: Option<&Vec<String>>) -> Runs {
    let Some(t) = types else { return Runs::Always };
    let has = |e: &str| t.iter().any(|x| x == e);
    if has("opened") && has("synchronize") { Runs::Always }
    else if ["opened", "synchronize", "reopened", "ready_for_review"].iter().any(|e| has(e)) { Runs::Maybe } else { Runs::Never }
}
/// Does this workflow run for the pull request (or for the pushes to its branch)?
pub fn runs_for(flow: &Flow, pr: &Pr) -> Runs {
    flow.on.iter().map(|(event, f)| match event.as_str() {
        "pull_request" | "pull_request_target" if branch_ok(f, pr.base) && paths_ok(f, &pr.files) => on_head(get(f, "types")),
        "push" if (get(f, "branches").is_some() || get(f, "branches-ignore").is_some() || (get(f, "tags").is_none() && get(f, "tags-ignore").is_none()))
            && branch_ok(f, pr.branch) && paths_ok(f, &pr.files) => Runs::Always,
        _ => Runs::Never,
    }).max().unwrap_or(Runs::Never)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn runs(f: &Flow, p: &Pr) -> bool { runs_for(f, p) != Runs::Never }
    fn pr(files: &[&str]) -> Pr<'static> { Pr { base: "main", branch: "work/1-x", files: Some(files.iter().map(|s| s.to_string()).collect()) } }
    #[test]
    fn a_workflow_runs_for_a_pull_request_as_github_decides_it() {
        let ci = parse("ci.yml", "name: Check\non:\n  push:\n    branches: [\"**\"]\n  pull_request:\njobs:\n  check:\n    runs-on: x\n");
        assert_eq!(ci.name, "Check"); assert!(runs(&ci, &pr(&["a.rs"])));
        let src = parse("s.yml", "on:\n  pull_request:\n    paths:\n      - 'src/**'\n      - '!src/docs/**'\njobs: {}\n");
        assert!(runs(&src, &pr(&["src/a/b.rs"])) && !runs(&src, &pr(&["docs/x.md"])) && !runs(&src, &pr(&["src/docs/x.md"])));
        let rel = parse("r.yml", "on:\n  pull_request:\n    branches: [release]\n");
        assert!(!runs(&rel, &pr(&["a"])));
        let tags = parse("t.yml", "on:\n  push:\n    tags: [\"v*\"]\n  workflow_dispatch:\n");
        assert!(!runs(&tags, &pr(&["a"])));
        let only = parse("o.yml", "on:\n  push:\n    branches: [main]\n");
        assert!(!runs(&only, &pr(&["a"])));
        assert!(runs(&parse("p.yml", "on: [push, pull_request]\n"), &pr(&[])));
        assert!(!runs(&parse("l.yml", "on:\n  pull_request:\n    types: [labeled]\n"), &pr(&["a"])));
        // types that leave out how a head comes (opened, or a push after it): maybe, not always (#420)
        let types = |t: &str| runs_for(&parse("t.yml", &format!("on:\n  pull_request:\n    types: [{t}]\n")), &pr(&["a"]));
        assert_eq!((types("opened, synchronize"), types("labeled, synchronize"), types("ready_for_review"), types("opened")), (Runs::Always, Runs::Maybe, Runs::Maybe, Runs::Maybe));
        assert_eq!(runs_for(&parse("b.yml", "on:\n  push:\n  pull_request:\n    types: [labeled, synchronize]\n"), &pr(&["a"])), Runs::Always, "its pushes run it");
        // the same, written inline
        let src = parse("s.yml", "on: {pull_request: {paths: ['src/**', '!src/docs/**']}}\njobs: {}\n");
        assert!(runs(&src, &pr(&["src/a.rs"])) && !runs(&src, &pr(&["docs/x.md"])) && !runs(&src, &pr(&["src/docs/x.md"])));
        assert!(!runs(&parse("r.yml", "on:\n  pull_request: {branches: [release]}\n"), &pr(&["a"])));
        assert!(runs(&parse("q.yml", "\"on\": {pull_request: {branches: main}}\n"), &pr(&["a"])));
        assert_eq!(parse("x.yml", "name: [unclosed\n").name, "x.yml");
        assert!(runs(&parse("x.yml", "on: {pull_request: [unclosed\n"), &pr(&["a"])), "unreadable: waited for, never missed");
        assert!(glob("**/*.js", "a.js") && glob("**/*.js", "x/y/a.js") && !glob("*.js", "x/a.js") && glob("feature/*", "feature/a") && !glob("feature/*", "feature/a/b"));
    }
}
