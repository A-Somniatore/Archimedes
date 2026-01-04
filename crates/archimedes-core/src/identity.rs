//! Caller identity types.
//!
//! The [`CallerIdentity`] enum represents the authenticated identity of a request caller.

use serde::{Deserialize, Serialize};

/// The authenticated identity of a request caller.
///
/// Archimedes supports multiple identity types:
/// - **SPIFFE**: Service identity from mTLS (for service-to-service calls)
/// - **User**: Human user identity (from JWT or session)
/// - **ApiKey**: API key-based identity
/// - **Anonymous**: No identity established
///
/// # Example
///
/// ```
/// use archimedes_core::CallerIdentity;
///
/// // Service identity from mTLS
/// let service = CallerIdentity::spiffe("spiffe://example.org/service/users");
///
/// // User identity from JWT
/// let user = CallerIdentity::user("user-123", Some("alice@example.com"));
///
/// // API key identity
/// let api_key = CallerIdentity::api_key("key-abc123", Some("Production Key"));
///
/// // Anonymous caller
/// let anon = CallerIdentity::Anonymous;
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CallerIdentity {
    /// SPIFFE identity from mTLS certificate.
    ///
    /// Used for service-to-service authentication within the platform.
    /// The SPIFFE ID follows the format: `spiffe://<trust-domain>/<path>`
    Spiffe {
        /// The SPIFFE ID (e.g., "spiffe://example.org/service/users")
        spiffe_id: String,
    },

    /// Human user identity.
    ///
    /// Typically extracted from a JWT token or session.
    User {
        /// Unique user identifier
        user_id: String,
        /// Optional email address
        email: Option<String>,
        /// Optional display name
        name: Option<String>,
        /// Roles or groups the user belongs to
        #[serde(default)]
        roles: Vec<String>,
    },

    /// API key-based identity.
    ///
    /// Used for external API access with long-lived credentials.
    ApiKey {
        /// The API key identifier (not the secret)
        key_id: String,
        /// Optional human-readable name for the key
        name: Option<String>,
        /// Scopes granted to this API key
        #[serde(default)]
        scopes: Vec<String>,
    },

    /// Anonymous caller with no established identity.
    ///
    /// Used when no authentication credentials are provided.
    Anonymous,
}

impl CallerIdentity {
    /// Creates a SPIFFE identity.
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_core::CallerIdentity;
    ///
    /// let identity = CallerIdentity::spiffe("spiffe://example.org/service/users");
    /// ```
    #[must_use]
    pub fn spiffe(spiffe_id: impl Into<String>) -> Self {
        Self::Spiffe {
            spiffe_id: spiffe_id.into(),
        }
    }

    /// Creates a user identity.
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_core::CallerIdentity;
    ///
    /// let identity = CallerIdentity::user("user-123", Some("alice@example.com"));
    /// ```
    #[must_use]
    pub fn user(user_id: impl Into<String>, email: Option<impl Into<String>>) -> Self {
        Self::User {
            user_id: user_id.into(),
            email: email.map(Into::into),
            name: None,
            roles: Vec::new(),
        }
    }

    /// Creates a user identity with full details.
    #[must_use]
    pub fn user_full(
        user_id: impl Into<String>,
        email: Option<impl Into<String>>,
        name: Option<impl Into<String>>,
        roles: Vec<String>,
    ) -> Self {
        Self::User {
            user_id: user_id.into(),
            email: email.map(Into::into),
            name: name.map(Into::into),
            roles,
        }
    }

    /// Creates an API key identity.
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_core::CallerIdentity;
    ///
    /// let identity = CallerIdentity::api_key("key-abc123", Some("Production Key"));
    /// ```
    #[must_use]
    pub fn api_key(key_id: impl Into<String>, name: Option<impl Into<String>>) -> Self {
        Self::ApiKey {
            key_id: key_id.into(),
            name: name.map(Into::into),
            scopes: Vec::new(),
        }
    }

    /// Creates an API key identity with scopes.
    #[must_use]
    pub fn api_key_with_scopes(
        key_id: impl Into<String>,
        name: Option<impl Into<String>>,
        scopes: Vec<String>,
    ) -> Self {
        Self::ApiKey {
            key_id: key_id.into(),
            name: name.map(Into::into),
            scopes,
        }
    }

    /// Returns `true` if this is an anonymous identity.
    #[must_use]
    pub const fn is_anonymous(&self) -> bool {
        matches!(self, Self::Anonymous)
    }

    /// Returns `true` if this is a service (SPIFFE) identity.
    #[must_use]
    pub const fn is_service(&self) -> bool {
        matches!(self, Self::Spiffe { .. })
    }

    /// Returns `true` if this is a user identity.
    #[must_use]
    pub const fn is_user(&self) -> bool {
        matches!(self, Self::User { .. })
    }

    /// Returns `true` if this is an API key identity.
    #[must_use]
    pub const fn is_api_key(&self) -> bool {
        matches!(self, Self::ApiKey { .. })
    }

    /// Returns a string identifier suitable for logging.
    ///
    /// This never returns sensitive information.
    #[must_use]
    pub fn log_id(&self) -> String {
        match self {
            Self::Spiffe { spiffe_id } => spiffe_id.clone(),
            Self::User { user_id, .. } => format!("user:{user_id}"),
            Self::ApiKey { key_id, .. } => format!("apikey:{key_id}"),
            Self::Anonymous => "anonymous".to_string(),
        }
    }
}

impl Default for CallerIdentity {
    fn default() -> Self {
        Self::Anonymous
    }
}

impl std::fmt::Display for CallerIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spiffe { spiffe_id } => write!(f, "SPIFFE({spiffe_id})"),
            Self::User { user_id, email, .. } => {
                if let Some(email) = email {
                    write!(f, "User({user_id}, {email})")
                } else {
                    write!(f, "User({user_id})")
                }
            }
            Self::ApiKey { key_id, name, .. } => {
                if let Some(name) = name {
                    write!(f, "ApiKey({key_id}, {name})")
                } else {
                    write!(f, "ApiKey({key_id})")
                }
            }
            Self::Anonymous => write!(f, "Anonymous"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spiffe_identity() {
        let identity = CallerIdentity::spiffe("spiffe://example.org/service/users");

        assert!(identity.is_service());
        assert!(!identity.is_anonymous());
        assert!(!identity.is_user());
        assert!(!identity.is_api_key());

        if let CallerIdentity::Spiffe { spiffe_id } = &identity {
            assert_eq!(spiffe_id, "spiffe://example.org/service/users");
        } else {
            panic!("Expected Spiffe identity");
        }
    }

    #[test]
    fn test_user_identity() {
        let identity = CallerIdentity::user("user-123", Some("alice@example.com"));

        assert!(identity.is_user());
        assert!(!identity.is_anonymous());

        if let CallerIdentity::User { user_id, email, .. } = &identity {
            assert_eq!(user_id, "user-123");
            assert_eq!(email.as_deref(), Some("alice@example.com"));
        } else {
            panic!("Expected User identity");
        }
    }

    #[test]
    fn test_api_key_identity() {
        let identity = CallerIdentity::api_key_with_scopes(
            "key-abc",
            Some("Prod Key"),
            vec!["read".to_string(), "write".to_string()],
        );

        assert!(identity.is_api_key());
        assert!(!identity.is_anonymous());

        if let CallerIdentity::ApiKey {
            key_id,
            name,
            scopes,
        } = &identity
        {
            assert_eq!(key_id, "key-abc");
            assert_eq!(name.as_deref(), Some("Prod Key"));
            assert_eq!(scopes, &vec!["read".to_string(), "write".to_string()]);
        } else {
            panic!("Expected ApiKey identity");
        }
    }

    #[test]
    fn test_anonymous_identity() {
        let identity = CallerIdentity::Anonymous;

        assert!(identity.is_anonymous());
        assert!(!identity.is_service());
        assert!(!identity.is_user());
        assert!(!identity.is_api_key());
    }

    #[test]
    fn test_log_id() {
        assert_eq!(
            CallerIdentity::spiffe("spiffe://example.org/svc").log_id(),
            "spiffe://example.org/svc"
        );
        assert_eq!(
            CallerIdentity::user("u123", None::<String>).log_id(),
            "user:u123"
        );
        assert_eq!(
            CallerIdentity::api_key("k456", None::<String>).log_id(),
            "apikey:k456"
        );
        assert_eq!(CallerIdentity::Anonymous.log_id(), "anonymous");
    }

    #[test]
    fn test_serialization() {
        let identity = CallerIdentity::user("u123", Some("test@example.com"));
        let json = serde_json::to_string(&identity).expect("serialization should work");
        assert!(json.contains("\"type\":\"user\""));
        assert!(json.contains("\"user_id\":\"u123\""));

        let parsed: CallerIdentity =
            serde_json::from_str(&json).expect("deserialization should work");
        assert_eq!(identity, parsed);
    }

    #[test]
    fn test_display() {
        assert_eq!(
            CallerIdentity::spiffe("spiffe://example.org/svc").to_string(),
            "SPIFFE(spiffe://example.org/svc)"
        );
        assert_eq!(
            CallerIdentity::user("u123", Some("a@b.com")).to_string(),
            "User(u123, a@b.com)"
        );
        assert_eq!(
            CallerIdentity::user("u123", None::<String>).to_string(),
            "User(u123)"
        );
        assert_eq!(
            CallerIdentity::api_key("k1", Some("Key")).to_string(),
            "ApiKey(k1, Key)"
        );
        assert_eq!(CallerIdentity::Anonymous.to_string(), "Anonymous");
    }
}
