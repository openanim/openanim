//! Stable content hashing used across the engine.

use blake3::Hasher;

/// A stable, displayable content hash.
///
/// We use BLAKE3 for speed and wide availability.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash(String);

impl ContentHash {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Domain-separated hashing helper.
///
/// Domain separation prevents accidental collisions between different kinds of objects
/// (e.g. IR vs provider config) even if the byte payload matches.
pub struct HashBuilder {
    hasher: Hasher,
}

impl HashBuilder {
    pub fn new(domain: &str) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(domain.as_bytes());
        hasher.update(&[0u8]);
        Self { hasher }
    }

    pub fn update_bytes(mut self, bytes: &[u8]) -> Self {
        self.hasher.update(bytes);
        self
    }

    pub fn update_str(self, s: &str) -> Self {
        self.update_bytes(s.as_bytes())
    }

    pub fn finalize(self) -> ContentHash {
        let out = self.hasher.finalize();
        ContentHash(hex::encode(out.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_separation_changes_hash() {
        let a = HashBuilder::new("ir").update_str("hello").finalize();
        let b = HashBuilder::new("provider").update_str("hello").finalize();
        assert_ne!(a, b);
    }

    #[test]
    fn stable_same_inputs_same_hash() {
        let a = HashBuilder::new("ir").update_str("hello").finalize();
        let b = HashBuilder::new("ir").update_bytes(b"hello").finalize();
        assert_eq!(a, b);
    }
}

