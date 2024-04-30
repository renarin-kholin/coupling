use std::collections::HashMap;
use std::fmt::format;

use futures_util::{FutureExt, SinkExt, StreamExt, TryFutureExt};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use utils::message::CCMessage;
use utils::user::{generate_userid, UserId, Users};
use warp::ws::{Message, WebSocket};
use warp::Filter;

pub mod utils;

#[tokio::main]
async fn main() {
    let users = Users::default();
    let users = warp::any().map(move || users.clone());
    let routes = warp::path("couple")
        .and(warp::ws())
        .and(users)
        .map(|ws: warp::ws::Ws, users| {
            ws.on_upgrade(move |websocket| user_connected(websocket, users))
        });
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
async fn user_connected(ws: WebSocket, users: Users) {
    let new_id = generate_userid();
    eprintln!("New user connected: {}", new_id);
    let (mut user_ws_tx, mut user_ws_rx) = ws.split();

    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);

    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            user_ws_tx
                .send(message)
                .unwrap_or_else(|e| {
                    eprintln!("Websocket send error: {}", e);
                })
                .await;
        }
    });
    users.write().await.insert(new_id.clone(), tx);
    user_message(&new_id, CCMessage::SetClientId(new_id.clone()), &users).await;
    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error(uid={}): {}", new_id, e);
                break;
            }
        };
        println!("msg: {:?}", msg);
    }
}
async fn user_message(user_id: &UserId, msg: CCMessage, users: &Users) {
    let users = users.read().await;
    let tx = users.get(user_id).unwrap();
    if let Err(_disconnected) = tx.send(Message::text(serde_json::to_string(&msg).unwrap())) {}
}
