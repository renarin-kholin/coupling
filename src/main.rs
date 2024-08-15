use std::collections::HashMap;
use std::fmt::format;

use futures_util::{FutureExt, SinkExt, StreamExt, TryFutureExt};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::iter;
use tokio_stream::wrappers::UnboundedReceiverStream;
use utils::message::WSCommand;
use utils::user::{generate_userid, UserConnections, UserId, Users};
use warp::ws::{Message, WebSocket};
use warp::Filter;

pub mod utils;

#[tokio::main]
async fn main() {
    let users = Users::default();
    let user_connections = UserConnections::default();
    let users = warp::any().map(move || users.clone());
    let user_connections = warp::any().map(move || user_connections.clone());
    let routes = warp::path("couple")
        .and(warp::ws())
        .and(users)
        .and(user_connections)
        .map(|ws: warp::ws::Ws, users, user_connections| {
            ws.on_upgrade(move |websocket| user_connected(websocket, users, user_connections))
        });
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
async fn user_connected(ws: WebSocket, users: Users, user_connections: UserConnections) {
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
    users.write().await.insert(new_id.clone(), tx.clone());
    user_message(&new_id, WSCommand::SetClientId(new_id.clone()), &users).await;
    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error(uid={}): {}", new_id, e);
                break;
            }
        };
        let cc_message: WSCommand = serde_json::from_str(&msg.to_str().unwrap()).unwrap();
        match cc_message {
            WSCommand::CallRequest(user_id) => {
                println!("callrequest");
                let failed = call_request_send(&new_id, &user_id, &users, &user_connections).await;
                if failed {
                    println!("{}", new_id);
                    let users = users.read().await;
                    let tx = users.get(&new_id).unwrap();
                    let cc_message = WSCommand::CallRequestFailure;
                    let msg_str = serde_json::to_string(&cc_message).unwrap();
                    if let Err(_disconnected) = tx.send(Message::text(msg_str)) {}
                }
            }
            WSCommand::CallAnswer(agreed, sdp) => {
                if agreed && sdp.is_some() {
                    let user_connections = user_connections.read().await;
                    println!("{:?}", user_connections);
                    if let Some(user_connection) = user_connections.get(&new_id) {
                        let users = users.read().await;
                        let tx = users.get(user_connection).unwrap();
                        let cc_message = WSCommand::CallAnswer(true, Some(sdp.unwrap()));
                        let msg_str = serde_json::to_string(&cc_message).unwrap();
                        if let Err(_disconnected) = tx.send(Message::text(msg_str)) {}
                    } else {
                        println!("Connection not found");
                    };
                } else {
                }
            }
            WSCommand::CallReply(sdp) => {
                let user_connections = user_connections.read().await;
                for user_connection in user_connections.iter() {
                    if user_connection.1 == &new_id {
                        let user_id = user_connection.0;
                        let users = users.read().await;
                        let tx = users.get(user_id).unwrap();
                        let cc_message = WSCommand::CallReply(sdp.clone());
                        let msg_str = serde_json::to_string(&cc_message).unwrap();
                        if let Err(_disconnected) = tx.send(Message::text(msg_str)) {}
                    }
                }
            }
            _ => {
                println!("Unrecognized CCMessage");
            }
        }

        println!("msg: {:?}", msg);
    }
}
async fn user_message(user_id: &UserId, msg: WSCommand, users: &Users) {
    let users = users.read().await;
    let tx = users.get(user_id).unwrap();
    if let Err(_disconnected) = tx.send(Message::text(serde_json::to_string(&msg).unwrap())) {}
}
async fn call_request_send(
    requester_user_id: &UserId,
    user_id: &UserId,
    users: &Users,
    user_connections: &UserConnections,
) -> bool {
    let users = users.read().await;
    match users.get(user_id) {
        Some(tx) => {
            user_connections
                .write()
                .await
                .insert(user_id.clone(), requester_user_id.clone());
            let cc_message = WSCommand::CallRequest(requester_user_id.clone());
            let msg_str = serde_json::to_string(&cc_message).unwrap();
            if let Err(_disconnected) = tx.send(Message::text(msg_str)) {};
            return false;
        }
        None => return true,
    }
}
