use crate::cli::Args;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Immutable application state (configuration)
#[derive(Debug, Clone)]
pub struct AppState {
    endpoint: String,
    keys: HashSet<String>,
}

impl AppState {
    pub fn new(args: &Args) -> Self {
        let keys: HashSet<String> = args.key.iter().cloned().collect();
        Self {
            endpoint: args.endpoint.clone(),
            keys,
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn is_key_valid(&self, key: &str) -> bool {
        self.keys.contains(key)
    }
}

/// Mutable token cache (used as Extension)
#[derive(Debug, Default)]
pub struct TokenCache {
    auth_token: Option<String>,
    valid: bool,
}

impl TokenCache {
    pub fn new() -> Self {
        Self {
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

    pub fn is_valid(&self) -> bool {
        self.valid
    }

    pub fn set_valid(&mut self, valid: bool) {
        self.valid = valid;
    }
}

pub type SharedTokenCache = Arc<RwLock<TokenCache>>;
