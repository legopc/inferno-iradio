pub mod favourites;
pub mod players;
pub mod stations;
pub mod system;

use crate::auth::basic_auth_middleware;
use crate::config::Config;
use crate::radiobrowser::RadioBrowserClient;
use crate::state::SharedState;
use axum::{
    http::StatusCode,
    middleware,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, patch, post},
    Router,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

/// Shared API context passed as axum State
#[derive(Clone)]
pub struct ApiContext {
    pub state: SharedState,
    /// Lazily-initialized RadioBrowser client (None until first use)
    pub rb_client: Arc<RwLock<Option<RadioBrowserClient>>>,
    pub config: Config,
}

pub type ApiState = Arc<ApiContext>;

/// Standard JSON error wrapper
pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": self.0.to_string() })),
        )
            .into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(e: E) -> Self {
        AppError(e.into())
    }
}

pub type ApiResult<T> = Result<Json<T>, AppError>;

/// Embed web-ui/ assets at compile time
#[derive(rust_embed::RustEmbed)]
#[folder = "../../web-ui/"]
struct WebAssets;

pub fn build_router(state: SharedState, config: Config) -> Router {
    let rb_holder: Arc<RwLock<Option<RadioBrowserClient>>> = Arc::new(RwLock::new(None));
    let rb_init = rb_holder.clone();
    let cfg_init = config.clone();
    tokio::spawn(async move {
        let client = RadioBrowserClient::new(
            &cfg_init.radiobrowser.api_url,
            cfg_init.radiobrowser.request_timeout_secs,
        )
        .await;
        *rb_init.write().await = Some(client);
    });

    let ctx: ApiState = Arc::new(ApiContext {
        state,
        rb_client: rb_holder,
        config: config.clone(),
    });

    let api = Router::new()
        .route("/health", get(system::health))
        .route("/config", get(system::get_config))
        .route("/players", get(players::list_players))
        .route("/players", post(players::create_player))
        .route("/players/:id", get(players::get_player))
        .route("/players/:id", delete(players::delete_player))
        .route("/players/:id/volume", patch(players::set_volume))
        .route("/stations/search", get(stations::search))
        .route("/stations/top", get(stations::top))
        .route("/stations/tags", get(stations::tags))
        .route("/stations/countries", get(stations::countries))
        .route("/stations/by-tag", get(stations::by_tag))
        .route("/stations/by-country", get(stations::by_country))
        .route("/favourites", get(favourites::list_favourites))
        .route("/favourites", post(favourites::add_favourite))
        .route("/favourites/:id", delete(favourites::remove_favourite))
        .with_state(ctx);

    let auth_enabled = config.auth.enabled;
    let auth_user = config.auth.username.clone();
    let auth_pass = config.auth.password.clone();

    let api_authed = api.layer(middleware::from_fn(move |req, next| {
        basic_auth_middleware(req, next, auth_user.clone(), auth_pass.clone(), auth_enabled)
    }));

    Router::new()
        .nest("/api/v1", api_authed)
        .route("/", get(serve_index))
        .route("/*path", get(serve_static))
        .layer(CorsLayer::permissive())
}

async fn serve_index() -> impl IntoResponse {
    serve_asset("index.html")
}

async fn serve_static(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> impl IntoResponse {
    serve_asset(&path)
}

fn serve_asset(path: &str) -> Response {
    match WebAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                StatusCode::OK,
                [(
                    axum::http::header::CONTENT_TYPE,
                    mime.as_ref().to_string(),
                )],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}
