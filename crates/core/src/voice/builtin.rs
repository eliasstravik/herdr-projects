//! The built-in voice in your ElevenLabs account. Warren (agent.rs: VOICE) is a Voice Library voice, so an agent can
//! speak with it only once it is in the account: it is added there the first time it is missing, which takes a key
//! with the Voices permission set to Write. Without that, an account made before March 2026 speaks with Will, the
//! default voice it still has until ElevenLabs retires it at the end of 2026, and tries Warren again a day later.
use super::eleven::request;
use crate::json::*;

/// Warren as the Voice Library shares it: its sharer, and its name.
const OWNER: &str = "64cbc624eb5aab4e95a968e1f41d75402277cca6e549036ed17e56ea33bbbc9e";
pub const NAME: &str = "Warren - Effortless and Cool";
/// Will, the default voice of accounts made before March 2026, until the end of 2026.
pub const OLD: &str = "bIHbv24MWmeRgasZH58o";
/// Speaking with Will, Warren is tried again after this long.
pub const RETRY_MS: i64 = 24 * 3600 * 1000;
/// ElevenLabs' answer that the agent's voice is not in the account.
pub fn not_found(e: &str) -> bool { e.contains("voice_not_found") }
/// Add Warren to the account from the Voice Library. Err: why not, as ElevenLabs said it.
pub fn add(key: &str) -> Result<(), String> {
    request(key, "POST", &format!("/v1/voices/add/{OWNER}/{}", super::agent::VOICE), &json!({ "new_name": NAME })).map(|_| ())
}
/// Neither Warren nor Will in the account, and Warren could not be added (`why`): how to give the call a voice.
pub fn missing(why: &str) -> String {
    format!("The call's built-in voice, {NAME} from ElevenLabs' Voice Library, is not in your ElevenLabs account, and it could not be added: {}. Add it yourself (https://elevenlabs.io/app/voice-library, search Warren, add it to My Voices), give your API key the Voices permission set to Write as well, or pick a voice of your own: herdr-projects set voice.voice <voice id>.", crate::text::cut(why, 300))
}
