// Package archimedes provides Go bindings for the Archimedes HTTP server framework.
//
// Archimedes is a contract-first HTTP server with built-in middleware for:
//   - Request ID generation and propagation
//   - OpenTelemetry tracing
//   - Caller identity extraction (SPIFFE, JWT, API Key)
//   - OPA/Eunomia authorization
//   - Request/response validation against Themis contracts
//
// Example usage:
//
//	package main
//
//	import "github.com/themis-platform/archimedes-go"
//
//	func main() {
//	    app := archimedes.New(archimedes.Config{
//	        Contract: "contract.json",
//	    })
//
//	    app.Operation("listUsers", func(ctx *archimedes.Context) error {
//	        users, _ := db.GetUsers()
//	        return ctx.JSON(200, map[string]any{"users": users})
//	    })
//
//	    app.Run(":8080")
//	}
package archimedes

/*
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -larchimedes_ffi
#cgo CFLAGS: -I${SRCDIR}/../../target/include

#include <archimedes.h>
#include <stdlib.h>
#include <string.h>

// Handler callback wrapper - declared here, implemented in Go
extern struct archimedes_response_data go_handler_callback(
    const struct archimedes_request_context* ctx,
    const uint8_t* body,
    size_t body_len,
    void* user_data
);
*/
import "C"
import (
	"encoding/json"
	"errors"
	"fmt"
	"runtime"
	"sync"
	"unsafe"
)

// =============================================================================
// Error Types
// =============================================================================

// Error codes matching archimedes_error enum
const (
	ErrOK                 = 0
	ErrInvalidConfig      = 1
	ErrContractLoadError  = 2
	ErrPolicyLoadError    = 3
	ErrHandlerRegistration = 4
	ErrServerStartError   = 5
	ErrInvalidOperation   = 6
	ErrHandlerError       = 7
	ErrValidationError    = 8
	ErrAuthorizationError = 9
	ErrNullPointer        = 10
	ErrInvalidUTF8        = 11
	ErrInternal           = 99
)

// Error represents an Archimedes error
type Error struct {
	Code    int
	Message string
}

func (e *Error) Error() string {
	return fmt.Sprintf("archimedes error %d: %s", e.Code, e.Message)
}

// =============================================================================
// Configuration
// =============================================================================

// Config holds Archimedes application configuration
type Config struct {
	// Contract is the path to the Themis contract JSON file (required)
	Contract string

	// PolicyBundle is the path to OPA policy bundle (optional)
	PolicyBundle string

	// ListenAddr is the address to listen on (default: "0.0.0.0")
	ListenAddr string

	// Port is the port to listen on (default: 8080)
	Port uint16

	// MetricsPort is the port for Prometheus metrics (default: 9090, 0 to disable)
	MetricsPort uint16

	// EnableValidation enables request validation (default: true)
	EnableValidation bool

	// EnableResponseValidation enables response validation (default: false)
	EnableResponseValidation bool

	// EnableAuthorization enables OPA authorization (default: true if PolicyBundle set)
	EnableAuthorization bool

	// EnableTracing enables OpenTelemetry tracing (default: true)
	EnableTracing bool

	// OTLPEndpoint is the OTLP endpoint for traces (optional)
	OTLPEndpoint string

	// ServiceName is the service name for telemetry (default: "archimedes-service")
	ServiceName string

	// ShutdownTimeout is graceful shutdown timeout in seconds (default: 30)
	ShutdownTimeout uint32

	// MaxBodySize is maximum request body size in bytes (default: 1MB)
	MaxBodySize uint64

	// RequestTimeout is request timeout in seconds (default: 30, 0 for no timeout)
	RequestTimeout uint32
}

// =============================================================================
// Caller Identity
// =============================================================================

// CallerIdentity represents the authenticated caller
type CallerIdentity struct {
	Type        string   `json:"type"`
	ID          string   `json:"id,omitempty"`
	TrustDomain string   `json:"trust_domain,omitempty"`
	Path        string   `json:"path,omitempty"`
	UserID      string   `json:"user_id,omitempty"`
	Roles       []string `json:"roles,omitempty"`
	KeyID       string   `json:"key_id,omitempty"`
}

// IsSpiffe returns true if this is a SPIFFE identity
func (c *CallerIdentity) IsSpiffe() bool {
	return c.Type == "spiffe"
}

// IsUser returns true if this is a user identity
func (c *CallerIdentity) IsUser() bool {
	return c.Type == "user"
}

// IsAPIKey returns true if this is an API key identity
func (c *CallerIdentity) IsAPIKey() bool {
	return c.Type == "api_key"
}

// IsAnonymous returns true if this is an anonymous identity
func (c *CallerIdentity) IsAnonymous() bool {
	return c.Type == "anonymous"
}

// =============================================================================
// Context
// =============================================================================

// Context provides request context and response methods to handlers
type Context struct {
	// RequestID is the unique request identifier (UUID v7)
	RequestID string

	// TraceID is the OpenTelemetry trace ID
	TraceID string

	// SpanID is the OpenTelemetry span ID
	SpanID string

	// OperationID is the matched operation from contract
	OperationID string

	// Method is the HTTP method
	Method string

	// Path is the request path
	Path string

	// Query is the query string (without leading ?)
	Query string

	// PathParams contains path parameters
	PathParams map[string]string

	// Headers contains request headers
	Headers map[string]string

	// Caller is the authenticated caller identity (may be nil for anonymous)
	Caller *CallerIdentity

	// body is the raw request body
	body []byte

	// response fields
	responseStatus  int
	responseBody    []byte
	responseHeaders map[string]string
	contentType     string
}

// Body returns the raw request body
func (c *Context) Body() []byte {
	return c.body
}

// BodyString returns the request body as a string
func (c *Context) BodyString() string {
	return string(c.body)
}

// Bind unmarshals the JSON body into the given struct
func (c *Context) Bind(v any) error {
	if len(c.body) == 0 {
		return errors.New("empty request body")
	}
	return json.Unmarshal(c.body, v)
}

// PathParam returns a path parameter by name
func (c *Context) PathParam(name string) string {
	return c.PathParams[name]
}

// Header returns a request header by name
func (c *Context) Header(name string) string {
	return c.Headers[name]
}

// JSON sends a JSON response
func (c *Context) JSON(status int, v any) error {
	data, err := json.Marshal(v)
	if err != nil {
		return err
	}
	c.responseStatus = status
	c.responseBody = data
	c.contentType = "application/json"
	return nil
}

// String sends a string response
func (c *Context) String(status int, s string) error {
	c.responseStatus = status
	c.responseBody = []byte(s)
	c.contentType = "text/plain"
	return nil
}

// Blob sends a binary response
func (c *Context) Blob(status int, contentType string, data []byte) error {
	c.responseStatus = status
	c.responseBody = data
	c.contentType = contentType
	return nil
}

// NoContent sends a 204 No Content response
func (c *Context) NoContent() error {
	c.responseStatus = 204
	c.responseBody = nil
	return nil
}

// SetHeader sets a response header
func (c *Context) SetHeader(name, value string) {
	if c.responseHeaders == nil {
		c.responseHeaders = make(map[string]string)
	}
	c.responseHeaders[name] = value
}

// =============================================================================
// Handler
// =============================================================================

// Handler is the function signature for operation handlers
type Handler func(ctx *Context) error

// =============================================================================
// Application
// =============================================================================

// App represents an Archimedes application instance
type App struct {
	handle   *C.struct_archimedes_app
	config   Config
	handlers map[string]Handler
	mu       sync.RWMutex
}

// Handler registry for callbacks
var (
	handlerRegistry   = make(map[uintptr]Handler)
	handlerRegistryMu sync.RWMutex
	nextHandlerID     uintptr
)

// New creates a new Archimedes application
func New(cfg Config) (*App, error) {
	// Set defaults
	if cfg.Port == 0 {
		cfg.Port = 8080
	}
	if cfg.MetricsPort == 0 {
		cfg.MetricsPort = 9090
	}
	if cfg.ServiceName == "" {
		cfg.ServiceName = "archimedes-service"
	}
	if cfg.ShutdownTimeout == 0 {
		cfg.ShutdownTimeout = 30
	}
	if cfg.MaxBodySize == 0 {
		cfg.MaxBodySize = 1024 * 1024 // 1MB
	}
	if cfg.RequestTimeout == 0 {
		cfg.RequestTimeout = 30
	}

	// Convert to C config
	cConfig := C.struct_archimedes_config{
		listen_port:                C.uint16_t(cfg.Port),
		metrics_port:               C.uint16_t(cfg.MetricsPort),
		enable_validation:          C.bool(cfg.EnableValidation),
		enable_response_validation: C.bool(cfg.EnableResponseValidation),
		enable_authorization:       C.bool(cfg.EnableAuthorization),
		enable_tracing:             C.bool(cfg.EnableTracing),
		shutdown_timeout_secs:      C.uint32_t(cfg.ShutdownTimeout),
		max_body_size:              C.size_t(cfg.MaxBodySize),
		request_timeout_secs:       C.uint32_t(cfg.RequestTimeout),
	}

	// Set string fields
	if cfg.Contract != "" {
		cContract := C.CString(cfg.Contract)
		defer C.free(unsafe.Pointer(cContract))
		cConfig.contract_path = cContract
	}
	if cfg.PolicyBundle != "" {
		cBundle := C.CString(cfg.PolicyBundle)
		defer C.free(unsafe.Pointer(cBundle))
		cConfig.policy_bundle_path = cBundle
	}
	if cfg.ListenAddr != "" {
		cAddr := C.CString(cfg.ListenAddr)
		defer C.free(unsafe.Pointer(cAddr))
		cConfig.listen_addr = cAddr
	}
	if cfg.OTLPEndpoint != "" {
		cEndpoint := C.CString(cfg.OTLPEndpoint)
		defer C.free(unsafe.Pointer(cEndpoint))
		cConfig.otlp_endpoint = cEndpoint
	}
	if cfg.ServiceName != "" {
		cName := C.CString(cfg.ServiceName)
		defer C.free(unsafe.Pointer(cName))
		cConfig.service_name = cName
	}

	// Create application
	handle := C.archimedes_new(&cConfig)
	if handle == nil {
		errMsg := C.GoString(C.archimedes_last_error())
		return nil, &Error{Code: ErrInvalidConfig, Message: errMsg}
	}

	app := &App{
		handle:   handle,
		config:   cfg,
		handlers: make(map[string]Handler),
	}

	// Prevent GC of app while handle is alive
	runtime.SetFinalizer(app, func(a *App) {
		a.Close()
	})

	return app, nil
}

// Operation registers a handler for an operation
func (a *App) Operation(operationID string, handler Handler) error {
	a.mu.Lock()
	defer a.mu.Unlock()

	// Store handler
	a.handlers[operationID] = handler

	// Register in global registry for callbacks
	handlerRegistryMu.Lock()
	id := nextHandlerID
	nextHandlerID++
	handlerRegistry[id] = handler
	handlerRegistryMu.Unlock()

	// Register with C API
	cOpID := C.CString(operationID)
	defer C.free(unsafe.Pointer(cOpID))

	err := C.archimedes_register_handler(
		a.handle,
		cOpID,
		(C.archimedes_handler_fn)(C.go_handler_callback),
		unsafe.Pointer(id),
	)

	if err != C.ARCHIMEDES_ERROR_OK {
		errMsg := C.GoString(C.archimedes_last_error())
		return &Error{Code: int(err), Message: errMsg}
	}

	return nil
}

// Run starts the server and blocks until shutdown
func (a *App) Run(addr string) error {
	// Parse port from addr if provided (e.g., ":8080")
	// For now, use configured port
	err := C.archimedes_run(a.handle)
	if err != C.ARCHIMEDES_ERROR_OK {
		errMsg := C.GoString(C.archimedes_last_error())
		return &Error{Code: int(err), Message: errMsg}
	}
	return nil
}

// Stop gracefully stops the server
func (a *App) Stop() error {
	err := C.archimedes_stop(a.handle)
	if err != C.ARCHIMEDES_ERROR_OK {
		errMsg := C.GoString(C.archimedes_last_error())
		return &Error{Code: int(err), Message: errMsg}
	}
	return nil
}

// IsRunning returns true if the server is running
func (a *App) IsRunning() bool {
	return C.archimedes_is_running(a.handle) != 0
}

// Close frees the application resources
func (a *App) Close() {
	if a.handle != nil {
		C.archimedes_free(a.handle)
		a.handle = nil
	}
}

// Version returns the Archimedes version string
func Version() string {
	return C.GoString(C.archimedes_version())
}

// =============================================================================
// CGO Callback Implementation
// =============================================================================

//export go_handler_callback
func go_handler_callback(
	ctx *C.struct_archimedes_request_context,
	body *C.uint8_t,
	bodyLen C.size_t,
	userData unsafe.Pointer,
) C.struct_archimedes_response_data {
	// Get handler from registry
	handlerID := uintptr(userData)
	handlerRegistryMu.RLock()
	handler, ok := handlerRegistry[handlerID]
	handlerRegistryMu.RUnlock()

	// Default error response
	var response C.struct_archimedes_response_data
	response.status_code = 500

	if !ok {
		errBody := `{"error":"Handler not found"}`
		response.body = C.CString(errBody)
		response.body_len = C.size_t(len(errBody))
		response.body_owned = true
		return response
	}

	// Build Go context
	goCtx := &Context{
		RequestID:       C.GoString(ctx.request_id),
		TraceID:         C.GoString(ctx.trace_id),
		SpanID:          C.GoString(ctx.span_id),
		OperationID:     C.GoString(ctx.operation_id),
		Method:          C.GoString(ctx.method),
		Path:            C.GoString(ctx.path),
		Query:           C.GoString(ctx.query),
		PathParams:      make(map[string]string),
		Headers:         make(map[string]string),
		responseStatus:  200,
		responseHeaders: make(map[string]string),
	}

	// Copy body
	if bodyLen > 0 {
		goCtx.body = C.GoBytes(unsafe.Pointer(body), C.int(bodyLen))
	}

	// Copy path params
	for i := C.size_t(0); i < ctx.path_params_count; i++ {
		name := C.GoString(*(**C.char)(unsafe.Pointer(uintptr(unsafe.Pointer(ctx.path_param_names)) + uintptr(i)*unsafe.Sizeof(uintptr(0)))))
		value := C.GoString(*(**C.char)(unsafe.Pointer(uintptr(unsafe.Pointer(ctx.path_param_values)) + uintptr(i)*unsafe.Sizeof(uintptr(0)))))
		goCtx.PathParams[name] = value
	}

	// Copy headers
	for i := C.size_t(0); i < ctx.headers_count; i++ {
		name := C.GoString(*(**C.char)(unsafe.Pointer(uintptr(unsafe.Pointer(ctx.header_names)) + uintptr(i)*unsafe.Sizeof(uintptr(0)))))
		value := C.GoString(*(**C.char)(unsafe.Pointer(uintptr(unsafe.Pointer(ctx.header_values)) + uintptr(i)*unsafe.Sizeof(uintptr(0)))))
		goCtx.Headers[name] = value
	}

	// Parse caller identity
	if ctx.caller_identity_json != nil {
		identityJSON := C.GoString(ctx.caller_identity_json)
		if identityJSON != "" {
			var caller CallerIdentity
			if err := json.Unmarshal([]byte(identityJSON), &caller); err == nil {
				goCtx.Caller = &caller
			}
		}
	}

	// Call handler
	err := handler(goCtx)
	if err != nil {
		errBody := fmt.Sprintf(`{"error":"%s"}`, err.Error())
		response.status_code = 500
		response.body = C.CString(errBody)
		response.body_len = C.size_t(len(errBody))
		response.body_owned = true
		return response
	}

	// Build response
	response.status_code = C.int32_t(goCtx.responseStatus)
	if len(goCtx.responseBody) > 0 {
		response.body = C.CString(string(goCtx.responseBody))
		response.body_len = C.size_t(len(goCtx.responseBody))
		response.body_owned = true
	}
	if goCtx.contentType != "" {
		response.content_type = C.CString(goCtx.contentType)
	}

	return response
}
