use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::routes::health::AppState;
use tools_common::types::{JobProgress, JobStatus};

/// Handle WebSocket upgrade at /api/job/{id}/ws.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state, job_id))
}

async fn handle_ws(ws: WebSocket, state: Arc<AppState>, job_id: Uuid) {
    let (mut sender, mut receiver) = ws.split();

    // Subscribe to NATS progress updates
    let nats = state.nats.clone();
    let subject = format!("tools.*.progress.{}", job_id);
    let mut subscriber = match nats.subscribe(subject).await {
        Ok(sub) => sub,
        Err(e) => {
            tracing::error!("Failed to subscribe to NATS: {}", e);
            let _ = sender
                .send(Message::Text(
                    serde_json::json!({
                        "type": "error",
                        "job_id": job_id,
                        "status": "failed",
                        "error": format!("Connection error: {}", e)
                    })
                    .to_string()
                    .into(),
                ))
                .await;
            return;
        }
    };

    // Send initial status from Redis
    if let Ok(mut conn) = state.redis.get_multiplexed_async_connection().await {
        if let Ok(job) = crate::redis::job::JobRepository::get(&mut conn, job_id).await {
            let init_msg = serde_json::json!({
                "type": "status",
                "job_id": job_id,
                "status": match &job.status {
                    JobStatus::Queued => "queued",
                    JobStatus::Processing { .. } => "processing",
                    JobStatus::Completed => "completed",
                    JobStatus::NeedsManualCrop => "needs_manual_crop",
                    JobStatus::Failed(_) => "failed",
                },
                "progress": match &job.status {
                    JobStatus::Processing { progress, .. } => *progress,
                    JobStatus::Completed => 100,
                    _ => 0,
                },
            });
            let _ = sender
                .send(Message::Text(init_msg.to_string().into()))
                .await;
        }
    }

    // Channel for NATS messages
    let (tx, mut rx) = mpsc::channel::<String>(32);

    // Spawn NATS listener
    let tx_clone = tx.clone();
    let nats_listener = tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = subscriber.next() => {
                    match msg {
                        Some(nats_msg) => {
                            if let Ok(progress) = serde_json::from_slice::<JobProgress>(&nats_msg.payload) {
                                let json = serde_json::json!({
                                    "type": "progress",
                                    "job_id": progress.job_id,
                                    "status": match &progress.status {
                                        JobStatus::Queued => "queued",
                                        JobStatus::Processing { .. } => "processing",
                                        JobStatus::Completed => "completed",
                                        JobStatus::NeedsManualCrop => "needs_manual_crop",
                                        JobStatus::Failed(_) => "failed",
                                    },
                                    "progress": progress.progress,
                                    "stage": progress.stage,
                                    "message": progress.message,
                                });
                                let _ = tx_clone.send(json.to_string()).await;
                            }
                        }
                        None => break,
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                    // Keepalive ping
                    let _ = tx_clone.send(serde_json::json!({"type": "ping"}).to_string()).await;
                }
            }
        }
    });

    // Forward messages from channel to WebSocket
    let ws_sender = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Listen for client close
    let ws_receiver = tokio::spawn(async move {
        while let Some(Ok(_)) = receiver.next().await {
            // Client messages ignored (we only forward server→client)
        }
    });

    // Wait for either task to complete (connection closed)
    tokio::select! {
        _ = ws_sender => {},
        _ = ws_receiver => {},
    }

    // Cancel NATS listener
    nats_listener.abort();
}