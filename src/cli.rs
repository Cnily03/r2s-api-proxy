use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "r2s-api-proxy")]
#[command(about = "Reverse proxy for Ret2Shell API with fixed token.")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Args {
    /// The endpoint to proxy requests to
    #[arg(long)]
    pub endpoint: String,

    /// Authorization keys (can be specified multiple times)
    #[arg(long)]
    pub key: Vec<String>,

    /// Ping interval in seconds
    #[arg(short = 'i', long, default_value = "1800")]
    pub ping_interval: u64,

    /// Host to listen on
    #[arg(short = 'H', long, default_value = "0.0.0.0")]
    pub host: String,

    /// Port to listen on
    #[arg(short = 'p', long, default_value = "8080")]
    pub port: u16,

    /// Base path for the proxy
    #[arg(long, default_value = "/", value_parser = parse_base_path)]
    pub base: String,

    /// Configuration path
    #[arg(short = 'd', long, default_value_t = {
        dirs::home_dir()
            .map(|home| format!("{}/.r2s-api-proxy", home.display()))
            .unwrap_or_else(|| "/data/r2s-api-proxy".to_string())
    })]
    pub cache_dir: String,
}

fn parse_base_path(s: &str) -> Result<String, String> {
    if s.is_empty() {
        Ok("/".to_string())
    } else if !s.starts_with('/') {
        Ok(format!("/{}", s))
    } else {
        Ok(s.to_string())
    }
}
