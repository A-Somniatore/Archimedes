//! Path parameter extraction and storage.
//!
//! This module provides efficient storage for extracted path parameters
//! using a small-vector optimization to avoid heap allocations for
//! common cases (1-4 parameters).

use smallvec::SmallVec;

/// Maximum number of parameters stored inline (stack allocated).
const INLINE_PARAMS: usize = 4;

/// Extracted path parameters from a route match.
///
/// Uses small-vector optimization to avoid heap allocation for common
/// cases with few parameters. Parameters are stored as (name, value) pairs.
///
/// # Example
///
/// ```rust
/// use archimedes_router::Params;
///
/// let mut params = Params::new();
/// params.push("userId", "123");
/// params.push("action", "view");
///
/// assert_eq!(params.get("userId"), Some("123"));
/// assert_eq!(params.get("action"), Some("view"));
/// assert_eq!(params.get("unknown"), None);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Params {
    /// Storage for parameter (name, value) pairs
    inner: SmallVec<[(String, String); INLINE_PARAMS]>,
}

impl Params {
    /// Creates a new empty parameter set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a params set with the given capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: SmallVec::with_capacity(capacity),
        }
    }

    /// Adds a parameter to the set.
    pub fn push(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.inner.push((name.into(), value.into()));
    }

    /// Returns the value for a parameter by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.inner
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// Returns true if there are no parameters.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of parameters.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns an iterator over the parameters.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.inner.iter().map(|(n, v)| (n.as_str(), v.as_str()))
    }

    /// Clears all parameters, retaining allocated capacity.
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

impl<'a> IntoIterator for &'a Params {
    type Item = (&'a str, &'a str);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'a, (String, String)>,
        fn(&'a (String, String)) -> (&'a str, &'a str),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter().map(|(n, v)| (n.as_str(), v.as_str()))
    }
}

impl FromIterator<(String, String)> for Params {
    fn from_iter<I: IntoIterator<Item = (String, String)>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_new() {
        let params = Params::new();
        assert!(params.is_empty());
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_params_push_and_get() {
        let mut params = Params::new();
        params.push("id", "123");
        params.push("name", "alice");

        assert_eq!(params.get("id"), Some("123"));
        assert_eq!(params.get("name"), Some("alice"));
        assert_eq!(params.get("unknown"), None);
    }

    #[test]
    fn test_params_len() {
        let mut params = Params::new();
        assert_eq!(params.len(), 0);

        params.push("a", "1");
        assert_eq!(params.len(), 1);

        params.push("b", "2");
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_params_iter() {
        let mut params = Params::new();
        params.push("a", "1");
        params.push("b", "2");

        let pairs: Vec<_> = params.iter().collect();
        assert_eq!(pairs, vec![("a", "1"), ("b", "2")]);
    }

    #[test]
    fn test_params_clear() {
        let mut params = Params::new();
        params.push("a", "1");
        params.push("b", "2");

        params.clear();
        assert!(params.is_empty());
    }

    #[test]
    fn test_params_from_iterator() {
        let pairs = vec![
            ("a".to_string(), "1".to_string()),
            ("b".to_string(), "2".to_string()),
        ];

        let params: Params = pairs.into_iter().collect();
        assert_eq!(params.len(), 2);
        assert_eq!(params.get("a"), Some("1"));
        assert_eq!(params.get("b"), Some("2"));
    }

    #[test]
    fn test_params_with_capacity() {
        let params = Params::with_capacity(10);
        assert!(params.is_empty());
    }

    #[test]
    fn test_params_many_params() {
        // Test that we can handle more than INLINE_PARAMS
        let mut params = Params::new();
        for i in 0..10 {
            params.push(format!("key{i}"), format!("value{i}"));
        }

        assert_eq!(params.len(), 10);
        assert_eq!(params.get("key5"), Some("value5"));
    }
}
