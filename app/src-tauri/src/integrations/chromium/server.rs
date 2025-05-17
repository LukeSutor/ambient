use axum::{
    extract::{WebSocketUpgrade, ws::{WebSocket, Message}},
    response::IntoResponse,
    routing::get,
    Router,
    Server,
};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::net::SocketAddr;
use futures::StreamExt;
use futures::FutureExt;
use futures::SinkExt;

/// Try to start the server on a range of ports, returning the port used.
pub async fn start_server_on_available_port() -> Result<u16, String> {
    // Try ports 3010..=3020
    for port in 3010..=3020 {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                let (tx, _rx) = broadcast::channel::<String>(100);
                let state = Arc::new(Mutex::new(tx));
                let app = Router::new().route(
                    "/ws",
                    get({
                        let state = state.clone();
                        move |ws: WebSocketUpgrade| handle_websocket(ws, state.clone())
                    }),
                );
                // Convert tokio listener to std listener for axum::Server
                let std_listener = match listener.into_std() {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("[chromium/server] Failed to convert listener: {}", e);
                        continue;
                    }
                };
                tokio::spawn(async move {
                    if let Err(e) = Server::from_tcp(std_listener)
                        .unwrap()
                        .serve(app.into_make_service())
                        .await
                    {
                        eprintln!("[chromium/server] Server error: {}", e);
                    }
                });
                println!("[chromium/server] Started on port {}", port);
                return Ok(port);
            }
            Err(_) => continue,
        }
    }
    Err("No available port found in range 3010-3020".to_string())
}

async fn handle_websocket(
    ws: WebSocketUpgrade,
    state: Arc<Mutex<broadcast::Sender<String>>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(
    socket: WebSocket,
    state: Arc<Mutex<broadcast::Sender<String>>>,
) {
    let tx = state.lock().await.clone();
    let mut rx = tx.subscribe();

    let (mut send_socket, mut recv_socket) = socket.split();

    // Task for sending broadcasted messages to the client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if send_socket.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Task for receiving messages from the client and broadcasting them
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = recv_socket.next().await {
            if let Message::Text(text) = msg {
                println!("[chromium/server] Received message: {}", text);
                let _ = tx.send(text.to_string());
            }
        }
    });

    // Wait for either task to finish
    let _ = tokio::try_join!(send_task, recv_task);
}
