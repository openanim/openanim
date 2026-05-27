//! Engine orchestrates IR -> provider plans -> execution.
//!
//! For now we implement the minimal deterministic planning layer:
//! - compute stable cache keys from IR + provider config
//! - decide whether we should execute based on cache presence

use cache::{Cache, CacheError, CacheKey};
use hash::HashBuilder;
use ir::Project;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderSpec {
    /// Provider identifier (e.g. "manim", "ffmpeg", "remotion").
    pub id: String,
    /// Provider version or image digest (pin for determinism).
    pub version: String,
    /// Provider-specific config as stable JSON.
    pub config_json: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderPlan {
    pub provider: ProviderSpec,
    pub key: CacheKey,
    pub needs_run: bool,
}

pub fn compute_cache_key(project: &Project, provider: &ProviderSpec) -> CacheKey {
    // Deterministic serialization: serde_json is stable for structs with fixed field order.
    // For maps, we should use BTreeMap in the IR/config types; config_json should be
    // produced deterministically by callers.
    let ir_json = serde_json::to_vec(project).expect("IR must serialize");

    let h = HashBuilder::new("engine.render.v1")
        .update_bytes(&ir_json)
        .update_str("\n")
        .update_str(&provider.id)
        .update_str("\n")
        .update_str(&provider.version)
        .update_str("\n")
        .update_str(&provider.config_json)
        .finalize();

    CacheKey(h.to_string())
}

pub fn plan_render(project: &Project, provider: ProviderSpec, cache: &dyn Cache) -> RenderPlan {
    let key = compute_cache_key(project, &provider);
    let needs_run = match cache.get(&key) {
        Ok(_) => false,
        Err(CacheError::Miss) => true,
        Err(_) => true,
    };

    RenderPlan {
        provider,
        key,
        needs_run,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cache::NullCache;

    #[test]
    fn plan_needs_run_with_null_cache() {
        let p = Project::new();
        let provider = ProviderSpec {
            id: "manim".to_string(),
            version: "0".to_string(),
            config_json: "{}".to_string(),
        };
        let plan = plan_render(&p, provider, &NullCache);
        assert!(plan.needs_run);
        assert!(!plan.key.0.is_empty());
    }

    #[test]
    fn cache_key_changes_when_provider_changes() {
        let p = Project::new();
        let a = ProviderSpec {
            id: "manim".to_string(),
            version: "1".to_string(),
            config_json: "{}".to_string(),
        };
        let b = ProviderSpec {
            id: "manim".to_string(),
            version: "2".to_string(),
            config_json: "{}".to_string(),
        };
        assert_ne!(compute_cache_key(&p, &a), compute_cache_key(&p, &b));
    }
}

