use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tracing::info;

mod api;
mod alsa;
mod alsa_setup;
mod audio;
mod auth;
mod config;
mod decode;
mod events;
mod player;
mod radiobrowser;
mod slot_keeper;
mod state;
mod stream;

#[derive(Parser, Debug)]
#[command(
    name = "iradio-bridge",
    about = "Internet Radio → Dante AoIP bridge for the Inferno appliance",
    version
)]
struct Args {
    #[arg(short, long, default_value = "/etc/iradio/config.toml", env = "IRADIO_CONFIG")]
    config: String,

    #[arg(short, long, env = "IRADIO_PORT")]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "iradio_bridge=info,tower_http=warn".parse().unwrap()),
        )
        .init();

    let args = Args::parse();

    let mut cfg = config::Config::load(&args.config).unwrap_or_else(|e| {
        tracing::warn!("Config load failed ({}), using defaults", e);
        config::Config::default()
    });

    if let Some(port) = args.port {
        cfg.port = port;
    }

    info!(
        "iradio-bridge v{} starting on port {}",
        env!("CARGO_PKG_VERSION"),
        cfg.port
    );

    // Auto-configure ~/.asoundrc with pcm.inferno_iradio_N blocks if requested
    if cfg.alsa.setup_alsa {
        if let Err(e) = alsa_setup::ensure_iradio_alsa(cfg.max_players, &cfg.alsa.asoundrc_path) {
            tracing::warn!("ALSA auto-setup failed (non-fatal): {}", e);
        }
    }

    let event_hub = crate::events::EventHub::new();
    let event_tx = event_hub.sender();
    let slot_senders = slot_keeper::start_slot_keepers(cfg.max_players, &cfg, event_tx);
    let app_state = Arc::new(state::AppState::new(cfg.clone(), slot_senders, event_hub));
    let router = api::build_router(app_state.clone(), cfg.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Listening on http://{}", addr);

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("received Ctrl+C, shutting down"); }
        _ = terminate => { tracing::info!("received SIGTERM, shutting down"); }
    }
}
