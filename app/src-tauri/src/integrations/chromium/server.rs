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
use futures::SinkExt;
use crate::integrations::chromium::workflow::{self, WorkflowStep};
use serde_json::Value;
use tauri::Manager;
use crate::db::DbState;
use crate::integrations::chromium::workflow::Workflow;
use once_cell::sync::OnceCell;

/// Global broadcast sender for Chromium websocket
pub static CHROMIUM_WS_BROADCAST: OnceCell<broadcast::Sender<String>> = OnceCell::new();

/// Try to start the server on a range of ports, returning the port used.
pub async fn start_server_on_available_port(app_handle: tauri::AppHandle) -> Result<u16, String> {
    // Try ports 3010..=3020
    for port in 3010..=3020 {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                let (tx, _rx) = broadcast::channel::<String>(100);
                // Set the global broadcast sender if not already set
                let _ = CHROMIUM_WS_BROADCAST.set(tx.clone());
                let state = Arc::new(Mutex::new(tx));
                let app = Router::new().route(
                    "/ws",
                    get({
                        let state = state.clone();
                        let app_handle = app_handle.clone(); // <-- clone here for each closure
                        move |ws: WebSocketUpgrade| handle_websocket(ws, state.clone(), app_handle.clone())
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
    app_handle: tauri::AppHandle
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, app_handle))
}

async fn handle_socket(
    socket: WebSocket,
    state: Arc<Mutex<broadcast::Sender<String>>>,
    app_handle: tauri::AppHandle
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
                // Parse event JSON
                if let Ok(event) = serde_json::from_str::<Value>(&text) {
                    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    let url = event.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    let timestamp_raw = event.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0);
                    // If timestamp is too large, assume it's in ms and convert to seconds
                    let timestamp = if timestamp_raw > 1_000_000_000_000 {
                        timestamp_raw / 1000
                    } else {
                        timestamp_raw
                    };
                    let step = WorkflowStep {
                        event_type: event_type.to_string(),
                        payload: event.clone(),
                        timestamp,
                    };
                    match event_type {
                        "page_open" => {
                            workflow::start_workflow(url, step);
                        },
                        "form_submitted" => {
                            workflow::append_step(url, step.clone());
                            // Get the tauri::State<DbState>
                            let db_state = app_handle.state::<crate::db::DbState>();
                            workflow::save_workflow(url, db_state);
                            workflow::remove_workflow(url);
                        },
                        "page_closed" => {
                            workflow::remove_workflow(url);
                        },
                        "click" | "input" | "change" | "scroll" | "keydown" => {
                            workflow::append_step(url, step);
                        },
                        _ => {}
                    }
                }
                let _ = tx.send(text.to_string());
            }
        }
    });

    // Wait for either task to finish
    let _ = tokio::try_join!(send_task, recv_task);
}

#[tauri::command]
pub async fn run_workflow_by_id(
    id: i64,
    state: tauri::State<'_, crate::db::DbState>,
) -> Result<(), String> {
    // Use the new DB helper
    let mut wf_json = crate::db::get_workflow_by_id(state, id)?;
    if let Some(steps_json_str) = wf_json.get("steps_json").and_then(|v| v.as_str()) {
        let steps_json_value: serde_json::Value = serde_json::from_str(steps_json_str)
            .map_err(|e| format!("Failed to parse steps_json: {}", e))?;
        if let Some(obj) = wf_json.as_object_mut() {
            obj.insert("steps_json".to_string(), steps_json_value);
        }
    }
    // Send to all websocket clients
    if let Some(sender) = CHROMIUM_WS_BROADCAST.get() {
        let msg = serde_json::json!({
            "event_type": "run_workflow",
            "payload": wf_json
        });
        let msg_str = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
        let _ = sender.send(msg_str);
        Ok(())
    } else {
        Err("WebSocket broadcast channel not initialized".to_string())
    }
}