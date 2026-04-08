use crate::api::{ApiResult, ApiState};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

pub async fn health(State(ctx): State<ApiState>) -> ApiResult<Value> {
    let players_active = ctx.state.players.read().await.len();
    Ok(axum::Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "players_active": players_active,
        "max_players": ctx.state.config.max_players,
    })))
}

pub async fn get_config(State(ctx): State<ApiState>) -> ApiResult<Value> {
    Ok(axum::Json(ctx.state.config.sanitised()))
}

/// GET /api/v1/volume — returns the stored default volume for each slot.
pub async fn get_volume(State(ctx): State<ApiState>) -> ApiResult<Value> {
    let vols = ctx.state.slot_volumes.read().await;
    // Return the first slot volume as the global default for the UI
    let default = vols.first().copied().unwrap_or(0.7);
    Ok(axum::Json(json!({ "volume": default, "slots": *vols })))
}

#[derive(Deserialize)]
pub struct SetVolumeBody {
    pub volume: f32,
}

/// PUT /api/v1/volume — set the default volume for all slots and persist.
pub async fn set_default_volume(
    State(ctx): State<ApiState>,
    Json(body): Json<SetVolumeBody>,
) -> impl IntoResponse {
    let volume = body.volume.clamp(0.0, 1.0);
    {
        let mut vols = ctx.state.slot_volumes.write().await;
        for v in vols.iter_mut() {
            *v = volume;
        }
    }
    ctx.state.save_volumes().await;
    Json(json!({ "volume": volume }))
}
