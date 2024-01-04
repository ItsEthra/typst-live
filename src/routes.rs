use crate::state::ServerState;
use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::{header::CONTENT_TYPE, Response},
    response::{Html, IntoResponse},
};
use std::sync::Arc;
use tokio::fs;

pub async fn root() -> Html<&'static str> {
    include_str!("index.html").into()
}

pub async fn target(State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    let filename = if state.args.no_recompile {
        &state.args.filename
    } else {
        "output.pdf"
    };

    let data = match fs::read(filename).await {
        Ok(data) => data,
        Err(err) => {
            println!("[ERR] Failed to read `{filename}` {err:?}");
            vec![]
        }
    };

    Response::builder()
        .header(CONTENT_TYPE, "application/pdf")
        .body(Body::from(data))
        .expect("Failed to build response")
}

pub async fn listen(
    State(state): State<Arc<ServerState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handler(socket, state))
}

async fn handler(mut socket: WebSocket, state: Arc<ServerState>) {
    loop {
        state.changed.notified().await;
        _ = socket.send(Message::Text("refresh".into())).await;
    }
}
