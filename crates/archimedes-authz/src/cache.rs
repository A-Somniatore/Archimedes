//! Decision caching for authorization.
//!
//! Caches policy decisions to avoid re-evaluating the same requests.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};

use themis_platform_types::{PolicyDecision, PolicyInput};

/// Configuration for the decision cache.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache.
    pub max_entries: usize,
    /// Time-to-live for cached decisions.
    pub ttl: Duration,
    /// Whether to cache deny decisions.
    pub cache_denies: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            ttl: Duration::from_secs(300), // 5 minutes
            cache_denies: false,
        }
    }
}

impl CacheConfig {
    /// Create a production cache configuration.
    pub fn production() -> Self {
        Self {
            max_entries: 50_000,
            ttl: Duration::from_secs(60), // 1 minute
            cache_denies: false,
        }
    }

    /// Create a development cache configuration.
    pub fn development() -> Self {
        Self {
            max_entries: 1_000,
            ttl: Duration::from_secs(30),
            cache_denies: true,
        }
    }

    /// Disable caching.
    pub fn disabled() -> Self {
        Self {
            max_entries: 0,
            ttl: Duration::ZERO,
            cache_denies: false,
        }
    }
}

/// Cache key derived from policy input.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    /// Caller identity hash.
    caller_hash: u64,
    /// Service name.
    service: String,
    /// Operation ID.
    operation_id: String,
    /// HTTP method.
    method: String,
}

impl CacheKey {
    fn from_input(input: &PolicyInput) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        // Hash the caller identity
        format!("{:?}", input.caller).hash(&mut hasher);
        let caller_hash = hasher.finish();

        Self {
            caller_hash,
            service: input.service.clone(),
            operation_id: input.operation_id.clone(),
            method: input.method.clone(),
        }
    }
}

/// Cached decision entry.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The cached decision.
    decision: PolicyDecision,
    /// When the entry was created.
    created_at: Instant,
}

impl CacheEntry {
    fn new(decision: PolicyDecision) -> Self {
        Self {
            decision,
            created_at: Instant::now(),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }
}

/// Cache statistics.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Number of entries currently in cache.
    pub size: usize,
    /// Number of evictions due to capacity.
    pub evictions: u64,
}

/// Decision cache for authorization.
#[derive(Debug)]
pub struct DecisionCache {
    /// Cache configuration.
    config: CacheConfig,
    /// Cached decisions.
    entries: RwLock<HashMap<CacheKey, CacheEntry>>,
    /// Cache hit counter.
    hits: AtomicU64,
    /// Cache miss counter.
    misses: AtomicU64,
    /// Eviction counter.
    evictions: AtomicU64,
}

impl DecisionCache {
    /// Create a new decision cache.
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            entries: RwLock::new(HashMap::new()),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }

    /// Get a cached decision for the given input.
    pub fn get(&self, input: &PolicyInput) -> Option<PolicyDecision> {
        if self.config.max_entries == 0 {
            self.misses.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        let key = CacheKey::from_input(input);
        let entries = self.entries.read().unwrap();

        if let Some(entry) = entries.get(&key) {
            if !entry.is_expired(self.config.ttl) {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Some(entry.decision.clone());
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Insert a decision into the cache.
    pub fn insert(&self, input: &PolicyInput, decision: &PolicyDecision) {
        if self.config.max_entries == 0 {
            return;
        }

        let key = CacheKey::from_input(input);
        let entry = CacheEntry::new(decision.clone());

        let mut entries = self.entries.write().unwrap();

        // Evict expired entries if we're at capacity
        if entries.len() >= self.config.max_entries {
            self.evict_expired(&mut entries);
        }

        // If still at capacity, evict oldest entries
        while entries.len() >= self.config.max_entries {
            if let Some(oldest_key) = self.find_oldest(&entries) {
                entries.remove(&oldest_key);
                self.evictions.fetch_add(1, Ordering::Relaxed);
            } else {
                break;
            }
        }

        entries.insert(key, entry);
    }

    /// Check if a decision should be cached.
    pub fn should_cache(&self, decision: &PolicyDecision) -> bool {
        if self.config.max_entries == 0 {
            return false;
        }
        decision.allowed || self.config.cache_denies
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        let mut entries = self.entries.write().unwrap();
        entries.clear();
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        let entries = self.entries.read().unwrap();
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            size: entries.len(),
            evictions: self.evictions.load(Ordering::Relaxed),
        }
    }

    fn evict_expired(&self, entries: &mut HashMap<CacheKey, CacheEntry>) {
        let ttl = self.config.ttl;
        let before = entries.len();
        entries.retain(|_, v| !v.is_expired(ttl));
        let evicted = before - entries.len();
        if evicted > 0 {
            self.evictions.fetch_add(evicted as u64, Ordering::Relaxed);
        }
    }

    fn find_oldest(&self, entries: &HashMap<CacheKey, CacheEntry>) -> Option<CacheKey> {
        entries
            .iter()
            .min_by_key(|(_, v)| v.created_at)
            .map(|(k, _)| k.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use themis_platform_types::{CallerIdentity, RequestId};

    fn create_test_input(operation_id: &str) -> PolicyInput {
        PolicyInput::builder()
            .caller(CallerIdentity::user("user-123", "user@example.com"))
            .service("test-service")
            .operation_id(operation_id)
            .method("GET")
            .path("/test")
            .request_id(RequestId::new())
            .try_build()
            .unwrap()
    }

    fn create_allow_decision() -> PolicyDecision {
        PolicyDecision::allow("authz", "1.0.0")
    }

    fn create_deny_decision() -> PolicyDecision {
        PolicyDecision::deny("authz", "1.0.0", "access denied")
    }

    #[test]
    fn test_cache_hit_miss() {
        let cache = DecisionCache::new(CacheConfig::default());
        let input = create_test_input("testOp");

        // Miss on empty cache
        assert!(cache.get(&input).is_none());

        // Insert and hit
        let decision = create_allow_decision();
        cache.insert(&input, &decision);
        let cached = cache.get(&input);
        assert!(cached.is_some());
        assert!(cached.unwrap().allowed);
    }

    #[test]
    fn test_cache_stats() {
        let cache = DecisionCache::new(CacheConfig::default());
        let input = create_test_input("testOp");
        let decision = create_allow_decision();

        // Generate some hits and misses
        cache.get(&input); // miss
        cache.insert(&input, &decision);
        cache.get(&input); // hit
        cache.get(&input); // hit

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.size, 1);
    }

    #[test]
    fn test_cache_disabled() {
        let cache = DecisionCache::new(CacheConfig::disabled());
        let input = create_test_input("testOp");
        let decision = create_allow_decision();

        cache.insert(&input, &decision);
        assert!(cache.get(&input).is_none());
    }

    #[test]
    fn test_should_cache_deny() {
        let config_no_deny = CacheConfig {
            cache_denies: false,
            ..Default::default()
        };
        let config_with_deny = CacheConfig {
            cache_denies: true,
            ..Default::default()
        };

        let cache_no_deny = DecisionCache::new(config_no_deny);
        let cache_with_deny = DecisionCache::new(config_with_deny);

        let allow = create_allow_decision();
        let deny = create_deny_decision();

        assert!(cache_no_deny.should_cache(&allow));
        assert!(!cache_no_deny.should_cache(&deny));
        assert!(cache_with_deny.should_cache(&allow));
        assert!(cache_with_deny.should_cache(&deny));
    }

    #[test]
    fn test_cache_clear() {
        let cache = DecisionCache::new(CacheConfig::default());
        let input = create_test_input("testOp");
        let decision = create_allow_decision();

        cache.insert(&input, &decision);
        assert!(cache.get(&input).is_some());

        cache.clear();
        assert!(cache.get(&input).is_none());
    }

    #[test]
    fn test_cache_key_different_ops() {
        let cache = DecisionCache::new(CacheConfig::default());
        let input1 = create_test_input("op1");
        let input2 = create_test_input("op2");
        let decision = create_allow_decision();

        cache.insert(&input1, &decision);

        assert!(cache.get(&input1).is_some());
        assert!(cache.get(&input2).is_none());
    }
}
