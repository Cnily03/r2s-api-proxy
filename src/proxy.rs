use axum::{
    extract::{Extension, OriginalUri, Path, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use tracing::{debug, error, info};

use crate::state::{AppState, SharedTokenCache};

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Proxy-Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

fn unauthorized_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [("Content-Type", "text/plain")],
        "unauthorized",
    )
        .into_response()
}

fn service_unavailable_response() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        [("Content-Type", "text/plain")],
        "service unavailable",
    )
        .into_response()
}

fn internal_server_error_response() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        [("Content-Type", "text/plain")],
        "internal server error",
    )
        .into_response()
}

async fn do_proxy_request(
    state: AppState,
    token_cache: SharedTokenCache,
    method: Method,
    path_and_query: &str,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // Extract and validate bearer token
    let token = match extract_bearer_token(&headers) {
        Some(t) => t,
        None => return unauthorized_response(),
    };

    // Check if key is valid
    if !state.is_key_valid(&token) {
        return unauthorized_response();
    }

    let endpoint = state.endpoint().to_string();

    // Build target URL
    let target_url = format!("{}{}", endpoint, path_and_query);
    debug!(uri = %target_url, "proxying request");

    let client = reqwest::Client::new();

    // Build proxied request headers
    let mut proxy_headers = reqwest::header::HeaderMap::new();
    let mut has_authorization_header = false;
    for (name, value) in headers.iter() {
        if name.as_str().eq_ignore_ascii_case("authorization") {
            has_authorization_header = true;
        }
        if name.as_str().eq_ignore_ascii_case("host") {
            continue;
        }
        if let Ok(v) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
            if let Ok(n) = reqwest::header::HeaderName::from_bytes(name.as_str().as_bytes()) {
                proxy_headers.insert(n, v);
            }
        }
    }

    if !has_authorization_header {
        let cache_guard = token_cache.read().await;

        // Check if service is available
        if !cache_guard.is_valid() {
            return service_unavailable_response();
        }

        let auth_token = match cache_guard.auth_token() {
            Some(t) => t.clone(),
            None => return service_unavailable_response(),
        };
        drop(cache_guard);

        // Set the auth token
        match reqwest::header::HeaderValue::from_str(&format!("Bearer {}", auth_token)) {
            Ok(auth_value) => {
                proxy_headers.insert(reqwest::header::AUTHORIZATION, auth_value);
            }
            Err(_) => {
                error!("failed to construct authorization header");
                return internal_server_error_response();
            }
        }
    }

    // Make the request
    let request_builder = client
        .request(method.clone(), &target_url)
        .headers(proxy_headers)
        .body(body);

    let response = match request_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            debug!(uri = %target_url, "proxy request failed: {}", e);
            return (
                StatusCode::BAD_GATEWAY,
                [("Content-Type", "text/plain")],
                "bad gateway",
            )
                .into_response();
        }
    };

    // Handle Set-Token header
    if let Some(new_token) = response.headers().get("Set-Token") {
        if let Ok(token_str) = new_token.to_str() {
            let mut cache_guard = token_cache.write().await;
            cache_guard.store_auth_token(token_str.to_string()).await;
            info!(
                "updated auth_token cache after requesting {}{}",
                endpoint, path_and_query
            );
        }
    }

    // Build response
    let status = StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::OK);

    let mut response_headers = HeaderMap::new();
    for (name, value) in response.headers().iter() {
        if let Ok(v) = HeaderValue::from_bytes(value.as_bytes()) {
            if let Ok(n) = axum::http::header::HeaderName::from_bytes(name.as_str().as_bytes()) {
                response_headers.insert(n, v);
            }
        }
    }

    let body_bytes = response.bytes().await.unwrap_or_default();

    (status, response_headers, body_bytes).into_response()
}

pub async fn proxy_handler(
    State(state): State<AppState>,
    Extension(token_cache): Extension<SharedTokenCache>,
    path: Option<Path<String>>,
    uri: OriginalUri,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let path = match path {
        Some(Path(p)) => format!("/{}", p),
        None => "/".to_string(),
    };
    let path_and_query = match uri.query() {
        Some(q) => format!("{}?{}", path, q),
        None => path.clone(),
    };
    do_proxy_request(state, token_cache, method, &path_and_query, headers, body).await
}

pub async fn ping(endpoint: &str, token_cache: SharedTokenCache) {
    let cache_guard = token_cache.read().await;
    let auth_token = match cache_guard.auth_token() {
        Some(t) => t.clone(),
        None => {
            debug!("no auth token available for ping");
            return;
        }
    };
    drop(cache_guard);

    let client = reqwest::Client::new();
    let ping_url = format!("{}/ping", endpoint);

    debug!("pinging {}", ping_url);

    let response = client
        .get(&ping_url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .send()
        .await;

    if let Ok(resp) = response {
        // Handle Set-Token header
        if let Some(new_token) = resp.headers().get("Set-Token") {
            if let Ok(token_str) = new_token.to_str() {
                let mut cache_guard = token_cache.write().await;
                cache_guard.store_auth_token(token_str.to_string()).await;
                info!("updated auth_token cache via ping");
            }
        }
        debug!("ping completed with status {}", resp.status());
    } else {
        debug!("ping failed");
    }
}
