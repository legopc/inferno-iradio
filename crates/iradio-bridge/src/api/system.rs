use crate::api::{ApiResult, ApiState};
use axum::extract::State;
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
