use crate::api::{ApiResult, ApiState};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

pub async fn health(State(ctx): State<ApiState>) -> ApiResult<Value> {
    let active_players = ctx.state.players.read().await.len();
    let max_players = ctx.state.config.max_players;

    let mut slot_health = Vec::with_capacity(ctx.state.slot_health.len());
    for lock in &ctx.state.slot_health {
        let h = lock.read().await;
        slot_health.push(json!({
            "slot": h.slot,
            "connects": h.connects,
            "errors": h.errors,
            "last_error": h.last_error,
        }));
    }

    Ok(axum::Json(json!({
        "version": "2.0.0",
        "active_players": active_players,
        "max_players": max_players,
        "slot_health": slot_health,
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
