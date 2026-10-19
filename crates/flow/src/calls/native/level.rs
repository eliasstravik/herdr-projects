//! How loud your microphone is, and a gentle gain for a quiet one: a chunk with speech in it (well over the room's
//! hum) moves the gain slowly toward a level speech recognition hears well, at most six times; the room's hum is never
//! what the gain is set by. The loudest speech so far is kept, so the call's line can say when the voice agent likely
//! cannot hear you.
/// What speech is lifted toward (16-bit rms), the most it is lifted, and the least that counts as speech.
const TARGET: f64 = 3000.0;
const MOST: f64 = 6.0;
const SPEECH: f64 = 120.0;
pub fn rms(pcm: &[u8]) -> f64 {
    let n = pcm.len() / 2;
    if n == 0 { return 0.0; }
    (pcm.chunks(2).filter(|c| c.len() == 2).map(|c| { let v = i16::from_le_bytes([c[0], c[1]]) as f64; v * v }).sum::<f64>() / n as f64).sqrt()
}
pub struct Gain { pub g: f64, pub loudest: f64 }
impl Default for Gain { fn default() -> Self { Gain { g: 1.0, loudest: 0.0 } } }
impl Gain {
    /// The chunk, lifted; its level as it goes out.
    pub fn apply(&mut self, pcm: &mut [u8]) -> f64 {
        let r = rms(pcm);
        if r > SPEECH { let want = (TARGET / r).clamp(1.0, MOST); self.g = self.g * 0.9 + want * 0.1; }
        if self.g > 1.01 { for c in pcm.chunks_mut(2) { if c.len() == 2 { let v = (i16::from_le_bytes([c[0], c[1]]) as f64 * self.g).clamp(-32768.0, 32767.0) as i16; c.copy_from_slice(&v.to_le_bytes()); } } }
        let out = r * self.g;
        self.loudest = self.loudest.max(out);
        out
    }
}
