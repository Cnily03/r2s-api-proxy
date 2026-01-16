use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug, Serialize, Deserialize, Default)]
struct CacheData(HashMap<String, String>);

fn get_cache_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".r2s-api-proxy").join("config.json"))
}

pub async fn load_cache(endpoint: &str) -> Option<String> {
    let path = get_cache_path()?;

    if !path.exists() {
        debug!("cache file does not exist");
        return None;
    }

    let content = tokio::fs::read_to_string(&path).await.ok()?;
    let cache: CacheData = serde_json::from_str(&content).ok()?;

    cache.0.get(endpoint).cloned()
}

pub async fn save_cache(endpoint: &str, token: &str) {
    let Some(path) = get_cache_path() else {
        debug!("could not determine cache path");
        return;
    };

    // Create directory if not exists
    if let Some(parent) = path.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            debug!("failed to create cache directory: {}", e);
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
            debug!("failed to serialize cache: {}", e);
            return;
        }
    };

    if let Err(e) = tokio::fs::write(&path, json).await {
        debug!("failed to write cache: {}", e);
    }
}
