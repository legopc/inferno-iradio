use crate::api::ApiState;
use crate::events::WsEvent;
use crate::state::SharedState;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::{interval, Duration};

pub async fn ws_handler(ws: WebSocketUpgrade, State(ctx): State<ApiState>) -> impl IntoResponse {
    let state = ctx.state.clone();
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: SharedState) {
    let mut event_rx = state.events.subscribe();

    // Send initial snapshot of all current players
    let players: Vec<_> = {
        let players = state.players.read().await;
        players.values().map(|(info, _)| info.clone()).collect()
    };
    let snapshot = json!({ "type": "snapshot", "players": players });
    if socket
        .send(Message::Text(snapshot.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    let vu_fps = state.config.websocket.vu_fps.max(1);
    let mut vu_interval = interval(Duration::from_millis(1000 / vu_fps as u64));
    // consume the immediate first tick so we don't flush an empty batch right away
    vu_interval.tick().await;

    let mut vu_batch: HashMap<usize, (f32, f32)> = HashMap::new();

    loop {
        tokio::select! {
            _ = vu_interval.tick() => {
                if !vu_batch.is_empty() {
                    let levels: serde_json::Map<String, serde_json::Value> = vu_batch
                        .drain()
                        .map(|(slot, (l, r))| (slot.to_string(), json!({"l": l, "r": r})))
                        .collect();
                    let msg = json!({ "type": "vu_batch", "levels": levels });
                    if socket.send(Message::Text(msg.to_string().into())).await.is_err() {
                        break;
                    }
                }
            }

            result = event_rx.recv() => {
                match result {
                    Ok(WsEvent::Vu { slot, l, r }) => {
                        vu_batch.insert(slot, (l, r));
                    }
                    Ok(event) => {
                        match serde_json::to_string(&event) {
                            Ok(json) => {
                                if socket.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => tracing::warn!("WS serialize error: {}", e),
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        tracing::warn!("WS lagged {n} events");
                    }
                    Err(RecvError::Closed) => {
                        break;
                    }
                }
            }

            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }
}
