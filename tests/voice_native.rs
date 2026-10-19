//! A call held in the terminal's computer (crates/flow/src/calls/native/): ElevenLabs' messages read and written, and
//! the call's loop against a played ElevenLabs, a played microphone and speaker, and a played board. Your words reach
//! the board before the tool that follows them; the voice agent's sound plays and stops when you talk over it; an
//! update is read only in a pause; words that never reached the board stop a message; goodbye ends the call.
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use sw::calls::native::audio::Audio;
use sw::calls::native::board::Board;
use sw::calls::native::engine::{Engine, QUIET_MS};
use sw::calls::native::wire::{self, Event};
use sw::calls::native::ws::Wire;
use sw::json::*;

#[derive(Default)]
struct Net { incoming: VecDeque<String>, sent: Vec<String>, closed: bool }
struct FakeWire(Arc<Mutex<Net>>);
impl Wire for FakeWire {
    fn send(&mut self, text: String) -> Result<(), String> { self.0.lock().unwrap().sent.push(text); Ok(()) }
    fn recv(&mut self) -> Result<Option<String>, String> { Ok(self.0.lock().unwrap().incoming.pop_front()) }
    fn close(&mut self) { self.0.lock().unwrap().closed = true; }
}
#[derive(Default)]
struct Sound { mic: Vec<Vec<u8>>, played: Vec<u8>, flushed: u32, stopped: bool }
struct FakeAudio(Arc<Mutex<Sound>>, bool);
impl Audio for FakeAudio {
    fn mic(&mut self) -> Vec<Vec<u8>> { std::mem::take(&mut self.0.lock().unwrap().mic) }
    fn play(&mut self, pcm: &[u8]) { self.0.lock().unwrap().played.extend_from_slice(pcm); }
    fn flush(&mut self) { self.0.lock().unwrap().flushed += 1; }
    fn aec(&self) -> bool { self.1 }
    fn how(&self) -> String { "a played speaker".into() }
    fn stop(&mut self) { self.0.lock().unwrap().stopped = true; }
}
#[derive(Default)]
struct Desk { asked: Vec<(String, Value)>, heard_fails: bool, feed: Vec<Value> }
struct FakeBoard(Arc<Mutex<Desk>>);
impl Board for FakeBoard {
    fn ask(&self, what: &str, body: Value) -> (u16, Value) {
        let mut d = self.0.lock().unwrap();
        d.asked.push((what.to_string(), body.clone()));
        match what {
            "heard" if d.heard_fails => (503, json!({ "ok": false })),
            "tool" => (200, json!({ "ok": true, "result": format!("ran {}", body.s("name")) })),
            "feed" => (200, json!({ "ok": true, "items": std::mem::take(&mut d.feed) })),
            _ => (200, json!({ "ok": true })),
        }
    }
}
struct Call { e: Engine, net: Arc<Mutex<Net>>, sound: Arc<Mutex<Sound>>, desk: Arc<Mutex<Desk>> }
fn call(aec: bool) -> Call {
    let (net, sound, desk) = (Arc::new(Mutex::new(Net::default())), Arc::new(Mutex::new(Sound::default())), Arc::new(Mutex::new(Desk::default())));
    let e = Engine::new(Box::new(FakeWire(net.clone())), Box::new(FakeAudio(sound.clone(), aec)), Arc::new(FakeBoard(desk.clone())), "the Coordinator");
    Call { e, net, sound, desk }
}
impl Call {
    fn hear(&self, m: Value) { self.net.lock().unwrap().incoming.push_back(m.to_string()); }
    fn sent(&self) -> Vec<Value> { self.net.lock().unwrap().sent.iter().map(|s| serde_json::from_str(s).unwrap()).collect() }
    /// Steps at time `t` until `f` holds (the board answers on threads of their own, in a moment).
    fn until(&mut self, t: i64, f: impl Fn(&Call) -> bool) { for _ in 0..200_000 { self.e.step(t); if f(self) { return; } std::thread::yield_now(); } panic!("never: sent {:?}", self.sent()); }
    fn asked(&self) -> Vec<String> { self.desk.lock().unwrap().asked.iter().map(|(w, b)| format!("{w}:{}", if w == "heard" { b.t("text") } else { b.s("name").to_string() })).collect() }
}
fn audio(id: i64, pcm: &[u8]) -> Value { json!({ "type": "audio", "audio_event": { "event_id": id, "audio_base_64": sw::util::base64(pcm) } }) }

#[test]
fn elevenlabs_messages_are_read_and_written_as_its_websocket_has_them() {
    assert_eq!(wire::parse(&audio(3, &[1, 2, 3, 4]).to_string()), Event::Audio { id: 3, pcm: vec![1, 2, 3, 4] });
    assert_eq!(wire::parse(r#"{"type":"interruption","interruption_event":{"event_id":4}}"#), Event::Interruption { id: 4 });
    assert_eq!(wire::parse(r#"{"type":"ping","ping_event":{"event_id":9}}"#), Event::Ping { id: 9 });
    assert_eq!(wire::parse(r#"{"type":"client_tool_call","client_tool_call":{"tool_call_id":"t1","tool_name":"board_status","parameters":{"a":1}}}"#), Event::Tool { call: "t1".into(), name: "board_status".into(), args: json!({ "a": 1 }) });
    assert_eq!(wire::parse(r#"{"type":"user_transcript","user_transcription_event":{"user_transcript":"hej"}}"#), Event::User("hej".into()));
    assert_eq!(wire::parse(r#"{"type":"vad_score","vad_score_event":{"vad_score":0.8}}"#), Event::Vad(0.8));
    assert_eq!(serde_json::from_str::<Value>(&wire::hello("HP-52's agent")).unwrap(), json!({ "type": "conversation_initiation_client_data", "dynamic_variables": { "target": "HP-52's agent" } }));
    assert_eq!(serde_json::from_str::<Value>(&wire::board_says("HP-52 needs you")).unwrap(), json!({ "type": "user_message", "text": "[board] HP-52 needs you" }));
    assert_eq!(serde_json::from_str::<Value>(&wire::audio(&[0, 1])).unwrap(), json!({ "user_audio_chunk": "AAE=" }));
}

#[test]
fn your_words_reach_the_board_before_the_tool_that_follows_them_and_the_sound_plays_and_stops_when_you_talk() {
    sw::clock::set(Some(1_000_000));
    let mut c = call(true);
    assert_eq!(c.sent()[0].s("type"), "conversation_initiation_client_data", "the first message says who the call talks to");
    // your microphone goes out as it comes
    c.sound.lock().unwrap().mic.push(vec![0, 1]);
    c.e.step(1_000_000);
    assert!(c.sent().iter().any(|m| sw::util::unbase64(m.s("user_audio_chunk")).map(|p| p.len()) == Some(2)));
    // you said something, then the voice agent runs a tool: the board has your words first
    c.hear(json!({ "type": "user_transcript", "user_transcription_event": { "user_transcript": "ask how HP-52 is doing" } }));
    c.hear(json!({ "type": "client_tool_call", "client_tool_call": { "tool_call_id": "t1", "tool_name": "send_message", "parameters": { "message": "How is HP-52 doing?" } } }));
    c.until(1_000_000, |c| c.sent().iter().any(|m| m.s("type") == "client_tool_result"));
    assert_eq!(c.asked().iter().filter(|a| a.starts_with("heard") || a.starts_with("tool")).cloned().collect::<Vec<_>>(), vec!["heard:ask how HP-52 is doing", "tool:send_message"]);
    let r = c.sent().into_iter().find(|m| m.s("type") == "client_tool_result").unwrap();
    assert_eq!((r.s("tool_call_id"), r.s("result")), ("t1", "ran send_message"));
    // the voice agent's sound plays; you talk over it: it stops, and the rest of that turn is not played
    c.hear(audio(1, &[5; 64]));
    c.e.step(1_000_010);
    assert_eq!(c.sound.lock().unwrap().played.len(), 64);
    c.hear(json!({ "type": "interruption", "interruption_event": { "event_id": 1 } }));
    c.hear(audio(1, &[6; 64]));
    c.hear(audio(2, &[7; 32]));
    c.e.step(1_000_020);
    let s = c.sound.lock().unwrap();
    assert_eq!((s.flushed, s.played.len()), (1, 96), "the interrupted turn's late sound is dropped, the next turn's plays");
    drop(s);
    // a ping is answered
    c.hear(json!({ "type": "ping", "ping_event": { "event_id": 42 } }));
    c.e.step(1_000_030);
    assert!(c.sent().iter().any(|m| m.s("type") == "pong" && m.i("event_id") == 42));
    // what the agent said goes to the board too, as the voice's
    c.hear(json!({ "type": "agent_response", "agent_response_event": { "agent_response": "Sent." } }));
    c.until(1_000_040, |c| c.asked().contains(&"heard:Sent.".to_string()));
}

#[test]
fn an_update_is_read_in_a_pause_only_and_goodbye_ends_the_call() {
    sw::clock::set(Some(2_000_000));
    let mut c = call(false);
    c.desk.lock().unwrap().feed = vec![json!({ "kind": "alert", "text": "HP-52 needs you" })];
    // the voice agent speaks: half a second of sound
    c.hear(audio(1, &[1; 16000]));
    let t = 2_000_000 + 1600;
    // the board's answer comes in on a thread of its own: the update waits in the queue
    c.until(t, |c| c.e.queued() == 1);
    assert!(!c.sent().iter().any(|m| m.s("type") == "user_message"), "not over the voice agent");
    // you speak
    c.hear(json!({ "type": "vad_score", "vad_score_event": { "vad_score": 0.9 } }));
    c.e.step(t + 600);
    assert!(!c.sent().iter().any(|m| m.s("type") == "user_message"), "not while you talk");
    // a pause: read
    c.e.step(t + 600 + QUIET_MS + 1);
    let said: Vec<Value> = c.sent().into_iter().filter(|m| m.s("type") == "user_message").collect();
    assert_eq!(said.len(), 1); assert_eq!(said[0].s("text"), "[board] HP-52 needs you");
    // how the sound stands is told to the board: here with no echo cancelling
    let told = |c: &Call| c.desk.lock().unwrap().asked.iter().any(|(w, b)| w == "audio" && b.s("via") == "native" && b.at("aec") == &json!(false));
    c.until(t + 600 + QUIET_MS + 1, told);
    // goodbye: the call ends a moment later, and the board is told
    c.hear(json!({ "type": "client_tool_call", "client_tool_call": { "tool_call_id": "t9", "tool_name": "end_call", "parameters": {} } }));
    sw::clock::set(Some(t + 5000));
    c.e.step(t + 5000);
    c.e.step(t + 5000 + 2600);
    assert_eq!(c.e.ended.as_deref(), Some("said goodbye"));
    assert!(c.asked().contains(&"end:".to_string()) && c.net.lock().unwrap().closed && c.sound.lock().unwrap().stopped);
}

#[test]
fn words_that_never_reached_the_board_stop_a_message_until_you_said_it_again_and_a_hang_up_ends_it() {
    sw::clock::set(Some(3_000_000));
    let mut c = call(true);
    c.desk.lock().unwrap().heard_fails = true;
    c.hear(json!({ "type": "user_transcript", "user_transcription_event": { "user_transcript": "only investigate" } }));
    c.hear(json!({ "type": "client_tool_call", "client_tool_call": { "tool_call_id": "t1", "tool_name": "send_message", "parameters": { "message": "Investigate" } } }));
    c.until(3_000_000, |c| c.sent().iter().any(|m| m.s("tool_call_id") == "t1"));
    let r = c.sent().into_iter().find(|m| m.s("tool_call_id") == "t1").unwrap();
    assert!(r.s("result").starts_with("Not sent: some of the human's words did not reach the board"), "{r}");
    assert!(!c.asked().iter().any(|a| a.starts_with("tool")));
    // said again, and it reached the board: the next message goes
    c.desk.lock().unwrap().heard_fails = false;
    c.hear(json!({ "type": "user_transcript", "user_transcription_event": { "user_transcript": "only investigate, please" } }));
    c.hear(json!({ "type": "client_tool_call", "client_tool_call": { "tool_call_id": "t2", "tool_name": "send_message", "parameters": { "message": "Investigate" } } }));
    c.until(3_000_000, |c| c.sent().iter().any(|m| m.s("tool_call_id") == "t2"));
    assert!(c.asked().contains(&"tool:send_message".to_string()));
    // hung up on the board: the call ends here, and the board is not told again
    c.e.hangup.store(true, std::sync::atomic::Ordering::SeqCst);
    c.e.step(3_000_100);
    assert_eq!(c.e.ended.as_deref(), Some("hung up"));
    assert!(!c.asked().iter().any(|a| a.starts_with("end")));
}

#[test]
fn a_quiet_microphone_is_lifted_by_its_speech_never_by_the_rooms_hum_and_its_loudest_is_kept() {
    use sw::calls::native::level::{rms, Gain};
    let chunk = |amp: i16| -> Vec<u8> { (0..320).flat_map(|i| (if i % 2 == 0 { amp } else { -amp }).to_le_bytes()).collect() };
    let mut g = Gain::default();
    // the room's hum alone: nothing is lifted
    for _ in 0..50 { let mut c = chunk(40); g.apply(&mut c); }
    assert!((g.g - 1.0).abs() < 1e-9);
    // quiet speech (rms 300): lifted toward the target, at most six times, a little at a time
    let mut c = chunk(300); g.apply(&mut c);
    assert!(g.g > 1.0 && g.g < 2.0, "a little at a time: {}", g.g);
    for _ in 0..200 { let mut c = chunk(300); g.apply(&mut c); }
    assert!((g.g - 6.0).abs() < 0.01, "{}", g.g);
    let mut c = chunk(300); let out = g.apply(&mut c);
    assert!((rms(&c) - 1800.0).abs() < 2.0 && (out - 1800.0).abs() < 2.0);
    assert!(g.loudest >= 1799.0, "its loudest is kept for the call's line");
    // loud speech is never lifted past its own level, and nothing clips past 16 bits
    let mut g = Gain::default();
    for _ in 0..200 { let mut c = chunk(20000); g.apply(&mut c); }
    assert!((g.g - 1.0).abs() < 1e-6);
}

#[test]
fn your_microphone_goes_out_also_while_the_voice_agent_speaks_lifted_and_measured_for_the_board() {
    sw::clock::set(Some(4_000_000));
    let mut c = call(true);
    c.hear(audio(1, &[1; 3200]));
    c.e.step(4_000_000);
    c.sound.lock().unwrap().mic.push((0..320).flat_map(|i| (if i % 2 == 0 { 400i16 } else { -400 }).to_le_bytes()).collect());
    c.e.step(4_000_010);
    let sent: Vec<Value> = c.sent().into_iter().filter(|m| m.has("user_audio_chunk")).collect();
    assert_eq!(sent.len(), 1, "never muted while the agent speaks");
    let pcm = sw::util::unbase64(sent[0].s("user_audio_chunk")).unwrap();
    assert_eq!(pcm.len(), 640, "16-bit little-endian, as it came");
    assert!(sw::calls::native::level::rms(&pcm) > 400.0, "a quiet microphone is lifted");
    let r = c.e.report();
    assert!(r.f("mic") > 0.0 && r.has("secs") && r.s("via") == "native");
}
