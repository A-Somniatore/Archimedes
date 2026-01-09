//! Lifecycle hooks FFI for C/C++ bindings.
//!
//! Provides C-compatible lifecycle hook registration for startup and shutdown
//! events.
//!
//! ## Example (C)
//!
//! ```c
//! #include <archimedes.h>
//!
//! void on_startup(void* user_data) {
//!     printf("Starting up...\n");
//!     // Initialize database, etc.
//! }
//!
//! void on_shutdown(void* user_data) {
//!     printf("Shutting down...\n");
//!     // Cleanup resources
//! }
//!
//! int main() {
//!     archimedes_app app = archimedes_new(&config);
//!     
//!     // Register lifecycle hooks
//!     archimedes_on_startup(app, "db_connect", on_startup, NULL);
//!     archimedes_on_shutdown(app, "db_close", on_shutdown, NULL);
//!     
//!     archimedes_run(app);
//!     archimedes_free(app);
//!     return 0;
//! }
//! ```

use std::ffi::{c_char, CStr, CString};
use std::ptr;

/// Function pointer type for lifecycle hooks
pub type ArchimedesLifecycleHook = Option<unsafe extern "C" fn(user_data: *mut std::ffi::c_void)>;

/// Opaque lifecycle manager handle
#[repr(C)]
pub struct ArchimedesLifecycle {
    _opaque: [u8; 0],
}

/// A lifecycle hook entry
pub(crate) struct LifecycleHookEntry {
    /// Optional name for debugging
    pub name: Option<String>,
    /// The hook function
    pub hook: ArchimedesLifecycleHook,
    /// User-provided data
    pub user_data: *mut std::ffi::c_void,
}

// Mark as Send+Sync for internal use (pointers are FFI-safe)
unsafe impl Send for LifecycleHookEntry {}
unsafe impl Sync for LifecycleHookEntry {}

/// Internal lifecycle state
pub(crate) struct LifecycleState {
    /// Startup hooks (run in order)
    pub startup_hooks: Vec<LifecycleHookEntry>,
    /// Shutdown hooks (run in reverse order)
    pub shutdown_hooks: Vec<LifecycleHookEntry>,
}

impl LifecycleState {
    pub fn new() -> Self {
        Self {
            startup_hooks: Vec::new(),
            shutdown_hooks: Vec::new(),
        }
    }

    /// Add a startup hook
    pub fn add_startup(
        &mut self,
        name: Option<String>,
        hook: ArchimedesLifecycleHook,
        user_data: *mut std::ffi::c_void,
    ) -> usize {
        let index = self.startup_hooks.len();
        self.startup_hooks.push(LifecycleHookEntry {
            name,
            hook,
            user_data,
        });
        index
    }

    /// Add a shutdown hook
    pub fn add_shutdown(
        &mut self,
        name: Option<String>,
        hook: ArchimedesLifecycleHook,
        user_data: *mut std::ffi::c_void,
    ) -> usize {
        let index = self.shutdown_hooks.len();
        self.shutdown_hooks.push(LifecycleHookEntry {
            name,
            hook,
            user_data,
        });
        index
    }

    /// Run all startup hooks
    pub unsafe fn run_startup(&self) {
        for entry in &self.startup_hooks {
            if let Some(hook) = entry.hook {
                hook(entry.user_data);
            }
        }
    }

    /// Run all shutdown hooks (in reverse order)
    pub unsafe fn run_shutdown(&self) {
        for entry in self.shutdown_hooks.iter().rev() {
            if let Some(hook) = entry.hook {
                hook(entry.user_data);
            }
        }
    }
}

impl Default for LifecycleState {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new lifecycle manager
///
/// # Safety
///
/// Returns a pointer that must be freed with `archimedes_lifecycle_free`.
#[no_mangle]
pub extern "C" fn archimedes_lifecycle_new() -> *mut ArchimedesLifecycle {
    let state = Box::new(LifecycleState::new());
    Box::into_raw(state) as *mut ArchimedesLifecycle
}

/// Free a lifecycle manager
///
/// # Safety
///
/// - `lifecycle` must be a valid pointer returned by `archimedes_lifecycle_new`
#[no_mangle]
pub unsafe extern "C" fn archimedes_lifecycle_free(lifecycle: *mut ArchimedesLifecycle) {
    if lifecycle.is_null() {
        return;
    }
    let _ = Box::from_raw(lifecycle as *mut LifecycleState);
}

/// Register a startup hook
///
/// # Safety
///
/// - `lifecycle` must be a valid lifecycle pointer
/// - `name` is optional (can be NULL)
/// - `hook` must be a valid function pointer or NULL
/// - `user_data` is passed to the hook when called
///
/// Returns the hook index on success, or -1 on error.
#[no_mangle]
pub unsafe extern "C" fn archimedes_lifecycle_on_startup(
    lifecycle: *mut ArchimedesLifecycle,
    name: *const c_char,
    hook: ArchimedesLifecycleHook,
    user_data: *mut std::ffi::c_void,
) -> i32 {
    if lifecycle.is_null() {
        crate::set_last_error("lifecycle pointer is null");
        return -1;
    }

    let state = &mut *(lifecycle as *mut LifecycleState);

    let name_str = if name.is_null() {
        None
    } else {
        match CStr::from_ptr(name).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => {
                crate::set_last_error(format!("Invalid UTF-8 in name: {}", e));
                return -1;
            }
        }
    };

    state.add_startup(name_str, hook, user_data) as i32
}

/// Register a shutdown hook
///
/// # Safety
///
/// - `lifecycle` must be a valid lifecycle pointer
/// - `name` is optional (can be NULL)
/// - `hook` must be a valid function pointer or NULL
/// - `user_data` is passed to the hook when called
///
/// Returns the hook index on success, or -1 on error.
#[no_mangle]
pub unsafe extern "C" fn archimedes_lifecycle_on_shutdown(
    lifecycle: *mut ArchimedesLifecycle,
    name: *const c_char,
    hook: ArchimedesLifecycleHook,
    user_data: *mut std::ffi::c_void,
) -> i32 {
    if lifecycle.is_null() {
        crate::set_last_error("lifecycle pointer is null");
        return -1;
    }

    let state = &mut *(lifecycle as *mut LifecycleState);

    let name_str = if name.is_null() {
        None
    } else {
        match CStr::from_ptr(name).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => {
                crate::set_last_error(format!("Invalid UTF-8 in name: {}", e));
                return -1;
            }
        }
    };

    state.add_shutdown(name_str, hook, user_data) as i32
}

/// Get the number of startup hooks
///
/// # Safety
///
/// - `lifecycle` must be a valid lifecycle pointer
#[no_mangle]
pub unsafe extern "C" fn archimedes_lifecycle_startup_count(
    lifecycle: *const ArchimedesLifecycle,
) -> usize {
    if lifecycle.is_null() {
        return 0;
    }
    let state = &*(lifecycle as *const LifecycleState);
    state.startup_hooks.len()
}

/// Get the number of shutdown hooks
///
/// # Safety
///
/// - `lifecycle` must be a valid lifecycle pointer
#[no_mangle]
pub unsafe extern "C" fn archimedes_lifecycle_shutdown_count(
    lifecycle: *const ArchimedesLifecycle,
) -> usize {
    if lifecycle.is_null() {
        return 0;
    }
    let state = &*(lifecycle as *const LifecycleState);
    state.shutdown_hooks.len()
}

/// Run all startup hooks
///
/// # Safety
///
/// - `lifecycle` must be a valid lifecycle pointer
/// - All registered hooks must still be valid
///
/// Returns 0 on success, 1 on error.
#[no_mangle]
pub unsafe extern "C" fn archimedes_lifecycle_run_startup(
    lifecycle: *const ArchimedesLifecycle,
) -> i32 {
    if lifecycle.is_null() {
        crate::set_last_error("lifecycle pointer is null");
        return 1;
    }

    let state = &*(lifecycle as *const LifecycleState);
    state.run_startup();
    0
}

/// Run all shutdown hooks (in reverse order)
///
/// # Safety
///
/// - `lifecycle` must be a valid lifecycle pointer
/// - All registered hooks must still be valid
///
/// Returns 0 on success, 1 on error.
#[no_mangle]
pub unsafe extern "C" fn archimedes_lifecycle_run_shutdown(
    lifecycle: *const ArchimedesLifecycle,
) -> i32 {
    if lifecycle.is_null() {
        crate::set_last_error("lifecycle pointer is null");
        return 1;
    }

    let state = &*(lifecycle as *const LifecycleState);
    state.run_shutdown();
    0
}

/// Clear all hooks
///
/// # Safety
///
/// - `lifecycle` must be a valid lifecycle pointer
#[no_mangle]
pub unsafe extern "C" fn archimedes_lifecycle_clear(lifecycle: *mut ArchimedesLifecycle) {
    if lifecycle.is_null() {
        return;
    }
    let state = &mut *(lifecycle as *mut LifecycleState);
    state.startup_hooks.clear();
    state.shutdown_hooks.clear();
}

/// Check if there are any startup hooks
///
/// # Safety
///
/// - `lifecycle` must be a valid lifecycle pointer
///
/// Returns 1 if there are hooks, 0 otherwise.
#[no_mangle]
pub unsafe extern "C" fn archimedes_lifecycle_has_startup(
    lifecycle: *const ArchimedesLifecycle,
) -> i32 {
    if lifecycle.is_null() {
        return 0;
    }
    let state = &*(lifecycle as *const LifecycleState);
    if state.startup_hooks.is_empty() {
        0
    } else {
        1
    }
}

/// Check if there are any shutdown hooks
///
/// # Safety
///
/// - `lifecycle` must be a valid lifecycle pointer
///
/// Returns 1 if there are hooks, 0 otherwise.
#[no_mangle]
pub unsafe extern "C" fn archimedes_lifecycle_has_shutdown(
    lifecycle: *const ArchimedesLifecycle,
) -> i32 {
    if lifecycle.is_null() {
        return 0;
    }
    let state = &*(lifecycle as *const LifecycleState);
    if state.shutdown_hooks.is_empty() {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI32, Ordering};

    static STARTUP_CALLED: AtomicI32 = AtomicI32::new(0);
    static SHUTDOWN_CALLED: AtomicI32 = AtomicI32::new(0);

    extern "C" fn test_startup_hook(_user_data: *mut std::ffi::c_void) {
        STARTUP_CALLED.fetch_add(1, Ordering::SeqCst);
    }

    extern "C" fn test_shutdown_hook(_user_data: *mut std::ffi::c_void) {
        SHUTDOWN_CALLED.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn test_lifecycle_creation() {
        unsafe {
            let lifecycle = archimedes_lifecycle_new();
            assert!(!lifecycle.is_null());
            assert_eq!(archimedes_lifecycle_startup_count(lifecycle), 0);
            assert_eq!(archimedes_lifecycle_shutdown_count(lifecycle), 0);
            archimedes_lifecycle_free(lifecycle);
        }
    }

    #[test]
    fn test_lifecycle_startup_hook() {
        unsafe {
            let lifecycle = archimedes_lifecycle_new();

            let name = CString::new("test_hook").unwrap();
            let idx = archimedes_lifecycle_on_startup(
                lifecycle,
                name.as_ptr(),
                Some(test_startup_hook),
                ptr::null_mut(),
            );
            assert_eq!(idx, 0);
            assert_eq!(archimedes_lifecycle_startup_count(lifecycle), 1);

            archimedes_lifecycle_free(lifecycle);
        }
    }

    #[test]
    fn test_lifecycle_shutdown_hook() {
        unsafe {
            let lifecycle = archimedes_lifecycle_new();

            let idx = archimedes_lifecycle_on_shutdown(
                lifecycle,
                ptr::null(), // No name
                Some(test_shutdown_hook),
                ptr::null_mut(),
            );
            assert_eq!(idx, 0);
            assert_eq!(archimedes_lifecycle_shutdown_count(lifecycle), 1);

            archimedes_lifecycle_free(lifecycle);
        }
    }

    #[test]
    fn test_lifecycle_run_startup() {
        unsafe {
            STARTUP_CALLED.store(0, Ordering::SeqCst);

            let lifecycle = archimedes_lifecycle_new();
            archimedes_lifecycle_on_startup(
                lifecycle,
                ptr::null(),
                Some(test_startup_hook),
                ptr::null_mut(),
            );
            archimedes_lifecycle_on_startup(
                lifecycle,
                ptr::null(),
                Some(test_startup_hook),
                ptr::null_mut(),
            );

            let result = archimedes_lifecycle_run_startup(lifecycle);
            assert_eq!(result, 0);
            assert_eq!(STARTUP_CALLED.load(Ordering::SeqCst), 2);

            archimedes_lifecycle_free(lifecycle);
        }
    }

    #[test]
    fn test_lifecycle_has_hooks() {
        unsafe {
            let lifecycle = archimedes_lifecycle_new();

            assert_eq!(archimedes_lifecycle_has_startup(lifecycle), 0);
            assert_eq!(archimedes_lifecycle_has_shutdown(lifecycle), 0);

            archimedes_lifecycle_on_startup(
                lifecycle,
                ptr::null(),
                Some(test_startup_hook),
                ptr::null_mut(),
            );
            assert_eq!(archimedes_lifecycle_has_startup(lifecycle), 1);

            archimedes_lifecycle_on_shutdown(
                lifecycle,
                ptr::null(),
                Some(test_shutdown_hook),
                ptr::null_mut(),
            );
            assert_eq!(archimedes_lifecycle_has_shutdown(lifecycle), 1);

            archimedes_lifecycle_free(lifecycle);
        }
    }

    #[test]
    fn test_lifecycle_clear() {
        unsafe {
            let lifecycle = archimedes_lifecycle_new();

            archimedes_lifecycle_on_startup(
                lifecycle,
                ptr::null(),
                Some(test_startup_hook),
                ptr::null_mut(),
            );
            archimedes_lifecycle_on_shutdown(
                lifecycle,
                ptr::null(),
                Some(test_shutdown_hook),
                ptr::null_mut(),
            );

            assert_eq!(archimedes_lifecycle_startup_count(lifecycle), 1);
            assert_eq!(archimedes_lifecycle_shutdown_count(lifecycle), 1);

            archimedes_lifecycle_clear(lifecycle);

            assert_eq!(archimedes_lifecycle_startup_count(lifecycle), 0);
            assert_eq!(archimedes_lifecycle_shutdown_count(lifecycle), 0);

            archimedes_lifecycle_free(lifecycle);
        }
    }

    #[test]
    fn test_null_safety() {
        unsafe {
            assert_eq!(
                archimedes_lifecycle_on_startup(ptr::null_mut(), ptr::null(), None, ptr::null_mut()),
                -1
            );
            assert_eq!(
                archimedes_lifecycle_on_shutdown(
                    ptr::null_mut(),
                    ptr::null(),
                    None,
                    ptr::null_mut()
                ),
                -1
            );
            assert_eq!(archimedes_lifecycle_startup_count(ptr::null()), 0);
            assert_eq!(archimedes_lifecycle_shutdown_count(ptr::null()), 0);
            assert_eq!(archimedes_lifecycle_run_startup(ptr::null()), 1);
            assert_eq!(archimedes_lifecycle_run_shutdown(ptr::null()), 1);
            assert_eq!(archimedes_lifecycle_has_startup(ptr::null()), 0);
            assert_eq!(archimedes_lifecycle_has_shutdown(ptr::null()), 0);
        }
    }
}
