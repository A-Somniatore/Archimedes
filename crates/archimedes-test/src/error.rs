//! Test error types.

use std::fmt;

/// Errors that can occur during testing.
#[derive(Debug)]
pub enum TestError {
    /// Request building failed
    RequestBuild(String),
    /// Response body reading failed
    BodyRead(String),
    /// JSON serialization/deserialization failed
    Json(serde_json::Error),
    /// Request processing failed
    Processing(String),
    /// Header value is invalid
    InvalidHeader(String),
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequestBuild(msg) => write!(f, "Request build error: {msg}"),
            Self::BodyRead(msg) => write!(f, "Body read error: {msg}"),
            Self::Json(e) => write!(f, "JSON error: {e}"),
            Self::Processing(msg) => write!(f, "Processing error: {msg}"),
            Self::InvalidHeader(msg) => write!(f, "Invalid header: {msg}"),
        }
    }
}

impl std::error::Error for TestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for TestError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
