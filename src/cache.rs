use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, error};

#[derive(Debug, Serialize, Deserialize, Default)]
struct CacheData(HashMap<String, String>);

fn get_cache_path(cache_dir: &str) -> PathBuf {
    PathBuf::from(cache_dir).join("config.json")
}

pub async fn load_cache(cache_dir: &str, endpoint: &str) -> Option<String> {
    let path = get_cache_path(cache_dir);

    if !path.exists() {
        debug!("cache file does not exist");
        return None;
    }

    let content = tokio::fs::read_to_string(&path).await.ok()?;
    let cache: CacheData = serde_json::from_str(&content).ok()?;

    cache.0.get(endpoint).cloned()
}

pub async fn save_cache(cache_dir: &str, endpoint: &str, token: &str) {
    let path = get_cache_path(cache_dir);

    // Create directory if not exists
    if let Some(parent) = path.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            error!("failed to create cache directory: {}", e);
            return;
        }
    }

    // Load existing cache or create new
    let mut cache = if path.exists() {
        tokio::fs::read_to_string(&path)
            .await
            .ok()
            .and_then(|content| serde_json::from_str::<CacheData>(&content).ok())
            .unwrap_or_default()
    } else {
        CacheData::default()
    };

    // Update cache
    cache.0.insert(endpoint.to_string(), token.to_string());

    // Write cache with pretty formatting (2 space indent)
    let json = match serde_json::to_string_pretty(&cache.0) {
        Ok(j) => j,
        Err(e) => {
            error!("failed to serialize cache: {}", e);
            return;
        }
    };

    if let Err(e) = tokio::fs::write(&path, json).await {
        error!("failed to write cache: {}", e);
    }
}
