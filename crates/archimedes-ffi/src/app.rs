//! Application lifecycle management
//!
//! This module provides the main Archimedes application structure and lifecycle
//! functions for FFI consumers.

use crate::config::{ArchimedesConfig, InternalConfig};
use crate::error::FfiError;
use crate::handler::HandlerRegistry;
use crate::types::{ArchimedesError, ArchimedesHandlerFn};
use std::ffi::{c_char, CStr, CString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Opaque application handle for FFI
///
/// This represents a running Archimedes application instance.
#[repr(C)]
pub struct ArchimedesApp {
    _opaque: [u8; 0],
}

/// Internal application state
pub(crate) struct AppState {
    /// Configuration
    pub config: InternalConfig,
    /// Handler registry
    pub handlers: Arc<HandlerRegistry>,
    /// Running flag
    pub running: Arc<AtomicBool>,
    /// Contract JSON (stored for lifetime)
    #[allow(dead_code)]
    pub contract_json: Option<String>,
}

impl AppState {
    /// Create a new application state
    pub fn new(config: InternalConfig) -> Self {
        Self {
            config,
            handlers: Arc::new(HandlerRegistry::new()),
            running: Arc::new(AtomicBool::new(false)),
            contract_json: None,
        }
    }

    /// Check if the app is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Set running state
    pub fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::SeqCst);
    }
}

/// Create a new Archimedes application
///
/// # Safety
///
/// - `config` must be a valid pointer to an `ArchimedesConfig` struct
/// - String pointers in config must be valid null-terminated UTF-8 strings
///
/// Returns a pointer to the application handle, or null on error.
/// Use `archimedes_last_error()` to get the error message on failure.
#[no_mangle]
pub unsafe extern "C" fn archimedes_new(config: *const ArchimedesConfig) -> *mut ArchimedesApp {
    if config.is_null() {
        crate::set_last_error(FfiError::NullPointer("config"));
        return std::ptr::null_mut();
    }

    let config_ref = &*config;

    let internal_config = match InternalConfig::try_from(config_ref) {
        Ok(c) => c,
        Err(e) => {
            crate::set_last_error(FfiError::InvalidConfig(e.to_string()));
            return std::ptr::null_mut();
        }
    };

    let state = Box::new(AppState::new(internal_config));
    Box::into_raw(state) as *mut ArchimedesApp
}

/// Free an Archimedes application
///
/// # Safety
///
/// - `app` must be a valid pointer returned by `archimedes_new`
/// - After calling this, `app` is no longer valid
#[no_mangle]
pub unsafe extern "C" fn archimedes_free(app: *mut ArchimedesApp) {
    if app.is_null() {
        return;
    }

    let _ = Box::from_raw(app as *mut AppState);
}

/// Register a handler for an operation
///
/// # Safety
///
/// - `app` must be a valid application pointer
/// - `operation_id` must be a valid null-terminated UTF-8 string
/// - `handler` must be a valid function pointer
/// - `user_data` is optional and passed to the handler on each call
///
/// Returns 0 on success, or an error code on failure.
#[no_mangle]
pub unsafe extern "C" fn archimedes_register_handler(
    app: *mut ArchimedesApp,
    operation_id: *const c_char,
    handler: ArchimedesHandlerFn,
    user_data: *mut std::ffi::c_void,
) -> ArchimedesError {
    if app.is_null() {
        crate::set_last_error(FfiError::NullPointer("app"));
        return ArchimedesError::NullPointer;
    }

    if operation_id.is_null() {
        crate::set_last_error(FfiError::NullPointer("operation_id"));
        return ArchimedesError::NullPointer;
    }

    let state = &mut *(app as *mut AppState);

    let op_id = match CStr::from_ptr(operation_id).to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            crate::set_last_error(FfiError::InvalidUtf8(e.to_string()));
            return ArchimedesError::InvalidUtf8;
        }
    };

    match state.handlers.register(&op_id, handler, user_data) {
        Ok(()) => ArchimedesError::Ok,
        Err(e) => {
            crate::set_last_error(FfiError::HandlerRegistration(e));
            ArchimedesError::HandlerRegistrationError
        }
    }
}

/// Load a contract from JSON
///
/// # Safety
///
/// - `app` must be a valid application pointer
/// - `json` must be a valid null-terminated UTF-8 JSON string
///
/// Returns 0 on success, or an error code on failure.
#[no_mangle]
pub unsafe extern "C" fn archimedes_load_contract(
    app: *mut ArchimedesApp,
    json: *const c_char,
) -> ArchimedesError {
    if app.is_null() {
        crate::set_last_error(FfiError::NullPointer("app"));
        return ArchimedesError::NullPointer;
    }

    if json.is_null() {
        crate::set_last_error(FfiError::NullPointer("json"));
        return ArchimedesError::NullPointer;
    }

    let state = &mut *(app as *mut AppState);

    let json_str = match CStr::from_ptr(json).to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            crate::set_last_error(FfiError::InvalidUtf8(e.to_string()));
            return ArchimedesError::InvalidUtf8;
        }
    };

    // TODO: Parse contract JSON and validate
    // For now, just store it
    state.contract_json = Some(json_str);

    ArchimedesError::Ok
}

/// Start the Archimedes server
///
/// This function blocks until the server is stopped.
///
/// # Safety
///
/// - `app` must be a valid application pointer
///
/// Returns 0 on success, or an error code on failure.
#[no_mangle]
pub unsafe extern "C" fn archimedes_run(app: *mut ArchimedesApp) -> ArchimedesError {
    if app.is_null() {
        crate::set_last_error(FfiError::NullPointer("app"));
        return ArchimedesError::NullPointer;
    }

    let state = &*(app as *const AppState);

    if state.is_running() {
        crate::set_last_error(FfiError::Internal("Server is already running".to_string()));
        return ArchimedesError::Internal;
    }

    state.set_running(true);

    // TODO: Actually start the server
    // This will integrate with archimedes-server once FFI layer is complete
    // For now, we'll just set up the runtime and return

    let result = crate::runtime::block_on(async {
        // Placeholder for actual server startup
        // let server = Server::new(state.config.clone(), state.handlers.clone());
        // server.run().await

        // For now, just signal we're ready
        tracing::info!(
            "Archimedes FFI server would start on {}:{}",
            state.config.listen_addr,
            state.config.listen_port
        );

        Ok::<(), FfiError>(())
    });

    state.set_running(false);

    match result {
        Ok(()) => ArchimedesError::Ok,
        Err(e) => {
            crate::set_last_error(e);
            ArchimedesError::Internal
        }
    }
}

/// Stop the Archimedes server
///
/// # Safety
///
/// - `app` must be a valid application pointer
///
/// Returns 0 on success, or an error code on failure.
#[no_mangle]
pub unsafe extern "C" fn archimedes_stop(app: *mut ArchimedesApp) -> ArchimedesError {
    if app.is_null() {
        crate::set_last_error(FfiError::NullPointer("app"));
        return ArchimedesError::NullPointer;
    }

    let state = &*(app as *const AppState);

    if !state.is_running() {
        return ArchimedesError::Ok; // Already stopped
    }

    state.set_running(false);
    ArchimedesError::Ok
}

/// Get the application version
///
/// Returns a pointer to a null-terminated string containing the version.
/// The string is statically allocated and should not be freed.
#[no_mangle]
pub extern "C" fn archimedes_version() -> *const c_char {
    static VERSION: std::sync::OnceLock<CString> = std::sync::OnceLock::new();

    VERSION
        .get_or_init(|| CString::new(env!("CARGO_PKG_VERSION")).unwrap())
        .as_ptr()
}

/// Check if the application is currently running
///
/// # Safety
///
/// - `app` must be a valid application pointer
///
/// Returns 1 if running, 0 if not running or on error.
#[no_mangle]
pub unsafe extern "C" fn archimedes_is_running(app: *const ArchimedesApp) -> i32 {
    if app.is_null() {
        return 0;
    }

    let state = &*(app as *const AppState);
    if state.is_running() {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ArchimedesRequestContext;

    fn create_test_config() -> (ArchimedesConfig, CString) {
        let contract_path = CString::new("contract.json").unwrap();

        let config = ArchimedesConfig {
            contract_path: contract_path.as_ptr(),
            ..Default::default()
        };

        (config, contract_path)
    }

    extern "C" fn test_handler(
        _ctx: *const ArchimedesRequestContext,
        _body: *const u8,
        _body_len: usize,
        _user_data: *mut std::ffi::c_void,
    ) -> crate::types::ArchimedesResponseData {
        crate::types::ArchimedesResponseData {
            status_code: 200,
            body: std::ptr::null(),
            body_len: 0,
            body_owned: false,
            content_type: std::ptr::null(),
            headers_count: 0,
            header_names: std::ptr::null(),
            header_values: std::ptr::null(),
        }
    }

    #[test]
    fn test_create_and_free_app() {
        let (config, _contract_path) = create_test_config();

        unsafe {
            let app = archimedes_new(&config);
            assert!(!app.is_null());

            archimedes_free(app);
        }
    }

    #[test]
    fn test_null_config() {
        unsafe {
            let app = archimedes_new(std::ptr::null());
            assert!(app.is_null());
        }
    }

    #[test]
    fn test_register_handler() {
        let (config, _contract_path) = create_test_config();
        let op_id = CString::new("getUser").unwrap();

        unsafe {
            let app = archimedes_new(&config);
            assert!(!app.is_null());

            let result =
                archimedes_register_handler(app, op_id.as_ptr(), test_handler, std::ptr::null_mut());
            assert_eq!(result, ArchimedesError::Ok);

            archimedes_free(app);
        }
    }

    #[test]
    fn test_register_handler_null_app() {
        let op_id = CString::new("getUser").unwrap();

        unsafe {
            let result = archimedes_register_handler(
                std::ptr::null_mut(),
                op_id.as_ptr(),
                test_handler,
                std::ptr::null_mut(),
            );
            assert_eq!(result, ArchimedesError::NullPointer);
        }
    }

    #[test]
    fn test_register_handler_null_operation_id() {
        let (config, _contract_path) = create_test_config();

        unsafe {
            let app = archimedes_new(&config);
            assert!(!app.is_null());

            let result = archimedes_register_handler(
                app,
                std::ptr::null(),
                test_handler,
                std::ptr::null_mut(),
            );
            assert_eq!(result, ArchimedesError::NullPointer);

            archimedes_free(app);
        }
    }

    #[test]
    fn test_load_contract() {
        let (config, _contract_path) = create_test_config();
        let contract = CString::new(r#"{"operations": []}"#).unwrap();

        unsafe {
            let app = archimedes_new(&config);
            assert!(!app.is_null());

            let result = archimedes_load_contract(app, contract.as_ptr());
            assert_eq!(result, ArchimedesError::Ok);

            archimedes_free(app);
        }
    }

    #[test]
    fn test_is_running_initially_false() {
        let (config, _contract_path) = create_test_config();

        unsafe {
            let app = archimedes_new(&config);
            assert!(!app.is_null());

            assert_eq!(archimedes_is_running(app), 0);

            archimedes_free(app);
        }
    }

    #[test]
    fn test_version() {
        let version = archimedes_version();
        assert!(!version.is_null());

        unsafe {
            let version_str = CStr::from_ptr(version).to_str().unwrap();
            assert!(!version_str.is_empty());
        }
    }

    #[test]
    fn test_stop_not_running() {
        let (config, _contract_path) = create_test_config();

        unsafe {
            let app = archimedes_new(&config);
            assert!(!app.is_null());

            // Stopping when not running should succeed
            let result = archimedes_stop(app);
            assert_eq!(result, ArchimedesError::Ok);

            archimedes_free(app);
        }
    }
}
