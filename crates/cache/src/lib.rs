//! Content-addressed cache (CAS) abstraction.
//!
//! The engine-level cache can sit above provider caches (e.g. Manim). It is used to
//! dedupe work across providers and to enable incremental builds.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey(pub String);

impl CacheKey {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// The cache key that produced this artifact.
    pub key: CacheKey,
    /// A stable, engine-level logical name (e.g. "scene.mp4").
    pub name: String,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum CacheError {
    #[error("cache miss")]
    Miss,

    #[error("cache failure: {0}")]
    Other(String),
}

/// Minimal CAS interface.
///
/// Initial implementation can be filesystem-based; cloud can swap in S3/GCS.
pub trait Cache: Send + Sync {
    fn get(&self, key: &CacheKey) -> Result<Vec<u8>, CacheError>;
    fn put(&self, key: &CacheKey, bytes: &[u8]) -> Result<(), CacheError>;
}

/// A no-op cache used for early wiring and tests.
#[derive(Default)]
pub struct NullCache;

impl Cache for NullCache {
    fn get(&self, _key: &CacheKey) -> Result<Vec<u8>, CacheError> {
        Err(CacheError::Miss)
    }

    fn put(&self, _key: &CacheKey, _bytes: &[u8]) -> Result<(), CacheError> {
        Ok(())
    }
}

