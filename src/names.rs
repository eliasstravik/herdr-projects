//! Herdr agent names. `agent start` names must be unique among live agents and
//! match `[a-z][a-z0-9_-]{0,31}` (32 characters), so slugs are truncated.

#[cfg_attr(not(test), allow(dead_code))]
pub const MAX_AGENT_NAME: usize = 32;

fn head(slug: &str, n: usize) -> String {
    let mut s: String = slug.chars().take(n).collect();
    while s.ends_with('-') {
        s.pop();
    }
    s
}

/// `hpc-<slug truncated to 24>` for the first coordinator of a project,
/// `hpc-<slug truncated to 22>-N` for later ones (`n` ≥ 1).
pub fn coordinator(slug: &str, n: u32) -> String {
    if n == 0 {
        format!("hpc-{}", head(slug, 24))
    } else {
        format!("hpc-{}-{n}", head(slug, 22))
    }
}

/// `hp-<slug truncated to 21>-<id>` (`t-0009` is 6 characters).
pub fn thread(slug: &str, id: &str) -> String {
    format!("hp-{}-{id}", head(slug, 21))
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn is_valid(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-'))
        && name.len() <= MAX_AGENT_NAME
}

/// The first coordinator name that is not among `taken`.
pub fn free_coordinator(slug: &str, taken: &[String]) -> String {
    (0..).map(|n| coordinator(slug, n)).find(|name| !taken.contains(name)).unwrap_or_default()
}

/// Two slugs whose thread or coordinator names collide after truncation.
pub fn collide(a: &str, b: &str) -> bool {
    a != b && (coordinator(a, 0) == coordinator(b, 0) || thread(a, "t-0001") == thread(b, "t-0001"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_fit_herdrs_limit_for_the_longest_slug() {
        let slug = "a".repeat(40);
        for name in [coordinator(&slug, 0), coordinator(&slug, 99), thread(&slug, "t-12345")] {
            assert!(is_valid(&name), "{name} ({})", name.len());
        }
        assert_eq!(coordinator("demo", 0), "hpc-demo");
        assert_eq!(coordinator("demo", 2), "hpc-demo-2");
        assert_eq!(thread("demo", "t-0001"), "hp-demo-t-0001");
        // A truncation never leaves a trailing hyphen before the suffix.
        assert_eq!(coordinator("aaaaaaaaaaaaaaaaaaaaaaa-bbbb", 0), "hpc-aaaaaaaaaaaaaaaaaaaaaaa");
        assert!(!is_valid("Hp-x"));
        assert!(!is_valid(""));
    }

    #[test]
    fn free_names_skip_taken_ones_and_collisions_are_detected() {
        let taken = vec!["hpc-demo".to_string(), "hpc-demo-1".to_string()];
        assert_eq!(free_coordinator("demo", &taken), "hpc-demo-2");
        assert_eq!(free_coordinator("demo", &[]), "hpc-demo");
        let long = "x".repeat(30);
        assert!(collide(&format!("{long}-a"), &format!("{long}-b")));
        assert!(!collide("alpha", "beta"));
        assert!(!collide("alpha", "alpha"));
    }
}
