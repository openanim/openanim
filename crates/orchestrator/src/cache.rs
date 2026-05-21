//! Artifact cache management.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheManifest {
    pub key: String,
    pub format: String,
    pub size_bytes: u64,
    pub created_at: u64,
}

pub struct ArtifactCache {
    pub cache_dir: PathBuf,
}

impl ArtifactCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Retrieve a cached file path for a key, if it exists and is valid.
    pub fn get(&self, key: &str) -> Option<PathBuf> {
        let manifest_path = self.cache_dir.join(format!("{}.json", key));
        if !manifest_path.exists() {
            return None;
        }

        // Read manifest
        let manifest_str = std::fs::read_to_string(&manifest_path).ok()?;
        let manifest: CacheManifest = serde_json::from_str(&manifest_str).ok()?;

        let cached_file_path = self.cache_dir.join(format!("{}.{}", key, manifest.format));
        if cached_file_path.exists() {
            Some(cached_file_path)
        } else {
            None
        }
    }

    /// Add a new artifact to the cache.
    pub fn put(&self, key: &str, artifact_file: &Path, format: &str) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(&self.cache_dir)?;

        let size_bytes = std::fs::metadata(artifact_file)?.len();
        let cached_file_path = self.cache_dir.join(format!("{}.{}", key, format));
        
        // Copy the artifact
        std::fs::copy(artifact_file, &cached_file_path)?;

        // Write the manifest
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let manifest = CacheManifest {
            key: key.to_string(),
            format: format.to_string(),
            size_bytes,
            created_at,
        };

        let manifest_path = self.cache_dir.join(format!("{}.json", key));
        let manifest_str = serde_json::to_string_pretty(&manifest).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;
        std::fs::write(&manifest_path, manifest_str)?;

        Ok(cached_file_path)
    }

    /// Remove a key from the cache.
    pub fn invalidate(&self, key: &str) -> std::io::Result<()> {
        let manifest_path = self.cache_dir.join(format!("{}.json", key));
        if manifest_path.exists() {
            // Read manifest to get format for clean deletion
            if let Ok(manifest_str) = std::fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = serde_json::from_str::<CacheManifest>(&manifest_str) {
                    let cached_file = self.cache_dir.join(format!("{}.{}", key, manifest.format));
                    if cached_file.exists() {
                        let _ = std::fs::remove_file(cached_file);
                    }
                }
            }
            std::fs::remove_file(manifest_path)?;
        }
        Ok(())
    }

    /// Clear all files in the cache.
    pub fn clear(&self) -> std::io::Result<()> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)?;
            std::fs::create_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_artifact_cache_operations() {
        let dir = tempdir().unwrap();
        let cache = ArtifactCache::new(dir.path().to_path_buf());

        // Create a dummy artifact file
        let dummy_src = dir.path().join("dummy.mp4");
        std::fs::write(&dummy_src, b"fake video bytes").unwrap();

        let key = "test_key_blake3_hash";
        
        // Assert miss
        assert!(cache.get(key).is_none());

        // Assert put success
        let cached_path = cache.put(key, &dummy_src, "mp4").unwrap();
        assert!(cached_path.exists());
        assert_eq!(std::fs::read(&cached_path).unwrap(), b"fake video bytes");

        // Assert hit
        let retrieved_path = cache.get(key).unwrap();
        assert_eq!(retrieved_path, cached_path);

        // Assert invalidate
        cache.invalidate(key).unwrap();
        assert!(cache.get(key).is_none());
        assert!(!cached_path.exists());
    }
}

