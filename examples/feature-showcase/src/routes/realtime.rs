//! Real-time routes - demonstrates WebSocket and SSE.
//!
//! ## Features Demonstrated
//! - WebSocket connections
//! - Server-Sent Events (SSE)
//! - Broadcast patterns
//! - Connection management

use archimedes_core::response::{json, Response};
use archimedes_router::Router;
use archimedes_sse::Sse;
use archimedes_ws::WebSocket;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

use super::AppState;

/// Event payload for SSE/WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: String,
}

/// WebSocket message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "ping")]
    Ping { timestamp: u64 },
    #[serde(rename = "pong")]
    Pong { timestamp: u64 },
    #[serde(rename = "subscribe")]
    Subscribe { channel: String },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { channel: String },
    #[serde(rename = "message")]
    Message { channel: String, data: serde_json::Value },
    #[serde(rename = "error")]
    Error { message: String },
}

/// Create the real-time router.
///
/// ## Endpoints
///
/// | Method | Path | Description |
/// |--------|------|-------------|
/// | GET | /events | SSE event stream |
/// | GET | /events/{channel} | SSE for specific channel |
/// | GET | /ws | WebSocket connection |
/// | GET | /ws/chat | WebSocket chat room |
pub fn routes(state: Arc<AppState>) -> Router {
    let mut router = Router::new();

    // -------------------------------------------------------------------------
    // SSE EVENTS - Demonstrates Server-Sent Events
    // -------------------------------------------------------------------------
    router.get("/events", || async {
        Sse::new()
            .event("connected", &serde_json::json!({
                "message": "SSE connection established"
            }))
            .interval(std::time::Duration::from_secs(1), |seq| {
                Event {
                    event_type: "heartbeat".to_string(),
                    data: serde_json::json!({ "seq": seq }),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }
            })
            .keep_alive(std::time::Duration::from_secs(30))
    });

    // -------------------------------------------------------------------------
    // SSE CHANNEL - Demonstrates channel-specific SSE
    // -------------------------------------------------------------------------
    router.get("/events/:channel", |archimedes_core::extract::Path(channel): archimedes_core::extract::Path<String>| async move {
        Sse::new()
            .event("subscribed", &serde_json::json!({
                "channel": channel,
                "message": format!("Subscribed to channel: {}", channel)
            }))
            .interval(std::time::Duration::from_secs(5), move |seq| {
                Event {
                    event_type: format!("{}_update", channel),
                    data: serde_json::json!({
                        "channel": channel.clone(),
                        "update_number": seq
                    }),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }
            })
    });

    // -------------------------------------------------------------------------
    // WEBSOCKET - Demonstrates basic WebSocket
    // -------------------------------------------------------------------------
    router.get("/ws", |ws: WebSocket| async {
        ws.on_upgrade(|mut socket| async move {
            // Send welcome message
            socket.send_json(&WsMessage::Message {
                channel: "system".to_string(),
                data: serde_json::json!({
                    "message": "WebSocket connection established"
                }),
            }).await.ok();

            // Echo loop
            while let Some(msg) = socket.recv().await {
                match msg {
                    Ok(text) => {
                        // Parse incoming message
                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            match ws_msg {
                                WsMessage::Ping { timestamp } => {
                                    socket.send_json(&WsMessage::Pong { timestamp }).await.ok();
                                }
                                WsMessage::Subscribe { channel } => {
                                    socket.send_json(&WsMessage::Message {
                                        channel: "system".to_string(),
                                        data: serde_json::json!({
                                            "action": "subscribed",
                                            "channel": channel
                                        }),
                                    }).await.ok();
                                }
                                _ => {
                                    // Echo other messages
                                    socket.send_json(&ws_msg).await.ok();
                                }
                            }
                        } else {
                            // Plain text echo
                            socket.send_text(&format!("Echo: {}", text)).await.ok();
                        }
                    }
                    Err(e) => {
                        socket.send_json(&WsMessage::Error {
                            message: format!("Error: {}", e),
                        }).await.ok();
                    }
                }
            }
        })
    });

    // -------------------------------------------------------------------------
    // WEBSOCKET CHAT - Demonstrates broadcast WebSocket
    // -------------------------------------------------------------------------
    router.get("/ws/chat", {
        // Create a broadcast channel for chat messages
        let (tx, _rx) = broadcast::channel::<String>(100);
        let tx = Arc::new(tx);
        
        move |ws: WebSocket| {
            let tx = tx.clone();
            async move {
                ws.on_upgrade(move |mut socket| async move {
                    let mut rx = tx.subscribe();
                    
                    // Welcome message
                    socket.send_json(&WsMessage::Message {
                        channel: "chat".to_string(),
                        data: serde_json::json!({
                            "message": "Welcome to the chat room!"
                        }),
                    }).await.ok();

                    loop {
                        tokio::select! {
                            // Receive from client
                            msg = socket.recv() => {
                                match msg {
                                    Some(Ok(text)) => {
                                        // Broadcast to all connected clients
                                        let _ = tx.send(text);
                                    }
                                    Some(Err(_)) | None => break,
                                }
                            }
                            // Receive from broadcast
                            msg = rx.recv() => {
                                if let Ok(text) = msg {
                                    if socket.send_text(&text).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                })
            }
        }
    });

    // -------------------------------------------------------------------------
    // NOTIFICATIONS - Demonstrates notification stream
    // -------------------------------------------------------------------------
    router.get("/notifications", |archimedes_core::extract::Query(params): archimedes_core::extract::Query<NotificationParams>| async move {
        let user_id = params.user_id.unwrap_or_else(|| "anonymous".to_string());
        
        Sse::new()
            .event("connected", &serde_json::json!({
                "user_id": user_id,
                "message": "Notification stream connected"
            }))
            .interval(std::time::Duration::from_secs(10), move |seq| {
                // Simulate notifications
                Event {
                    event_type: "notification".to_string(),
                    data: serde_json::json!({
                        "id": format!("notif_{}", seq),
                        "user_id": user_id.clone(),
                        "title": format!("Notification #{}", seq),
                        "body": "This is a sample notification",
                        "read": false
                    }),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }
            })
    });

    router.tag("realtime");
    router
}

/// Query parameters for notifications
#[derive(Debug, Deserialize)]
pub struct NotificationParams {
    pub user_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = Event {
            event_type: "test".to_string(),
            data: serde_json::json!({"key": "value"}),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&event).expect("Should serialize");
        assert!(json.contains("test"));
        assert!(json.contains("key"));
    }

    #[test]
    fn test_ws_message_ping() {
        let msg = WsMessage::Ping { timestamp: 12345 };
        let json = serde_json::to_string(&msg).expect("Should serialize");
        assert!(json.contains("ping"));
        assert!(json.contains("12345"));
    }

    #[test]
    fn test_ws_message_subscribe() {
        let json = r#"{"type":"subscribe","channel":"updates"}"#;
        let msg: WsMessage = serde_json::from_str(json).expect("Should deserialize");
        match msg {
            WsMessage::Subscribe { channel } => assert_eq!(channel, "updates"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_ws_message_deserialization() {
        let cases = vec![
            (r#"{"type":"ping","timestamp":123}"#, "ping"),
            (r#"{"type":"pong","timestamp":123}"#, "pong"),
            (r#"{"type":"subscribe","channel":"test"}"#, "subscribe"),
        ];

        for (json, expected_type) in cases {
            let msg: WsMessage = serde_json::from_str(json).expect("Should deserialize");
            let actual_type = match msg {
                WsMessage::Ping { .. } => "ping",
                WsMessage::Pong { .. } => "pong",
                WsMessage::Subscribe { .. } => "subscribe",
                WsMessage::Unsubscribe { .. } => "unsubscribe",
                WsMessage::Message { .. } => "message",
                WsMessage::Error { .. } => "error",
            };
            assert_eq!(actual_type, expected_type);
        }
    }
}
