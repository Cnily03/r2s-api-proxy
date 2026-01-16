use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init_logger() {
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_target(false)
                .with_level(true)
                .with_ansi(true),
        )
        .with(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();
}
