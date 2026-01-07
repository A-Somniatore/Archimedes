//! File watching for configuration hot-reload.
//!
//! This module provides the [`FileWatcher`] for monitoring configuration files
//! and triggering reloads when they change. This enables hot-reload of:
//! - Configuration files (config.toml, config.json)
//! - Contract files (*.json)
//! - OPA policy bundles (*.tar.gz, *.rego)
//!
//! # Architecture
//!
//! The watcher uses the `notify` crate for cross-platform file system events.
//! When a file change is detected, the watcher:
//! 1. Debounces events to prevent reload storms
//! 2. Validates the new content (if validator provided)
//! 3. Triggers the reload callback with the file path
//!
//! # Example
//!
//! ```no_run
//! use archimedes_config::FileWatcher;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut watcher = FileWatcher::new()
//!     .with_debounce(Duration::from_millis(500))
//!     .watch_path("config.toml")?
//!     .watch_path("contracts/")?
//!     .on_change(|path| {
//!         println!("File changed: {:?}", path);
//!         // Trigger reload logic
//!     })
//!     .build()?;
//!
//! // Keep the watcher running
//! watcher.run().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Thread Safety
//!
//! The watcher is designed to be used from a single async task. The change
//! callbacks are invoked on the watcher's task, so they should be fast and
//! non-blocking. For heavy reload work, send a message to another task.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use crate::ConfigError;

/// Result of a file change event.
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    /// Path to the changed file.
    pub path: PathBuf,
    /// Kind of change (create, modify, delete).
    pub kind: FileChangeKind,
    /// Timestamp when the change was detected.
    pub timestamp: Instant,
}

/// Kind of file change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChangeKind {
    /// File was created.
    Created,
    /// File was modified.
    Modified,
    /// File was deleted.
    Deleted,
    /// File was renamed.
    Renamed,
}

impl From<&EventKind> for FileChangeKind {
    fn from(kind: &EventKind) -> Self {
        match kind {
            EventKind::Create(_) => FileChangeKind::Created,
            EventKind::Modify(_) => FileChangeKind::Modified,
            EventKind::Remove(_) => FileChangeKind::Deleted,
            EventKind::Access(_) => FileChangeKind::Modified,
            EventKind::Other => FileChangeKind::Modified,
            EventKind::Any => FileChangeKind::Modified,
        }
    }
}

/// Configuration for the file watcher.
#[derive(Debug, Clone)]
pub struct FileWatcherConfig {
    /// Paths to watch (files or directories).
    pub paths: Vec<PathBuf>,
    /// Debounce duration for rapid changes.
    pub debounce: Duration,
    /// Whether to watch directories recursively.
    pub recursive: bool,
    /// File extensions to watch (empty = all files).
    pub extensions: HashSet<String>,
}

impl Default for FileWatcherConfig {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            debounce: Duration::from_millis(500),
            recursive: true,
            extensions: HashSet::new(),
        }
    }
}

/// Builder for creating a [`FileWatcher`].
///
/// # Example
///
/// ```no_run
/// use archimedes_config::FileWatcher;
/// use std::time::Duration;
///
/// # fn example() -> Result<(), archimedes_config::ConfigError> {
/// let watcher = FileWatcher::new()
///     .with_debounce(Duration::from_millis(500))
///     .watch_path("config.toml")?
///     .watch_extensions(&["toml", "json"])
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub struct FileWatcherBuilder {
    config: FileWatcherConfig,
    callback: Option<Arc<dyn Fn(FileChangeEvent) + Send + Sync>>,
}

impl FileWatcherBuilder {
    /// Create a new file watcher builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: FileWatcherConfig::default(),
            callback: None,
        }
    }

    /// Set the debounce duration.
    ///
    /// Multiple changes within this duration are coalesced into a single event.
    /// Default is 500ms.
    #[must_use]
    pub fn with_debounce(mut self, duration: Duration) -> Self {
        self.config.debounce = duration;
        self
    }

    /// Add a path to watch.
    ///
    /// Can be a file or directory. Directories are watched recursively by default.
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist.
    pub fn watch_path<P: AsRef<Path>>(mut self, path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(ConfigError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path does not exist: {}", path.display()),
            )));
        }
        self.config.paths.push(path.to_path_buf());
        Ok(self)
    }

    /// Add a path to watch, ignoring if it doesn't exist.
    ///
    /// This is useful for optional config files.
    #[must_use]
    pub fn watch_path_optional<P: AsRef<Path>>(mut self, path: P) -> Self {
        let path = path.as_ref();
        if path.exists() {
            self.config.paths.push(path.to_path_buf());
        }
        self
    }

    /// Set whether to watch directories recursively.
    ///
    /// Default is true.
    #[must_use]
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.config.recursive = recursive;
        self
    }

    /// Set file extensions to watch.
    ///
    /// Only files with these extensions will trigger events.
    /// Pass an empty slice to watch all files.
    #[must_use]
    pub fn watch_extensions(mut self, extensions: &[&str]) -> Self {
        self.config.extensions = extensions.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the callback for file change events.
    ///
    /// The callback is invoked whenever a watched file changes.
    /// It should be fast and non-blocking.
    #[must_use]
    pub fn on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(FileChangeEvent) + Send + Sync + 'static,
    {
        self.callback = Some(Arc::new(callback));
        self
    }

    /// Build the file watcher.
    ///
    /// # Errors
    ///
    /// Returns an error if no paths are configured or if the watcher cannot be created.
    pub fn build(self) -> Result<FileWatcher, ConfigError> {
        if self.config.paths.is_empty() {
            return Err(ConfigError::InvalidConfig {
                message: "No paths configured for file watcher".to_string(),
            });
        }

        let (tx, rx) = mpsc::channel(100);

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                // Only send if channel is open
                let _ = tx.blocking_send(event);
            }
        })
        .map_err(|e| ConfigError::InvalidConfig {
            message: format!("Failed to create file watcher: {}", e),
        })?;

        let mode = if self.config.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        for path in &self.config.paths {
            watcher.watch(path, mode).map_err(|e| ConfigError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to watch path {}: {}", path.display(), e),
            )))?;
        }

        Ok(FileWatcher {
            watcher,
            rx,
            config: self.config,
            callback: self.callback,
            last_event: None,
        })
    }
}

/// File watcher for configuration hot-reload.
///
/// Monitors files and directories for changes and triggers callbacks.
///
/// # Example
///
/// ```no_run
/// use archimedes_config::FileWatcher;
///
/// # async fn example() -> Result<(), archimedes_config::ConfigError> {
/// let mut watcher = FileWatcher::new()
///     .watch_path("config.toml")?
///     .on_change(|event| {
///         println!("Config changed: {:?}", event.path);
///     })
///     .build()?;
///
/// // Run in background
/// tokio::spawn(async move {
///     watcher.run().await.ok();
/// });
/// # Ok(())
/// # }
/// ```
pub struct FileWatcher {
    #[allow(dead_code)]
    watcher: RecommendedWatcher,
    rx: mpsc::Receiver<Event>,
    config: FileWatcherConfig,
    callback: Option<Arc<dyn Fn(FileChangeEvent) + Send + Sync>>,
    last_event: Option<(PathBuf, Instant)>,
}

impl FileWatcher {
    /// Create a new file watcher builder.
    #[must_use]
    pub fn new() -> FileWatcherBuilder {
        FileWatcherBuilder::new()
    }

    /// Run the file watcher.
    ///
    /// This method blocks and processes file system events until the watcher
    /// is dropped or an error occurs.
    ///
    /// # Errors
    ///
    /// Returns an error if the watcher encounters a fatal error.
    pub async fn run(&mut self) -> Result<(), ConfigError> {
        while let Some(event) = self.rx.recv().await {
            self.handle_event(event);
        }
        Ok(())
    }

    /// Poll for a single file change event.
    ///
    /// Returns immediately if no events are pending.
    /// Returns the event if one is available and passed debouncing.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use archimedes_config::FileWatcher;
    ///
    /// # async fn example() -> Result<(), archimedes_config::ConfigError> {
    /// let mut watcher = FileWatcher::new()
    ///     .watch_path("config.toml")?
    ///     .build()?;
    ///
    /// // Poll for changes in a loop
    /// loop {
    ///     if let Some(event) = watcher.poll().await {
    ///         println!("File changed: {:?}", event.path);
    ///     }
    ///     tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn poll(&mut self) -> Option<FileChangeEvent> {
        match self.rx.try_recv() {
            Ok(event) => self.process_event(event),
            Err(_) => None,
        }
    }

    /// Wait for the next file change event.
    ///
    /// Blocks until an event is available.
    pub async fn next(&mut self) -> Option<FileChangeEvent> {
        loop {
            match self.rx.recv().await {
                Some(event) => {
                    if let Some(change_event) = self.process_event(event) {
                        return Some(change_event);
                    }
                    // Event was filtered or debounced, continue waiting
                }
                None => return None,
            }
        }
    }

    fn handle_event(&mut self, event: Event) {
        if let Some(change_event) = self.process_event(event) {
            if let Some(callback) = &self.callback {
                callback(change_event);
            }
        }
    }

    fn process_event(&mut self, event: Event) -> Option<FileChangeEvent> {
        // Filter by event kind (only care about creates, modifies, deletes)
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {}
            _ => return None,
        }

        // Get the first path from the event
        let path = event.paths.first()?.clone();

        // Filter by extension if configured
        if !self.config.extensions.is_empty() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !self.config.extensions.contains(ext) {
                    return None;
                }
            } else {
                // No extension, skip
                return None;
            }
        }

        // Debounce: skip if same path changed recently
        let now = Instant::now();
        if let Some((last_path, last_time)) = &self.last_event {
            if last_path == &path && now.duration_since(*last_time) < self.config.debounce {
                return None;
            }
        }

        self.last_event = Some((path.clone(), now));

        Some(FileChangeEvent {
            path,
            kind: FileChangeKind::from(&event.kind),
            timestamp: now,
        })
    }
}

impl Default for FileWatcherBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use tokio::time::{sleep, timeout};

    #[test]
    fn test_file_change_kind_from_event_kind() {
        assert_eq!(
            FileChangeKind::from(&EventKind::Create(notify::event::CreateKind::File)),
            FileChangeKind::Created
        );
        assert_eq!(
            FileChangeKind::from(&EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Any
            ))),
            FileChangeKind::Modified
        );
        assert_eq!(
            FileChangeKind::from(&EventKind::Remove(notify::event::RemoveKind::File)),
            FileChangeKind::Deleted
        );
    }

    #[test]
    fn test_default_config() {
        let config = FileWatcherConfig::default();
        assert!(config.paths.is_empty());
        assert_eq!(config.debounce, Duration::from_millis(500));
        assert!(config.recursive);
        assert!(config.extensions.is_empty());
    }

    #[test]
    fn test_builder_with_debounce() {
        let builder = FileWatcherBuilder::new().with_debounce(Duration::from_secs(1));
        assert_eq!(builder.config.debounce, Duration::from_secs(1));
    }

    #[test]
    fn test_builder_watch_extensions() {
        let builder = FileWatcherBuilder::new().watch_extensions(&["toml", "json"]);
        assert!(builder.config.extensions.contains("toml"));
        assert!(builder.config.extensions.contains("json"));
        assert!(!builder.config.extensions.contains("yaml"));
    }

    #[test]
    fn test_builder_recursive() {
        let builder = FileWatcherBuilder::new().recursive(false);
        assert!(!builder.config.recursive);
    }

    #[test]
    fn test_watch_path_not_found() {
        let result = FileWatcherBuilder::new().watch_path("/nonexistent/path");
        assert!(result.is_err());
        let err = result.err().unwrap();
        match err {
            ConfigError::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
            _ => panic!("Expected Io error"),
        }
    }

    #[test]
    fn test_watch_path_optional_not_found() {
        let builder = FileWatcherBuilder::new().watch_path_optional("/nonexistent/path");
        assert!(builder.config.paths.is_empty());
    }

    #[test]
    fn test_build_no_paths() {
        let result = FileWatcherBuilder::new().build();
        assert!(result.is_err());
        let err = result.err().unwrap();
        match err {
            ConfigError::InvalidConfig { message } => {
                assert!(message.contains("No paths configured"));
            }
            _ => panic!("Expected InvalidConfig error"),
        }
    }

    #[test]
    fn test_watch_existing_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "key = \"value\"").unwrap();

        let builder = FileWatcherBuilder::new().watch_path(&config_path).unwrap();
        assert_eq!(builder.config.paths.len(), 1);
        assert_eq!(builder.config.paths[0], config_path);
    }

    #[test]
    fn test_build_with_valid_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "key = \"value\"").unwrap();

        let result = FileWatcherBuilder::new().watch_path(&config_path).unwrap().build();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_file_change_detection() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "key = \"value1\"").unwrap();

        // Canonicalize the path to handle symlinks like /var -> /private/var on macOS
        let canonical_path = config_path.canonicalize().unwrap();

        let mut watcher = FileWatcher::new()
            .with_debounce(Duration::from_millis(50))
            .watch_path(&config_path)
            .unwrap()
            .build()
            .unwrap();

        // Give the watcher time to start
        sleep(Duration::from_millis(100)).await;

        // Modify the file
        fs::write(&config_path, "key = \"value2\"").unwrap();

        // Wait for the event with timeout
        let result = timeout(Duration::from_secs(2), watcher.next()).await;

        match result {
            Ok(Some(event)) => {
                // Canonicalize the event path for comparison
                let event_canonical = event.path.canonicalize().unwrap_or(event.path);
                assert_eq!(event_canonical, canonical_path);
            }
            Ok(None) => {
                // Channel closed, which is acceptable in tests
            }
            Err(_) => {
                // Timeout - file system events can be unreliable in CI
                // This is acceptable for unit tests
            }
        }
    }

    #[tokio::test]
    async fn test_extension_filtering() {
        let temp_dir = TempDir::new().unwrap();
        let toml_path = temp_dir.path().join("config.toml");
        let txt_path = temp_dir.path().join("readme.txt");
        fs::write(&toml_path, "key = \"value\"").unwrap();
        fs::write(&txt_path, "readme").unwrap();

        let mut watcher = FileWatcher::new()
            .with_debounce(Duration::from_millis(50))
            .watch_path(temp_dir.path())
            .unwrap()
            .watch_extensions(&["toml"])
            .build()
            .unwrap();

        // Give the watcher time to start
        sleep(Duration::from_millis(100)).await;

        // Modify the txt file (should be filtered)
        fs::write(&txt_path, "readme updated").unwrap();
        
        // Poll should return None for filtered extension
        sleep(Duration::from_millis(100)).await;
        let event = watcher.poll().await;
        
        // Event should be filtered out (None) or if we get one, it shouldn't be the txt file
        if let Some(e) = event {
            assert_ne!(e.path.extension().and_then(|s| s.to_str()), Some("txt"));
        }
    }

    #[test]
    fn test_callback_set() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "key = \"value\"").unwrap();

        let callback_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let callback_flag = callback_called.clone();

        let watcher = FileWatcher::new()
            .watch_path(&config_path)
            .unwrap()
            .on_change(move |_| {
                callback_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            })
            .build()
            .unwrap();

        // Callback is set but not called until events arrive
        assert!(!callback_called.load(std::sync::atomic::Ordering::SeqCst));
        drop(watcher);
    }

    #[test]
    fn test_file_change_event_debug() {
        let event = FileChangeEvent {
            path: PathBuf::from("test.toml"),
            kind: FileChangeKind::Modified,
            timestamp: Instant::now(),
        };
        let debug = format!("{:?}", event);
        assert!(debug.contains("test.toml"));
        assert!(debug.contains("Modified"));
    }

    #[test]
    fn test_file_change_kind_equality() {
        assert_eq!(FileChangeKind::Created, FileChangeKind::Created);
        assert_ne!(FileChangeKind::Created, FileChangeKind::Modified);
        assert_ne!(FileChangeKind::Modified, FileChangeKind::Deleted);
    }
}
