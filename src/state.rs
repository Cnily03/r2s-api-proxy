use crate::cache;
use crate::cli::Args;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Immutable application state (configuration)
#[derive(Debug, Clone)]
pub struct AppState {
    endpoint: String,
    keys: HashSet<String>,
    cache_dir: String,
}

impl AppState {
    pub fn new(args: &Args) -> Self {
        let keys: HashSet<String> = args.key.iter().cloned().collect();
        Self {
            endpoint: args.endpoint.clone(),
            keys,
            cache_dir: args.cache_dir.clone(),
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn cache_dir(&self) -> &str {
        &self.cache_dir
    }

    pub fn is_key_valid(&self, key: &str) -> bool {
        self.keys.contains(key)
    }
}

/// Mutable token cache (used as Extension)
#[derive(Debug)]
pub struct TokenCache {
    endpoint: String,
    cache_dir: String,
    auth_token: Option<String>,
    valid: bool,
}

impl TokenCache {
    pub fn new(endpoint: String, cache_dir: String) -> Self {
        Self {
            endpoint,
            cache_dir,
            auth_token: None,
            valid: true,
        }
    }

    pub fn auth_token(&self) -> Option<&String> {
        self.auth_token.as_ref()
    }

    pub fn set_auth_token(&mut self, token: String) {
        self.auth_token = Some(token);
    }

    pub async fn store_auth_token(&mut self, token: String) {
        self.auth_token = Some(token.clone());
        cache::save_cache(&self.cache_dir, &self.endpoint, &token).await;
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }

    pub fn set_valid(&mut self, valid: bool) {
        self.valid = valid;
    }
}

pub type SharedTokenCache = Arc<RwLock<TokenCache>>;
