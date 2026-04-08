use crate::api::{ApiResult, ApiState};
use crate::state::Station;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde_json::json;

pub async fn list_favourites(State(ctx): State<ApiState>) -> ApiResult<Vec<Station>> {
    let favs = ctx.state.favourites.read().await;
    Ok(Json(favs.clone()))
}

pub async fn add_favourite(
    State(ctx): State<ApiState>,
    Json(station): Json<Station>,
) -> impl IntoResponse {
    let added = ctx.state.add_favourite(station).await;
    if !added {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "already in favourites" })),
        )
            .into_response();
    }
    (StatusCode::CREATED, Json(json!({ "ok": true }))).into_response()
}

pub async fn remove_favourite(
    State(ctx): State<ApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    {
        let mut favs = ctx.state.favourites.write().await;
        let before = favs.len();
        favs.retain(|s| s.id != id);
        if favs.len() == before {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "favourite not found" })),
            )
                .into_response();
        }
    }
    if let Err(e) = ctx.state.save_favourites().await {
        tracing::warn!("failed to persist favourites: {e}");
    }
    StatusCode::NO_CONTENT.into_response()
}
