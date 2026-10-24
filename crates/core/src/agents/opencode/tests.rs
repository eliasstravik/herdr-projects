//! OpenCode V2 selects a session before launch, preserves resumed conversations, and refuses failed preparation.
use super::*;
use crate::agents::start::start_args;

fn cwd() -> &'static Path { Path::new(env!("CARGO_MANIFEST_DIR")) }
fn selected() -> Value { json!({ "providerID": "provider", "id": "family/model", "variant": "high" }) }
fn catalog() -> String { json!({ "data": [{ "providerID": "provider", "id": "family/model", "variants": [{ "id": "high" }] }] }).to_string() }

#[test]
fn explicit_model_and_effort_are_selected_before_the_full_screen_launch() {
    let mut created = 0;
    let o = prepare_with(cwd(), StartOpts { model: "provider/family/model".into(), effort: "high".into(), ..Default::default() }, |dir, method, path, body| {
        assert_eq!(dir, cwd().canonicalize().unwrap());
        if method == "get" { assert!(path.starts_with("/api/model?location[directory]=")); return Ok(catalog()); }
        assert_eq!((method, path), ("post", "/api/session"));
        assert_eq!(body.unwrap(), json!({ "location": { "directory": dir }, "model": selected() }));
        created += 1;
        Ok(json!({ "data": { "id": "ses_worker" } }).to_string())
    }).unwrap();
    assert_eq!(created, 1);
    assert_eq!(o.session, "ses_worker");
    assert_eq!(start_args("opencode", &o).unwrap(), ["--session", "ses_worker", "--auto"]);
    let again = prepare_with(cwd(), o, |_, _, _, _| panic!("prepared options must reuse their session")).unwrap();
    assert_eq!(start_args("opencode", &again).unwrap(), ["--session", "ses_worker", "--auto"]);
}

#[test]
fn effort_without_a_model_uses_the_launch_directorys_default() {
    let mut paths = vec![];
    let o = prepare_with(cwd(), StartOpts { effort: "high".into(), ..Default::default() }, |dir, method, path, body| {
        paths.push(path.to_string());
        if path.starts_with("/api/model/default?") { return Ok(json!({ "data": { "providerID": "provider", "id": "family/model" } }).to_string()); }
        if method == "get" { return Ok(catalog()); }
        assert_eq!(body.unwrap(), json!({ "location": { "directory": dir }, "model": selected() }));
        Ok(json!({ "data": { "id": "ses_default" } }).to_string())
    }).unwrap();
    assert!(paths[0].contains("location[directory]="));
    assert_eq!(paths.len(), 3);
    assert_eq!(o.resume, "ses_default");
}

#[test]
fn resume_keeps_its_conversation_and_applies_effort_before_launch() {
    let mut changed = false;
    let o = prepare_with(cwd(), StartOpts { resume: "ses_existing".into(), effort: "high".into(), ..Default::default() }, |_, method, path, body| {
        match (method, path) {
            ("get", "/api/session/ses_existing") => Ok(json!({ "data": { "model": { "providerID": "provider", "id": "family/model" } } }).to_string()),
            ("post", "/api/session/ses_existing/model") => { assert_eq!(body.unwrap(), json!({ "model": selected() })); changed = true; Ok(String::new()) }
            ("get", _) => Ok(catalog()),
            _ => panic!("must not create a replacement conversation"),
        }
    }).unwrap();
    assert!(changed);
    assert_eq!(start_args("opencode", &o).unwrap(), ["--session", "ses_existing", "--auto"]);
}

#[test]
fn unspecified_model_and_effort_are_left_to_opencode() {
    let o = prepare_with(cwd(), StartOpts::default(), |dir, method, path, body| {
        assert_eq!((method, path), ("post", "/api/session"));
        assert_eq!(body.unwrap(), json!({ "location": { "directory": dir } }));
        Ok(json!({ "data": { "id": "ses_default" } }).to_string())
    }).unwrap();
    assert_eq!(o.session, "ses_default");
    let resumed = prepare_with(cwd(), StartOpts { resume: "ses_existing".into(), ..Default::default() }, |_, _, _, _| panic!("unspecified selections must preserve the resumed session")).unwrap();
    assert_eq!(resumed.resume, "ses_existing");
}

#[test]
fn failed_api_and_unsupported_variants_never_produce_launch_options() {
    let opts = || StartOpts { model: "provider/family/model".into(), effort: "high".into(), ..Default::default() };
    let failed = prepare_with(cwd(), opts(), |_, _, _, _| Err("API unavailable".into()));
    assert_eq!(failed.err().unwrap().msg, "API unavailable");
    let unsupported = prepare_with(cwd(), StartOpts { effort: "ultra".into(), ..opts() }, |_, method, _, _| {
        assert_eq!(method, "get", "must refuse the variant before creating a session");
        Ok(catalog())
    });
    assert!(unsupported.err().unwrap().msg.contains("has no effort \"ultra\""));
    for response in ["not JSON", "{}", "{\"data\":{}}"] {
        assert!(prepare_with(cwd(), StartOpts::default(), |_, _, _, _| Ok(response.into())).is_err());
    }
}
