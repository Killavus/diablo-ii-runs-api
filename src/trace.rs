use tracing_subscriber::fmt::format::FmtSpan;

use super::AppResult;

fn env_filter_config() -> String {
    std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "tracing=info,warp=debug,runs_api=debug".to_owned())
}

pub fn setup() -> AppResult<()> {
    color_eyre::install()?;
    let sub = tracing_subscriber::fmt()
        .with_env_filter(env_filter_config())
        .with_span_events(FmtSpan::CLOSE)
        .init();

    Ok(())
}
