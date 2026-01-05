//! Archimedes extensions for caller identity.
//!
//! This module provides the [`CallerIdentityExt`] extension trait that adds
//! Archimedes-specific functionality to the shared [`CallerIdentity`] type
//! from `themis-platform-types`.
//!
//! The core [`CallerIdentity`] type is re-exported from `themis-platform-types`
//! at the crate root.

use themis_platform_types::CallerIdentity;

/// Extension trait for [`CallerIdentity`] providing Archimedes-specific functionality.
///
/// This trait adds methods useful for logging and middleware processing that
/// aren't part of the core shared type.
///
/// # Example
///
/// ```rust
/// use archimedes_core::{CallerIdentity, CallerIdentityExt};
///
/// let identity = CallerIdentity::user("user-123", "alice@example.com");
/// println!("Request from: {}", identity.log_id());
/// ```
pub trait CallerIdentityExt {
    /// Returns a string identifier suitable for logging.
    ///
    /// This never returns sensitive information like secrets or tokens.
    /// The format is designed to be human-readable and useful for debugging.
    ///
    /// # Returns
    ///
    /// - SPIFFE: Returns the SPIFFE ID (e.g., `spiffe://example.org/service`)
    /// - User: Returns `user:<user_id>` (e.g., `user:u123`)
    /// - `ApiKey`: Returns `apikey:<key_id>` (e.g., `apikey:k456`)
    /// - Anonymous: Returns `anonymous`
    fn log_id(&self) -> String;

    /// Returns roles extracted from the identity for authorization.
    ///
    /// This extracts role information from different identity types:
    /// - User: Returns the roles field
    /// - `ApiKey`: Returns scopes as pseudo-roles
    /// - SPIFFE: Returns the service name as a single role
    /// - Anonymous: Returns an empty list
    fn roles(&self) -> Vec<&str>;
}

impl CallerIdentityExt for CallerIdentity {
    fn log_id(&self) -> String {
        match self {
            Self::Spiffe(s) => s.spiffe_id.clone(),
            Self::User(u) => format!("user:{}", u.user_id),
            Self::ApiKey(k) => format!("apikey:{}", k.key_id),
            Self::Anonymous => "anonymous".to_string(),
            // CallerIdentity is #[non_exhaustive] for future extensibility
            _ => "unknown".to_string(),
        }
    }

    fn roles(&self) -> Vec<&str> {
        match self {
            Self::Spiffe(s) => {
                // Use service name as a pseudo-role if available
                s.service_name.as_deref().into_iter().collect()
            }
            Self::User(u) => u.roles.iter().map(String::as_str).collect(),
            Self::ApiKey(k) => k.scopes.iter().map(String::as_str).collect(),
            Self::Anonymous => Vec::new(),
            // CallerIdentity is #[non_exhaustive] for future extensibility
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use themis_platform_types::identity::{ApiKeyIdentity, SpiffeIdentity, UserIdentity};

    #[test]
    fn test_spiffe_log_id() {
        let identity = CallerIdentity::spiffe("spiffe://example.org/service/users");
        assert_eq!(identity.log_id(), "spiffe://example.org/service/users");
    }

    #[test]
    fn test_user_log_id() {
        let identity = CallerIdentity::user("user-123", "alice@example.com");
        assert_eq!(identity.log_id(), "user:user-123");
    }

    #[test]
    fn test_api_key_log_id() {
        let identity = CallerIdentity::api_key("key-abc123", "Production Key");
        assert_eq!(identity.log_id(), "apikey:key-abc123");
    }

    #[test]
    fn test_anonymous_log_id() {
        let identity = CallerIdentity::anonymous();
        assert_eq!(identity.log_id(), "anonymous");
    }

    #[test]
    fn test_user_roles() {
        let identity = CallerIdentity::User(UserIdentity {
            user_id: "u123".to_string(),
            email: None,
            name: None,
            roles: vec!["admin".to_string(), "user".to_string()],
            groups: vec![],
            tenant_id: None,
        });
        assert_eq!(identity.roles(), vec!["admin", "user"]);
    }

    #[test]
    fn test_api_key_roles_from_scopes() {
        let identity = CallerIdentity::ApiKey(ApiKeyIdentity {
            key_id: "k123".to_string(),
            name: "Test Key".to_string(),
            scopes: vec!["read".to_string(), "write".to_string()],
            owner_id: None,
        });
        assert_eq!(identity.roles(), vec!["read", "write"]);
    }

    #[test]
    fn test_spiffe_roles_from_service_name() {
        let identity = CallerIdentity::Spiffe(SpiffeIdentity {
            spiffe_id: "spiffe://example.org/orders".to_string(),
            trust_domain: Some("example.org".to_string()),
            service_name: Some("orders".to_string()),
        });
        assert_eq!(identity.roles(), vec!["orders"]);
    }

    #[test]
    fn test_anonymous_roles_empty() {
        let identity = CallerIdentity::anonymous();
        assert!(identity.roles().is_empty());
    }

    #[test]
    fn test_display() {
        // The Display impl is from themis-platform-types, but let's verify it works
        let identity = CallerIdentity::user("u123", "test@example.com");
        let display = format!("{:?}", identity);
        assert!(display.contains("User"));
        assert!(display.contains("u123"));
    }

    #[test]
    fn test_serialization() {
        let identity = CallerIdentity::user("u123", "test@example.com");
        let json = serde_json::to_string(&identity).expect("serialization should work");
        assert!(json.contains("\"type\":\"user\""));
        assert!(json.contains("\"user_id\":\"u123\""));

        let parsed: CallerIdentity =
            serde_json::from_str(&json).expect("deserialization should work");
        assert_eq!(identity, parsed);
    }
}
