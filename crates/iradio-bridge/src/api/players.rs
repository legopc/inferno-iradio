use crate::api::{ApiResult, ApiState};
use crate::player::spawn_player;
use crate::state::PlayerInfo;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreatePlayerRequest {
    pub url: String,
    pub name: String,
    /// Specific slot (1-based). If omitted, the next free slot is used.
    pub slot: Option<usize>,
}

pub async fn list_players(State(ctx): State<ApiState>) -> ApiResult<Vec<PlayerInfo>> {
    let players = ctx.state.players.read().await;
    let list: Vec<PlayerInfo> = players.values().map(|(info, _)| info.clone()).collect();
    Ok(Json(list))
}

pub async fn get_player(
    State(ctx): State<ApiState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let players = ctx.state.players.read().await;
    match players.get(&id) {
        Some((info, _)) => Json(info.clone()).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "player not found" })),
        )
            .into_response(),
    }
}

pub async fn create_player(
    State(ctx): State<ApiState>,
    Json(body): Json<CreatePlayerRequest>,
) -> impl IntoResponse {
    if body.url.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "url is required" })),
        )
            .into_response();
    }

    // Determine slot
    let slot = if let Some(s) = body.slot {
        // Validate requested slot
        if s == 0 || s > ctx.state.config.max_players {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": format!("slot must be 1–{}", ctx.state.config.max_players)
                })),
            )
                .into_response();
        }
        // Check if slot is already in use
        let players = ctx.state.players.read().await;
        if players.values().any(|(info, _)| info.slot == s) {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": format!("slot {} is already in use", s) })),
            )
                .into_response();
        }
        drop(players);
        s
    } else {
        match ctx.state.next_free_slot().await {
            Some(s) => s,
            None => {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": "all player slots are in use" })),
                )
                    .into_response();
            }
        }
    };

    let (id, info, handle) = spawn_player(
        slot,
        body.name,
        body.url,
        ctx.state.clone(),
        &ctx.state.config,
    );

    ctx.state.players.write().await.insert(id, (info.clone(), handle));

    (StatusCode::CREATED, Json(info)).into_response()
}

pub async fn delete_player(
    State(ctx): State<ApiState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let mut players = ctx.state.players.write().await;
    match players.remove(&id) {
        Some((_, mut handle)) => {
            // Stop the player task (async, brief)
            drop(players); // release lock before await
            handle.stop().await;
            StatusCode::NO_CONTENT.into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "player not found" })),
        )
            .into_response(),
    }
}
