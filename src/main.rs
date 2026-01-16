mod cache;
mod cli;
mod logging;
mod proxy;
mod state;

use axum::{http::StatusCode, routing::any, Extension, Router};
use clap::Parser;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::cli::Args;
use crate::state::{AppState, SharedTokenCache, TokenCache};

fn generate_random_key(keylen: usize) -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..keylen)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    logging::init_logger();

    // Parse CLI arguments
    let mut args = Args::parse();
    debug!("parsed arguments: {:?}", args);

    // Generate fallback key if no keys provided
    if args.key.is_empty() {
        let fallback_key = generate_random_key(32);
        warn!("no key provided, generated fallback key: {}", fallback_key);
        args.key.push(fallback_key);
    }

    info!(
        "loading {} {} for application authorization",
        args.key.len(),
        if args.key.len() == 1 { "key" } else { "keys" }
    );

    // Load .env files
    let _ = dotenvy::from_filename(".env");
    let _ = dotenvy::from_filename(".env.local");

    let state = AppState::new(&args);
    let token_cache: SharedTokenCache = Arc::new(RwLock::new(TokenCache::new(
        args.endpoint.clone(),
        args.cache_dir.clone(),
    )));

    // Perform auth_token validity check
    let valid = check_auth_token(&state, token_cache.clone()).await;

    if !valid {
        token_cache.write().await.set_valid(false);
    }

    // Start ping task if valid
    if valid {
        let ping_cache = token_cache.clone();
        let endpoint = args.endpoint.clone();
        let interval = args.ping_interval;

        tokio::spawn(async move {
            // Initial ping
            proxy::ping(&endpoint, ping_cache.clone()).await;

            let mut interval_timer =
                tokio::time::interval(tokio::time::Duration::from_secs(interval));
            interval_timer.tick().await; // Skip first tick (already done)

            loop {
                interval_timer.tick().await;
                let cache_guard = ping_cache.read().await;
                if !cache_guard.is_valid() {
                    break;
                }
                drop(cache_guard);
                proxy::ping(&endpoint, ping_cache.clone()).await;
            }
        });
    }

    // Build router

    // arg.base / -> /, /*
    // arg.base /api -> /api, /api/, /api/*
    // arg.base /api// -> /api//, /api//*

    let base_trimmed = if args.base.ends_with('/') && args.base.len() > 0 {
        args.base[..args.base.len() - 1].to_string()
    } else {
        args.base.clone()
    };
    let base_with_slash = format!("{}/", base_trimmed);
    let base_wildcard = format!("{}/*path", base_trimmed);

    let mut proxy_router = Router::new()
        .route(&base_wildcard, any(proxy::proxy_handler))
        .route(&base_with_slash, any(proxy::proxy_handler));
    if !base_trimmed.ends_with('/') && !base_trimmed.is_empty() {
        proxy_router = proxy_router.route(&base_trimmed, any(proxy::proxy_handler));
    }

    info!("proxy base endpoint set to: {}", args.base);

    let app = Router::new()
        .merge(proxy_router)
        .fallback(|| async { (StatusCode::NOT_FOUND, "not found") })
        .layer(Extension(token_cache))
        .with_state(state);

    // Start server

    let addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!("server started at {}:{}", args.host, args.port);

    axum::serve(listener, app).await.unwrap();
}

async fn check_auth_token(state: &AppState, token_cache: SharedTokenCache) -> bool {
    let endpoint = state.endpoint().to_string();

    let client = reqwest::Client::new();
    let profile_url = format!("{}/account/profile", endpoint);

    // Try environment variable first
    if let Ok(env_token) = std::env::var("AUTH_TOKEN") {
        let response = client
            .get(&profile_url)
            .header("Authorization", format!("Bearer {}", env_token))
            .send()
            .await;

        if let Ok(resp) = response {
            // Check for Set-Token header
            if let Some(new_token) = resp.headers().get("Set-Token") {
                if let Ok(token_str) = new_token.to_str() {
                    let mut cache_guard = token_cache.write().await;
                    cache_guard.store_auth_token(token_str.to_string()).await;
                    info!(
                        "updated auth_token cache after requesting {}/account/profile",
                        endpoint
                    );
                }
            }

            if resp.status().is_success() {
                let mut cache_guard = token_cache.write().await;
                cache_guard.store_auth_token(env_token.clone()).await;
                info!("using environment auth_token");
                return true;
            }
        }
    }

    // Try cached token
    if let Some(cached_token) = cache::load_cache(state.cache_dir(), &endpoint).await {
        let response = client
            .get(&profile_url)
            .header("Authorization", format!("Bearer {}", cached_token))
            .send()
            .await;

        if let Ok(resp) = response {
            // Check for Set-Token header
            if let Some(new_token) = resp.headers().get("Set-Token") {
                if let Ok(token_str) = new_token.to_str() {
                    let mut cache_guard = token_cache.write().await;
                    cache_guard.store_auth_token(token_str.to_string()).await;
                    info!(
                        "updated auth_token cache after requesting {}/account/profile",
                        endpoint
                    );
                }
            }

            if resp.status().is_success() {
                let mut cache_guard = token_cache.write().await;
                cache_guard.set_auth_token(cached_token);
                info!("using cached auth_token");
                return true;
            }
        }
    }

    // Check if we have any token at all
    let has_env_token = std::env::var("AUTH_TOKEN").is_ok();
    let has_cached = cache::load_cache(state.cache_dir(), &endpoint)
        .await
        .is_some();

    if !has_env_token && !has_cached {
        warn!("warning: auth_token not found");
    } else {
        warn!("warning: all auth_token invalid");
    }

    false
}
