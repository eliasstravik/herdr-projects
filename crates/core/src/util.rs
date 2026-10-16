//! Small tools the parts share: a link's escaping, a random id, quoting for a shell, versions compared.
use std::io::Read;

/// A text as one part of a link (JavaScript's encodeURIComponent).
pub fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) { out.push(b as char); } else { out.push_str(&format!("%{b:02X}")); }
    }
    out
}
pub fn url_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) { out.push(v); i += 3; continue; }
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
/// `n` random bytes as hex.
pub fn random_hex(n: usize) -> String {
    let mut buf = vec![0u8; n];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut buf)).is_err() {
        let t = crate::clock::now() as u64 ^ (std::process::id() as u64) << 32;
        for (i, b) in buf.iter_mut().enumerate() { *b = (t >> ((i % 8) * 8)) as u8 ^ i as u8; }
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}
/// A random UUID (version 4).
pub fn uuid() -> String {
    let h = random_hex(16);
    let mut c: Vec<char> = h.chars().collect();
    c[12] = '4';
    c[16] = ['8', '9', 'a', 'b'][(c[16].to_digit(16).unwrap_or(0) % 4) as usize];
    let s: String = c.into_iter().collect();
    format!("{}-{}-{}-{}-{}", &s[0..8], &s[8..12], &s[12..16], &s[16..20], &s[20..32])
}
/// A word quoted for a POSIX shell: `'it'\''s'`.
pub fn shq(s: &str) -> String { format!("'{}'", s.replace('\'', "'\\''")) }
/// A folder copied whole into another (made where it is missing).
pub fn copy_dir(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let (src, dst) = (e.path(), to.join(e.file_name()));
        let ft = e.file_type()?;
        if ft.is_dir() { copy_dir(&src, &dst)?; } else if ft.is_symlink() { let target = std::fs::read_link(&src)?; let _ = std::fs::remove_file(&dst); crate::os::symlink(&target, &dst)?; } else { std::fs::copy(&src, &dst)?; }
    }
    Ok(())
}
/// Standard base64, padded: what another machine's door reads its request in.
pub fn base64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 { if i <= c.len() { out.push(A[(n >> (18 - 6 * i) & 63) as usize] as char); } else { out.push('='); } }
    }
    out
}
/// Base64 read back (standard or URL-safe, padded or not). None for anything else.
pub fn unbase64(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> { Some(match c { b'A'..=b'Z' => c - b'A', b'a'..=b'z' => c - b'a' + 26, b'0'..=b'9' => c - b'0' + 52, b'+' | b'-' => 62, b'/' | b'_' => 63, _ => return None } as u32) };
    let s: Vec<u8> = s.trim().bytes().filter(|c| *c != b'=').collect();
    let mut out = vec![];
    for c in s.chunks(4) {
        let mut n = 0u32;
        for (i, b) in c.iter().enumerate() { n |= val(*b)? << (18 - 6 * i); }
        for i in 0..c.len().saturating_sub(1) { out.push((n >> (16 - 8 * i)) as u8); }
    }
    Some(out)
}
/// A regular expression made once, where it is written: `re!(r"\d+")`.
#[macro_export]
macro_rules! re { ($s:expr) => {{ static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new(); R.get_or_init(|| regex::Regex::new($s).unwrap()) }} }
/// An environment variable that says something: set, and not empty (as JavaScript's `process.env.X` reads true).
pub fn env_set(k: &str) -> bool { std::env::var(k).map(|v| !v.is_empty()).unwrap_or(false) }
/// Its value when it says something.
pub fn env_val(k: &str) -> Option<String> { std::env::var(k).ok().filter(|v| !v.is_empty()) }
fn parts(v: &str) -> Vec<u64> { v.trim_start_matches('v').split('.').map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0)).collect() }
/// Versions compared part by part as numbers (0.10.0 is newer than 0.9.1): true when `a` is newer than `b`.
pub fn newer(a: &str, b: &str) -> bool {
    let (x, y) = (parts(a), parts(b));
    for i in 0..x.len().max(y.len()) { let (p, q) = (x.get(i).copied().unwrap_or(0), y.get(i).copied().unwrap_or(0)); if p != q { return p > q; } }
    false
}
