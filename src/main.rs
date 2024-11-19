use warp::Filter;
use futures::{FutureExt, StreamExt, SinkExt}; // Add `SinkExt` here
use warp::ws::{Message, WebSocket};
use tokio::sync::broadcast;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Edit {
    content: String,   // The current text content
    cursor_position: usize, // Cursor position of the user
}
async fn user_connected(ws: WebSocket, tx: broadcast::Sender<Edit>) {
    // Split WebSocket into sender and receiver
    let (mut user_ws_tx, mut user_ws_rx) = ws.split();

    // Subscribe to document changes
    let mut rx = tx.subscribe();
    // Task to send document updates to the user
    let send_task = tokio::spawn(async move {
        while let Ok(edit) = rx.recv().await {
            if let Ok(msg) = serde_json::to_string(&edit) {
                // Send the serialized JSON message to the WebSocket
                if user_ws_tx.send(Message::text(msg)).await.is_err() {
                    break;
                }
            }
        }
    });
    // Task to receive user's edits
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = user_ws_rx.next().await {
            if let Ok(text) = msg.to_str() {
                if let Ok(edit) = serde_json::from_str::<Edit>(text) {
                    // Broadcast the received edit to other clients
                    tx.send(edit).unwrap();
                }
            }
        }
    });

    // Wait for both tasks to complete
    tokio::select! {
        _ = send_task => (),
        _ = recv_task => (),
    }

}


#[tokio::main]
async fn main() {
    //Create a broadcast channel
    let (tx, _rx) = broadcast::channel(100);
    //Create a ws route
    let ws_route = warp::path("ws")
    .and(warp::ws())
    .and(warp::any().map(move || tx.clone()))
    .map(|ws: warp::ws::Ws, tx| {
        ws.on_upgrade(move |socket| user_connected(socket, tx))
    });
        //Create a html route
        let html_route = warp::path::end()
        .and(warp::fs::file("./static/index.html"));
        let routes = ws_route.or(html_route);
        warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;


}