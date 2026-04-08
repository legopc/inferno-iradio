use crate::api::ApiState;
use crate::events::WsEvent;
use crate::player::spawn_player;
use crate::state::PlayerInfo;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreatePlayerRequest {
    pub url: String,
    pub name: String,
    pub slot: Option<usize>,
    /// Initial volume 0.0–1.0. Defaults to 0.8 to protect ears.
    #[serde(default = "default_create_volume")]
    pub volume: f32,
    /// Per-slot gain in dB (-6.0 to 6.0). Defaults to stored slot gain.
    pub gain_db: Option<f32>,
}

fn default_create_volume() -> f32 { 0.8 }

#[derive(Deserialize)]
pub struct SetVolumeRequest {
    pub volume: f32,
}

#[derive(Deserialize)]
pub struct GainBody {
    pub gain_db: f32,
}

pub async fn list_players(State(ctx): State<ApiState>) -> Json<Vec<PlayerInfo>> {
    let players = ctx.state.players.read().await;
    let list: Vec<PlayerInfo> = players.values().map(|(info, _)| info.clone()).collect();
    Json(list)
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

    let slot = if let Some(s) = body.slot {
        if s == 0 || s > ctx.state.config.max_players {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": format!("slot must be 1–{}", ctx.state.config.max_players)
                })),
            )
                .into_response();
        }
        // Stop any existing player on this slot (overwrite)
        let occupied_id = {
            let players = ctx.state.players.read().await;
            players.values()
                .find(|(info, _)| info.slot == s)
                .map(|(info, _)| info.id)
        };
        if let Some(old_id) = occupied_id {
            let removed = ctx.state.players.write().await.remove(&old_id);
            if let Some((_, mut handle)) = removed {
                handle.stop().await;
            }
        }
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

    let slot_tx = match ctx.state.slot_senders.get(slot - 1) {
        Some(s) => s.clone(),
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "slot sender not configured" })),
            )
                .into_response();
        }
    };

    let initial_volume = if body.volume != default_create_volume() {
        body.volume.clamp(0.0, 1.0)
    } else {
        ctx.state.get_slot_volume(slot).await
    };

    let initial_gain = match body.gain_db {
        Some(g) => g.clamp(-6.0, 6.0),
        None => ctx.state.get_slot_gain(slot).await,
    };

    let (id, mut info, mut handle) = spawn_player(
        slot,
        body.name,
        body.url,
        ctx.state.clone(),
        &ctx.state.config,
        initial_volume,
        slot_tx,
    );

    // Apply initial gain
    info.gain_db = initial_gain;
    handle.set_gain(initial_gain);

    let player_json = serde_json::to_value(&info).unwrap_or_default();
    ctx.state.players.write().await.insert(id, (info.clone(), handle));

    // Broadcast new player to WebSocket subscribers
    ctx.state.events.send(WsEvent::PlayerUpdate { player: player_json });

    (StatusCode::CREATED, Json(info)).into_response()
}

pub async fn set_volume(
    State(ctx): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<SetVolumeRequest>,
) -> impl IntoResponse {
    let volume = body.volume.clamp(0.0, 1.0);
    let slot = {
        let mut players = ctx.state.players.write().await;
        match players.get_mut(&id) {
            Some((info, handle)) => {
                info.volume = volume;
                handle.set_volume(volume);
                info.slot
            }
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "player not found" })),
                )
                    .into_response();
            }
        }
    };
    // Persist the volume so next play on this slot starts at the same level
    ctx.state.set_slot_volume(slot, volume).await;
    Json(json!({ "id": id, "volume": volume })).into_response()
}

pub async fn patch_player_gain(
    State(ctx): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<GainBody>,
) -> impl IntoResponse {
    let gain_db = body.gain_db.clamp(-6.0, 6.0);
    let slot = {
        let mut players = ctx.state.players.write().await;
        if let Some((info, handle)) = players.get_mut(&id) {
            info.gain_db = gain_db;
            handle.set_gain(gain_db);
            info.slot
        } else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "not found" })),
            )
                .into_response();
        }
    };
    ctx.state.set_slot_gain(slot, gain_db).await;
    (StatusCode::OK, Json(json!({ "gain_db": gain_db }))).into_response()
}

pub async fn delete_player(
    State(ctx): State<ApiState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let removed = ctx.state.players.write().await.remove(&id);
    match removed {
        Some((_, mut handle)) => {
            handle.stop().await;
            ctx.state.events.send(WsEvent::PlayerStopped { id: id.to_string() });
            StatusCode::NO_CONTENT.into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "player not found" })),
        )
            .into_response(),
    }
}
