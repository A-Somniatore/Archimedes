//! WebSocket HTTP upgrade handling.
//!
//! This module provides functionality for upgrading HTTP connections
//! to WebSocket connections according to RFC 6455.

use std::future::Future;

use base64::Engine;
use http::{header, Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::Bytes;
use sha1::{Digest, Sha1};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::WebSocketStream;
use tracing::{debug, instrument};

use crate::config::WebSocketConfig;
use crate::connection::{ConnectionId, WebSocket};
use crate::error::{WsError, WsResult};

/// The WebSocket magic GUID used in the handshake.
const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Check if a request is a WebSocket upgrade request.
///
/// A valid WebSocket upgrade request must have:
/// - `Connection: Upgrade` header
/// - `Upgrade: websocket` header
/// - `Sec-WebSocket-Key` header
/// - `Sec-WebSocket-Version: 13` header
pub fn is_websocket_request<B>(request: &Request<B>) -> bool {
    has_upgrade_header(request)
        && has_websocket_upgrade(request)
        && has_websocket_key(request)
        && has_websocket_version(request)
}

/// Check if the request has a Connection: Upgrade header.
fn has_upgrade_header<B>(request: &Request<B>) -> bool {
    request
        .headers()
        .get(header::CONNECTION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_lowercase().contains("upgrade"))
        .unwrap_or(false)
}

/// Check if the request has an Upgrade: websocket header.
fn has_websocket_upgrade<B>(request: &Request<B>) -> bool {
    request
        .headers()
        .get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false)
}

/// Check if the request has a Sec-WebSocket-Key header.
fn has_websocket_key<B>(request: &Request<B>) -> bool {
    request
        .headers()
        .get("sec-websocket-key")
        .and_then(|v| v.to_str().ok())
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

/// Check if the request has Sec-WebSocket-Version: 13.
fn has_websocket_version<B>(request: &Request<B>) -> bool {
    request
        .headers()
        .get("sec-websocket-version")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "13")
        .unwrap_or(false)
}

/// Get the Sec-WebSocket-Key from the request.
fn get_websocket_key<B>(request: &Request<B>) -> Option<&str> {
    request
        .headers()
        .get("sec-websocket-key")
        .and_then(|v| v.to_str().ok())
}

/// Get the requested subprotocols from the request.
pub fn get_websocket_protocols<B>(request: &Request<B>) -> Vec<String> {
    request
        .headers()
        .get_all("sec-websocket-protocol")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(',').map(str::trim))
        .map(String::from)
        .collect()
}

/// Compute the Sec-WebSocket-Accept value from the key.
fn compute_accept_key(key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WEBSOCKET_GUID.as_bytes());
    let result = hasher.finalize();
    base64::engine::general_purpose::STANDARD.encode(result)
}

/// Create a WebSocket upgrade response.
fn create_upgrade_response(accept_key: &str, protocol: Option<&str>) -> Response<Full<Bytes>> {
    let mut builder = Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(header::CONNECTION, "Upgrade")
        .header(header::UPGRADE, "websocket")
        .header("Sec-WebSocket-Accept", accept_key);

    if let Some(protocol) = protocol {
        builder = builder.header("Sec-WebSocket-Protocol", protocol);
    }

    builder.body(Full::new(Bytes::new())).unwrap()
}

/// Create a bad request response.
fn create_bad_request_response(reason: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Full::new(Bytes::from(reason.to_string())))
        .unwrap()
}

/// A WebSocket upgrade result.
///
/// This is returned from the upgrade process and contains either
/// a successful upgrade response or an error response.
pub struct WebSocketUpgrade {
    /// The response to send to the client.
    pub response: Response<Full<Bytes>>,
    /// The selected subprotocol, if any.
    pub protocol: Option<String>,
    /// Whether the upgrade was successful.
    pub success: bool,
}

impl WebSocketUpgrade {
    /// Create a successful upgrade.
    fn success(response: Response<Full<Bytes>>, protocol: Option<String>) -> Self {
        Self {
            response,
            protocol,
            success: true,
        }
    }

    /// Create a failed upgrade.
    fn failure(response: Response<Full<Bytes>>) -> Self {
        Self {
            response,
            protocol: None,
            success: false,
        }
    }
}

/// Validate a WebSocket upgrade request.
///
/// Returns the accept key if valid, or an error describing why it's invalid.
#[instrument(skip(request))]
pub fn validate_upgrade_request<B>(request: &Request<B>) -> WsResult<String> {
    if !has_upgrade_header(request) {
        return Err(WsError::not_websocket("missing Connection: Upgrade header"));
    }

    if !has_websocket_upgrade(request) {
        return Err(WsError::not_websocket("missing Upgrade: websocket header"));
    }

    let key = get_websocket_key(request).ok_or_else(|| {
        WsError::not_websocket("missing Sec-WebSocket-Key header")
    })?;

    if !has_websocket_version(request) {
        return Err(WsError::not_websocket(
            "missing or invalid Sec-WebSocket-Version header (must be 13)",
        ));
    }

    Ok(compute_accept_key(key))
}

/// Prepare a WebSocket upgrade.
///
/// This validates the request and prepares the upgrade response.
/// If the upgrade is successful, you must complete the upgrade by
/// calling [`complete_upgrade`] with the underlying IO stream.
///
/// # Arguments
///
/// * `request` - The HTTP request to upgrade
/// * `allowed_protocols` - Optional list of allowed subprotocols. If provided,
///   the selected protocol will be the first one that matches a requested protocol.
///
/// # Returns
///
/// A [`WebSocketUpgrade`] containing the response to send and upgrade status.
#[instrument(skip(request, allowed_protocols))]
pub fn prepare_upgrade<B>(
    request: &Request<B>,
    allowed_protocols: Option<&[&str]>,
) -> WebSocketUpgrade {
    let accept_key = match validate_upgrade_request(request) {
        Ok(key) => key,
        Err(e) => {
            debug!("WebSocket upgrade validation failed: {}", e);
            return WebSocketUpgrade::failure(create_bad_request_response(&e.to_string()));
        }
    };

    // Select subprotocol if requested
    let selected_protocol = if let Some(allowed) = allowed_protocols {
        let requested = get_websocket_protocols(request);
        requested
            .iter()
            .find(|p| allowed.iter().any(|a| a.eq_ignore_ascii_case(p)))
            .cloned()
    } else {
        None
    };

    let response = create_upgrade_response(&accept_key, selected_protocol.as_deref());
    WebSocketUpgrade::success(response, selected_protocol)
}

/// Complete a WebSocket upgrade.
///
/// This should be called after sending the upgrade response to the client.
/// It converts the underlying IO stream to a WebSocket connection.
///
/// # Arguments
///
/// * `stream` - The underlying IO stream (e.g., TcpStream)
/// * `config` - WebSocket configuration
///
/// # Returns
///
/// A [`WebSocket`] connection ready for use.
pub async fn complete_upgrade<S>(stream: S, config: WebSocketConfig) -> WebSocket<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let ws_stream = WebSocketStream::from_raw_socket(
        stream,
        tungstenite::protocol::Role::Server,
        None,
    )
    .await;

    WebSocket::new(ws_stream, config)
}

/// Complete a WebSocket upgrade with a specific connection ID.
pub async fn complete_upgrade_with_id<S>(
    stream: S,
    config: WebSocketConfig,
    connection_id: ConnectionId,
) -> WebSocket<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let ws_stream = WebSocketStream::from_raw_socket(
        stream,
        tungstenite::protocol::Role::Server,
        None,
    )
    .await;

    WebSocket::with_id(ws_stream, config, connection_id)
}

/// Handler type for WebSocket connections.
///
/// This is the signature of a function that handles a WebSocket connection.
pub trait WebSocketHandler<S>: Clone + Send + Sync + 'static
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    /// The future returned by the handler.
    type Future: Future<Output = ()> + Send;

    /// Handle a WebSocket connection.
    fn handle(&self, ws: WebSocket<S>) -> Self::Future;
}

impl<S, F, Fut> WebSocketHandler<S> for F
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    F: Fn(WebSocket<S>) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send,
{
    type Future = Fut;

    fn handle(&self, ws: WebSocket<S>) -> Self::Future {
        self(ws)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ws_request() -> Request<()> {
        Request::builder()
            .header(header::CONNECTION, "Upgrade")
            .header(header::UPGRADE, "websocket")
            .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
            .header("Sec-WebSocket-Version", "13")
            .body(())
            .unwrap()
    }

    #[test]
    fn test_is_websocket_request_valid() {
        let request = make_ws_request();
        assert!(is_websocket_request(&request));
    }

    #[test]
    fn test_is_websocket_request_missing_connection() {
        let request = Request::builder()
            .header(header::UPGRADE, "websocket")
            .header("Sec-WebSocket-Key", "key")
            .header("Sec-WebSocket-Version", "13")
            .body(())
            .unwrap();
        assert!(!is_websocket_request(&request));
    }

    #[test]
    fn test_is_websocket_request_missing_upgrade() {
        let request = Request::builder()
            .header(header::CONNECTION, "Upgrade")
            .header("Sec-WebSocket-Key", "key")
            .header("Sec-WebSocket-Version", "13")
            .body(())
            .unwrap();
        assert!(!is_websocket_request(&request));
    }

    #[test]
    fn test_is_websocket_request_missing_key() {
        let request = Request::builder()
            .header(header::CONNECTION, "Upgrade")
            .header(header::UPGRADE, "websocket")
            .header("Sec-WebSocket-Version", "13")
            .body(())
            .unwrap();
        assert!(!is_websocket_request(&request));
    }

    #[test]
    fn test_is_websocket_request_wrong_version() {
        let request = Request::builder()
            .header(header::CONNECTION, "Upgrade")
            .header(header::UPGRADE, "websocket")
            .header("Sec-WebSocket-Key", "key")
            .header("Sec-WebSocket-Version", "12")
            .body(())
            .unwrap();
        assert!(!is_websocket_request(&request));
    }

    #[test]
    fn test_compute_accept_key() {
        // RFC 6455 example
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let accept = compute_accept_key(key);
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn test_validate_upgrade_request_valid() {
        let request = make_ws_request();
        let result = validate_upgrade_request(&request);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn test_validate_upgrade_request_missing_connection() {
        let request = Request::builder()
            .header(header::UPGRADE, "websocket")
            .header("Sec-WebSocket-Key", "key")
            .header("Sec-WebSocket-Version", "13")
            .body(())
            .unwrap();
        let result = validate_upgrade_request(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Connection"));
    }

    #[test]
    fn test_prepare_upgrade_success() {
        let request = make_ws_request();
        let upgrade = prepare_upgrade(&request, None);
        assert!(upgrade.success);
        assert_eq!(upgrade.response.status(), StatusCode::SWITCHING_PROTOCOLS);
        assert_eq!(
            upgrade.response.headers().get(header::UPGRADE).unwrap(),
            "websocket"
        );
    }

    #[test]
    fn test_prepare_upgrade_with_protocol() {
        let request = Request::builder()
            .header(header::CONNECTION, "Upgrade")
            .header(header::UPGRADE, "websocket")
            .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Protocol", "chat, json")
            .body(())
            .unwrap();

        let upgrade = prepare_upgrade(&request, Some(&["json", "xml"]));
        assert!(upgrade.success);
        assert_eq!(upgrade.protocol, Some("json".to_string()));
    }

    #[test]
    fn test_prepare_upgrade_no_matching_protocol() {
        let request = Request::builder()
            .header(header::CONNECTION, "Upgrade")
            .header(header::UPGRADE, "websocket")
            .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Protocol", "chat")
            .body(())
            .unwrap();

        let upgrade = prepare_upgrade(&request, Some(&["json", "xml"]));
        assert!(upgrade.success);
        assert_eq!(upgrade.protocol, None);
    }

    #[test]
    fn test_prepare_upgrade_invalid_request() {
        let request = Request::builder().body(()).unwrap();
        let upgrade = prepare_upgrade(&request, None);
        assert!(!upgrade.success);
        assert_eq!(upgrade.response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_get_websocket_protocols() {
        let request = Request::builder()
            .header("Sec-WebSocket-Protocol", "chat, json")
            .body(())
            .unwrap();

        let protocols = get_websocket_protocols(&request);
        assert_eq!(protocols, vec!["chat", "json"]);
    }

    #[test]
    fn test_get_websocket_protocols_multiple_headers() {
        let request = Request::builder()
            .header("Sec-WebSocket-Protocol", "chat")
            .header("Sec-WebSocket-Protocol", "json")
            .body(())
            .unwrap();

        let protocols = get_websocket_protocols(&request);
        assert_eq!(protocols, vec!["chat", "json"]);
    }
}
