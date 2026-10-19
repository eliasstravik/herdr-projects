//! Where the ElevenLabs API key is found (crates/core/src/voice/key.rs): `ELEVENLABS_API_KEY`, then the board's
//! settings file, nowhere else, and never said; and with none, how to add one, as a call and `herdr-projects voice`
//! say it.
use crate::common;
use crate::voice::put_key;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::voice::call::{self, Target};

#[test]
fn the_key_is_found_in_the_environment_then_the_settings_file_and_never_said() {
    let _w = world();
    let home = sw::store::files::user_home();
    // places only some machines have are never looked in
    for (file, text) in [(".config/agent-tokens/env.sh", "export ELEVENLABS_API_KEY=\"sk_tokens_0123456789abcd\"\n"), (".config/herdr/plugins/config/herdr-call/config.toml", "elevenlabs_api_key = \"sk_other_0123456789\"\n"), ("reachy-voice/.env", "ELEVENLABS_API_KEY=sk_reachy_0123456789\n")] {
        std::fs::create_dir_all(home.join(file).parent().unwrap()).unwrap();
        std::fs::write(home.join(file), text).unwrap();
    }
    assert!(sw::voice::key::find().is_none());
    // the settings file; an empty key there is none
    std::fs::write(sw::voice::key::own_file(), json!({ "api_key": "  " }).to_string()).unwrap();
    assert!(sw::voice::key::find().is_none());
    put_key();
    let k = sw::voice::key::find().unwrap();
    assert_eq!(k.key, "sk_test_0123456789abcdef"); assert!(k.from.ends_with("settings/elevenlabs.json") && !k.from.contains("sk_"), "{}", k.from);
    // the environment comes first; empty, it is not there
    unsafe { std::env::set_var("ELEVENLABS_API_KEY", " sk_env_0123456789abcdef \n") };
    assert_eq!(sw::voice::key::find().unwrap(), sw::voice::key::Key { key: "sk_env_0123456789abcdef".into(), from: "ELEVENLABS_API_KEY in the environment".into() });
    unsafe { std::env::set_var("ELEVENLABS_API_KEY", "") };
    assert_eq!(sw::voice::key::find().unwrap().key, "sk_test_0123456789abcdef");
    unsafe { std::env::remove_var("ELEVENLABS_API_KEY") };
    // what `voice` says names where, never the key
    let (v, said) = sw::calls::command::main(&[]);
    assert!(said.starts_with("ElevenLabs key: found (") && said.contains("elevenlabs.json") && !said.contains("sk_test") && !v.to_string().contains("sk_test"), "{said}");
}

#[test]
fn with_no_key_a_call_says_where_to_make_one_its_permission_and_the_two_places_it_can_go() {
    let _w = world();
    let said = sw::voice::key::missing();
    assert_eq!(said, format!("No ElevenLabs API key on this machine. Make one at https://elevenlabs.io/app/developers/api-keys (Developers, API Keys) with the ElevenAgents permission set to Write, then put it in {} as {{\"api_key\": \"…\"}}, or set ELEVENLABS_API_KEY in the environment herdr-projects runs in, and call again.", sw::store::marks::tilde(&sw::voice::key::own_file().display().to_string())));
    assert!(said.contains("settings/elevenlabs.json"));
    // `herdr-projects voice` says it too, and a call from either board refuses with it
    let (v, out) = sw::calls::command::main(&[]);
    assert!(out.starts_with(&format!("ElevenLabs key: none. {said}")) && v.at("key").is_null(), "{out}");
    let t = Target { name: "the Coordinator".into(), ..Default::default() };
    assert_eq!(sw::calls::ring::ring("terminal", &t, "").unwrap_err(), said);
    assert!(call::get().is_none());
}
