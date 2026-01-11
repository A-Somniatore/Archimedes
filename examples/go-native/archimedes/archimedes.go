// Package archimedes provides Go bindings for the Archimedes HTTP server framework.
//
// Archimedes is a contract-first HTTP server with built-in middleware for:
//   - Request ID generation and propagation
//   - OpenTelemetry tracing
//   - Caller identity extraction (SPIFFE, JWT, API Key)
//   - OPA/Eunomia authorization
//   - Request/response validation against Themis contracts
//   - Sub-routers with prefix and tag support
//   - Lifecycle hooks for startup/shutdown
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
//	    // Lifecycle hooks
//	    app.OnStartup("db_init", func() error {
//	        return db.Connect()
//	    })
//	    app.OnShutdown("db_close", func() error {
//	        return db.Close()
//	    })
//
//	    // Sub-router
//	    usersRouter := archimedes.NewRouter().Prefix("/users").Tag("users")
//	    usersRouter.Operation("listUsers", listUsersHandler)
//	    app.Merge(usersRouter)
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
	"strings"
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
	handle    *C.struct_archimedes_app
	config    Config
	handlers  map[string]Handler
	lifecycle *Lifecycle
	mu        sync.RWMutex
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
		handle:    handle,
		config:    cfg,
		handlers:  make(map[string]Handler),
		lifecycle: NewLifecycle(),
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
// Router
// =============================================================================

// Router is a sub-router for grouping operations with shared configuration
type Router struct {
	prefix     string
	tags       []string
	operations map[string]Handler
}

// NewRouter creates a new router
func NewRouter() *Router {
	return &Router{
		tags:       []string{},
		operations: make(map[string]Handler),
	}
}

// Prefix sets the path prefix for all operations in this router
func (r *Router) Prefix(prefix string) *Router {
	// Normalize prefix
	if prefix != "" && prefix[0] != '/' {
		prefix = "/" + prefix
	}
	if len(prefix) > 1 && prefix[len(prefix)-1] == '/' {
		prefix = prefix[:len(prefix)-1]
	}
	r.prefix = prefix
	return r
}

// Tag adds a tag to this router for grouping
func (r *Router) Tag(tag string) *Router {
	// Don't add duplicates
	for _, t := range r.tags {
		if t == tag {
			return r
		}
	}
	r.tags = append(r.tags, tag)
	return r
}

// Operation registers a handler for an operation on this router
func (r *Router) Operation(operationID string, handler Handler) *Router {
	r.operations[operationID] = handler
	return r
}

// GetPrefix returns the current prefix
func (r *Router) GetPrefix() string {
	return r.prefix
}

// GetTags returns all tags
func (r *Router) GetTags() []string {
	return r.tags
}

// GetOperations returns all registered operations
func (r *Router) GetOperations() map[string]Handler {
	return r.operations
}

// Nest adds a child router under this router
func (r *Router) Nest(child *Router) *Router {
	// Copy operations from child with combined prefix
	for opID, handler := range child.operations {
		r.operations[opID] = handler
	}
	return r
}

// Merge copies all operations from another router
func (r *Router) Merge(other *Router) *Router {
	for opID, handler := range other.operations {
		r.operations[opID] = handler
	}
	return r
}

// Merge merges a router's operations into this app
func (a *App) Merge(router *Router) error {
	for opID, handler := range router.GetOperations() {
		if err := a.Operation(opID, handler); err != nil {
			return err
		}
	}
	return nil
}

// Nest nests a router under a prefix in this app
func (a *App) Nest(prefix string, router *Router) error {
	// Set prefix on router if not already set
	if router.GetPrefix() == "" {
		router.Prefix(prefix)
	}
	return a.Merge(router)
}

// =============================================================================
// Form Data Extractor
// =============================================================================

// Form parses URL-encoded form data from the request body
type Form map[string]string

// ParseForm parses the request body as URL-encoded form data
func (c *Context) ParseForm() (Form, error) {
	if len(c.body) == 0 {
		return Form{}, nil
	}

	form := make(Form)
	pairs := string(c.body)

	for _, pair := range splitString(pairs, '&') {
		if pair == "" {
			continue
		}
		kv := splitString(pair, '=')
		if len(kv) >= 1 {
			key := urlDecode(kv[0])
			value := ""
			if len(kv) >= 2 {
				value = urlDecode(kv[1])
			}
			form[key] = value
		}
	}

	return form, nil
}

// Get returns a form field value by name
func (f Form) Get(name string) string {
	return f[name]
}

// GetOr returns a form field value or a default if not present
func (f Form) GetOr(name, defaultValue string) string {
	if val, ok := f[name]; ok {
		return val
	}
	return defaultValue
}

// Has returns true if the form has a field with the given name
func (f Form) Has(name string) bool {
	_, ok := f[name]
	return ok
}

// =============================================================================
// Cookie Extractor
// =============================================================================

// Cookies parses cookies from the Cookie header
type Cookies map[string]string

// ParseCookies parses the Cookie header into a map
func (c *Context) ParseCookies() Cookies {
	cookies := make(Cookies)
	cookieHeader := c.Headers["Cookie"]
	if cookieHeader == "" {
		cookieHeader = c.Headers["cookie"]
	}
	if cookieHeader == "" {
		return cookies
	}

	for _, part := range splitString(cookieHeader, ';') {
		part = trimSpace(part)
		if part == "" {
			continue
		}
		kv := splitString(part, '=')
		if len(kv) >= 2 {
			cookies[trimSpace(kv[0])] = trimSpace(kv[1])
		}
	}

	return cookies
}

// Get returns a cookie value by name
func (c Cookies) Get(name string) string {
	return c[name]
}

// GetOr returns a cookie value or a default if not present
func (c Cookies) GetOr(name, defaultValue string) string {
	if val, ok := c[name]; ok {
		return val
	}
	return defaultValue
}

// Has returns true if the cookie exists
func (c Cookies) Has(name string) bool {
	_, ok := c[name]
	return ok
}

// =============================================================================
// Set-Cookie Builder
// =============================================================================

// SameSite represents the SameSite cookie attribute
type SameSite string

const (
	SameSiteNone   SameSite = "None"
	SameSiteLax    SameSite = "Lax"
	SameSiteStrict SameSite = "Strict"
)

// SetCookie builds Set-Cookie header values
type SetCookie struct {
	name     string
	value    string
	path     string
	domain   string
	expires  string
	maxAge   int
	secure   bool
	httpOnly bool
	sameSite SameSite
	hasMaxAge bool
}

// NewSetCookie creates a new Set-Cookie builder
func NewSetCookie(name, value string) *SetCookie {
	return &SetCookie{
		name:     name,
		value:    value,
		sameSite: SameSiteLax,
	}
}

// Path sets the Path attribute
func (s *SetCookie) Path(path string) *SetCookie {
	s.path = path
	return s
}

// Domain sets the Domain attribute
func (s *SetCookie) Domain(domain string) *SetCookie {
	s.domain = domain
	return s
}

// Expires sets the Expires attribute (RFC 7231 format)
func (s *SetCookie) Expires(expires string) *SetCookie {
	s.expires = expires
	return s
}

// MaxAge sets the Max-Age attribute in seconds
func (s *SetCookie) MaxAge(seconds int) *SetCookie {
	s.maxAge = seconds
	s.hasMaxAge = true
	return s
}

// Secure sets the Secure attribute
func (s *SetCookie) Secure(secure bool) *SetCookie {
	s.secure = secure
	return s
}

// HttpOnly sets the HttpOnly attribute
func (s *SetCookie) HttpOnly(httpOnly bool) *SetCookie {
	s.httpOnly = httpOnly
	return s
}

// SetSameSite sets the SameSite attribute
func (s *SetCookie) SetSameSite(sameSite SameSite) *SetCookie {
	s.sameSite = sameSite
	return s
}

// Build returns the Set-Cookie header value
func (s *SetCookie) Build() string {
	result := s.name + "=" + s.value

	if s.path != "" {
		result += "; Path=" + s.path
	}
	if s.domain != "" {
		result += "; Domain=" + s.domain
	}
	if s.expires != "" {
		result += "; Expires=" + s.expires
	}
	if s.hasMaxAge {
		result += fmt.Sprintf("; Max-Age=%d", s.maxAge)
	}
	if s.secure {
		result += "; Secure"
	}
	if s.httpOnly {
		result += "; HttpOnly"
	}
	result += "; SameSite=" + string(s.sameSite)

	return result
}

// SetCookie sets a Set-Cookie response header
func (c *Context) SetCookie(cookie *SetCookie) {
	c.SetHeader("Set-Cookie", cookie.Build())
}

// =============================================================================
// Multipart Form Data
// =============================================================================

// MultipartField represents a field in multipart form data
type MultipartField struct {
	Name        string
	Value       string
	Filename    string
	ContentType string
	Data        []byte
	IsFile      bool
}

// Multipart represents parsed multipart form data
type Multipart struct {
	Fields []MultipartField
}

// ParseMultipart parses multipart/form-data from the request body
func (c *Context) ParseMultipart() (*Multipart, error) {
	contentType := c.Headers["Content-Type"]
	if contentType == "" {
		contentType = c.Headers["content-type"]
	}

	if contentType == "" {
		return nil, errors.New("missing Content-Type header")
	}

	// Extract boundary
	boundary := ""
	for _, part := range splitString(contentType, ';') {
		part = trimSpace(part)
		if hasPrefix(part, "boundary=") {
			boundary = part[9:]
			// Remove quotes if present
			if len(boundary) >= 2 && boundary[0] == '"' && boundary[len(boundary)-1] == '"' {
				boundary = boundary[1 : len(boundary)-1]
			}
			break
		}
	}

	if boundary == "" {
		return nil, errors.New("missing multipart boundary")
	}

	multipart := &Multipart{Fields: []MultipartField{}}
	delimiter := "--" + boundary
	bodyStr := string(c.body)

	parts := splitString(bodyStr, '\n')
	inPart := false
	var currentField *MultipartField
	var contentBuffer string
	inHeaders := false

	for _, line := range parts {
		line = trimSuffix(line, "\r")

		if hasPrefix(line, delimiter) {
			// End previous part if any
			if currentField != nil && inPart {
				// Trim trailing CRLF from content
				content := trimSuffix(contentBuffer, "\r\n")
				content = trimSuffix(content, "\n")
				if currentField.IsFile {
					currentField.Data = []byte(content)
				} else {
					currentField.Value = content
				}
				multipart.Fields = append(multipart.Fields, *currentField)
			}

			if hasSuffix(line, "--") {
				// End of multipart
				break
			}

			// Start new part
			currentField = &MultipartField{}
			contentBuffer = ""
			inPart = true
			inHeaders = true
			continue
		}

		if inPart {
			if inHeaders {
				if line == "" {
					// End of headers, start of content
					inHeaders = false
					continue
				}

				// Parse headers
				lowerLine := toLower(line)
				if hasPrefix(lowerLine, "content-disposition:") {
					// Parse name and filename
					if name := extractHeaderParam(line, "name"); name != "" {
						currentField.Name = name
					}
					if filename := extractHeaderParam(line, "filename"); filename != "" {
						currentField.Filename = filename
						currentField.IsFile = true
					}
				} else if hasPrefix(lowerLine, "content-type:") {
					currentField.ContentType = trimSpace(line[13:])
				}
			} else {
				// Content
				if contentBuffer != "" {
					contentBuffer += "\n"
				}
				contentBuffer += line
			}
		}
	}

	return multipart, nil
}

// Get returns a field by name
func (m *Multipart) Get(name string) *MultipartField {
	for i := range m.Fields {
		if m.Fields[i].Name == name {
			return &m.Fields[i]
		}
	}
	return nil
}

// GetFile returns a file field by name
func (m *Multipart) GetFile(name string) *MultipartField {
	for i := range m.Fields {
		if m.Fields[i].Name == name && m.Fields[i].IsFile {
			return &m.Fields[i]
		}
	}
	return nil
}

// GetValue returns a text field value by name
func (m *Multipart) GetValue(name string) string {
	field := m.Get(name)
	if field != nil && !field.IsFile {
		return field.Value
	}
	return ""
}

// =============================================================================
// File Response
// =============================================================================

// File sends a file as a response with appropriate headers
func (c *Context) File(filename string, data []byte, inline bool) error {
	c.responseStatus = 200
	c.responseBody = data
	c.contentType = guessMimeType(filename)

	disposition := "attachment"
	if inline {
		disposition = "inline"
	}
	c.SetHeader("Content-Disposition", fmt.Sprintf(`%s; filename="%s"`, disposition, filename))

	return nil
}

// Attachment sends a file as a download
func (c *Context) Attachment(filename string, data []byte) error {
	return c.File(filename, data, false)
}

// Inline sends a file for inline display (e.g., in browser)
func (c *Context) Inline(filename string, data []byte) error {
	return c.File(filename, data, true)
}

// =============================================================================
// Redirect Responses
// =============================================================================

// Redirect sends a redirect response with the given status code
func (c *Context) Redirect(status int, location string) error {
	c.responseStatus = status
	c.responseBody = nil
	c.SetHeader("Location", location)
	return nil
}

// RedirectFound sends a 302 Found redirect
func (c *Context) RedirectFound(location string) error {
	return c.Redirect(302, location)
}

// RedirectPermanent sends a 301 Moved Permanently redirect
func (c *Context) RedirectPermanent(location string) error {
	return c.Redirect(301, location)
}

// RedirectSeeOther sends a 303 See Other redirect
func (c *Context) RedirectSeeOther(location string) error {
	return c.Redirect(303, location)
}

// RedirectTemporary sends a 307 Temporary Redirect
func (c *Context) RedirectTemporary(location string) error {
	return c.Redirect(307, location)
}

// =============================================================================
// Helper Functions
// =============================================================================

// splitString splits a string by a separator (avoids importing strings)
func splitString(s string, sep byte) []string {
	var result []string
	start := 0
	for i := 0; i < len(s); i++ {
		if s[i] == sep {
			result = append(result, s[start:i])
			start = i + 1
		}
	}
	result = append(result, s[start:])
	return result
}

// trimSpace trims leading and trailing whitespace
func trimSpace(s string) string {
	start := 0
	end := len(s)
	for start < end && (s[start] == ' ' || s[start] == '\t' || s[start] == '\r' || s[start] == '\n') {
		start++
	}
	for end > start && (s[end-1] == ' ' || s[end-1] == '\t' || s[end-1] == '\r' || s[end-1] == '\n') {
		end--
	}
	return s[start:end]
}

// trimSuffix removes a suffix from a string
func trimSuffix(s, suffix string) string {
	if len(s) >= len(suffix) && s[len(s)-len(suffix):] == suffix {
		return s[:len(s)-len(suffix)]
	}
	return s
}

// hasPrefix checks if string has prefix
func hasPrefix(s, prefix string) bool {
	return len(s) >= len(prefix) && s[:len(prefix)] == prefix
}

// hasSuffix checks if string has suffix
func hasSuffix(s, suffix string) bool {
	return len(s) >= len(suffix) && s[len(s)-len(suffix):] == suffix
}

// toLower converts to lowercase
func toLower(s string) string {
	result := make([]byte, len(s))
	for i := 0; i < len(s); i++ {
		c := s[i]
		if c >= 'A' && c <= 'Z' {
			c += 'a' - 'A'
		}
		result[i] = c
	}
	return string(result)
}

// extractHeaderParam extracts a parameter from a header line
func extractHeaderParam(line, param string) string {
	search := param + `="`
	idx := -1
	lineLower := toLower(line)
	searchLower := toLower(search)

	for i := 0; i <= len(lineLower)-len(searchLower); i++ {
		if lineLower[i:i+len(searchLower)] == searchLower {
			idx = i
			break
		}
	}

	if idx >= 0 {
		rest := line[idx+len(search):]
		for i := 0; i < len(rest); i++ {
			if rest[i] == '"' {
				return rest[:i]
			}
		}
	}

	// Try without quotes
	search = param + "="
	searchLower = toLower(search)
	for i := 0; i <= len(lineLower)-len(searchLower); i++ {
		if lineLower[i:i+len(searchLower)] == searchLower {
			idx = i
			break
		}
	}

	if idx >= 0 {
		rest := line[idx+len(search):]
		end := len(rest)
		for i := 0; i < len(rest); i++ {
			if rest[i] == ';' || rest[i] == ' ' {
				end = i
				break
			}
		}
		return trimSpace(rest[:end])
	}

	return ""
}

// urlDecode decodes a URL-encoded string
func urlDecode(s string) string {
	result := make([]byte, 0, len(s))
	for i := 0; i < len(s); i++ {
		c := s[i]
		if c == '+' {
			result = append(result, ' ')
		} else if c == '%' && i+2 < len(s) {
			h1 := hexValue(s[i+1])
			h2 := hexValue(s[i+2])
			if h1 >= 0 && h2 >= 0 {
				result = append(result, byte(h1<<4|h2))
				i += 2
			} else {
				result = append(result, c)
			}
		} else {
			result = append(result, c)
		}
	}
	return string(result)
}

// hexValue returns the value of a hex digit, or -1 if invalid
func hexValue(c byte) int {
	switch {
	case c >= '0' && c <= '9':
		return int(c - '0')
	case c >= 'a' && c <= 'f':
		return int(c - 'a' + 10)
	case c >= 'A' && c <= 'F':
		return int(c - 'A' + 10)
	default:
		return -1
	}
}

// guessMimeType guesses MIME type from filename extension
func guessMimeType(filename string) string {
	ext := ""
	for i := len(filename) - 1; i >= 0; i-- {
		if filename[i] == '.' {
			ext = toLower(filename[i+1:])
			break
		}
	}

	switch ext {
	// Text
	case "html", "htm":
		return "text/html"
	case "css":
		return "text/css"
	case "js", "mjs":
		return "text/javascript"
	case "json":
		return "application/json"
	case "xml":
		return "application/xml"
	case "txt":
		return "text/plain"
	case "csv":
		return "text/csv"
	case "md":
		return "text/markdown"
	case "yaml", "yml":
		return "application/yaml"

	// Images
	case "png":
		return "image/png"
	case "jpg", "jpeg":
		return "image/jpeg"
	case "gif":
		return "image/gif"
	case "svg":
		return "image/svg+xml"
	case "webp":
		return "image/webp"
	case "ico":
		return "image/x-icon"
	case "bmp":
		return "image/bmp"

	// Audio/Video
	case "mp3":
		return "audio/mpeg"
	case "wav":
		return "audio/wav"
	case "mp4":
		return "video/mp4"
	case "webm":
		return "video/webm"

	// Documents
	case "pdf":
		return "application/pdf"
	case "doc":
		return "application/msword"
	case "docx":
		return "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
	case "xls":
		return "application/vnd.ms-excel"
	case "xlsx":
		return "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"

	// Archives
	case "zip":
		return "application/zip"
	case "tar":
		return "application/x-tar"
	case "gz", "gzip":
		return "application/gzip"

	// Fonts
	case "woff":
		return "font/woff"
	case "woff2":
		return "font/woff2"
	case "ttf":
		return "font/ttf"
	case "otf":
		return "font/otf"

	// Other
	case "wasm":
		return "application/wasm"
	default:
		return "application/octet-stream"
	}
}

// =============================================================================
// Lifecycle Hooks
// =============================================================================

// LifecycleHook is a function that runs during startup or shutdown
type LifecycleHook func() error

// LifecycleEntry stores a hook with its name
type LifecycleEntry struct {
	Name string
	Hook LifecycleHook
}

// Lifecycle manages startup and shutdown hooks
type Lifecycle struct {
	startupHooks  []LifecycleEntry
	shutdownHooks []LifecycleEntry
}

// NewLifecycle creates a new lifecycle manager
func NewLifecycle() *Lifecycle {
	return &Lifecycle{
		startupHooks:  []LifecycleEntry{},
		shutdownHooks: []LifecycleEntry{},
	}
}

// OnStartup registers a startup hook
func (l *Lifecycle) OnStartup(name string, hook LifecycleHook) {
	l.startupHooks = append(l.startupHooks, LifecycleEntry{Name: name, Hook: hook})
}

// OnShutdown registers a shutdown hook
func (l *Lifecycle) OnShutdown(name string, hook LifecycleHook) {
	l.shutdownHooks = append(l.shutdownHooks, LifecycleEntry{Name: name, Hook: hook})
}

// RunStartup runs all startup hooks in order
func (l *Lifecycle) RunStartup() error {
	for _, entry := range l.startupHooks {
		if err := entry.Hook(); err != nil {
			return fmt.Errorf("startup hook %s failed: %w", entry.Name, err)
		}
	}
	return nil
}

// RunShutdown runs all shutdown hooks in reverse order (LIFO)
func (l *Lifecycle) RunShutdown() error {
	var lastErr error
	for i := len(l.shutdownHooks) - 1; i >= 0; i-- {
		entry := l.shutdownHooks[i]
		if err := entry.Hook(); err != nil {
			lastErr = fmt.Errorf("shutdown hook %s failed: %w", entry.Name, err)
		}
	}
	return lastErr
}

// StartupCount returns the number of startup hooks
func (l *Lifecycle) StartupCount() int {
	return len(l.startupHooks)
}

// ShutdownCount returns the number of shutdown hooks
func (l *Lifecycle) ShutdownCount() int {
	return len(l.shutdownHooks)
}

// App lifecycle methods

// OnStartup registers a startup hook on the app
func (a *App) OnStartup(name string, hook LifecycleHook) {
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.lifecycle == nil {
		a.lifecycle = NewLifecycle()
	}
	a.lifecycle.OnStartup(name, hook)
}

// OnShutdown registers a shutdown hook on the app
func (a *App) OnShutdown(name string, hook LifecycleHook) {
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.lifecycle == nil {
		a.lifecycle = NewLifecycle()
	}
	a.lifecycle.OnShutdown(name, hook)
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

// =============================================================================
// CORS Configuration
// =============================================================================

// CorsConfig configures CORS (Cross-Origin Resource Sharing) middleware.
type CorsConfig struct {
	allowedOrigins   map[string]bool
	allowAnyOrigin   bool
	allowedMethods   map[string]bool
	allowedHeaders   map[string]bool
	exposedHeaders   map[string]bool
	allowCredentials bool
	maxAgeSeconds    uint32
}

// NewCorsConfig creates a new CORS configuration with sensible defaults.
func NewCorsConfig() *CorsConfig {
	return &CorsConfig{
		allowedOrigins:   make(map[string]bool),
		allowAnyOrigin:   false,
		allowedMethods:   map[string]bool{"GET": true, "HEAD": true, "POST": true, "PUT": true, "DELETE": true, "PATCH": true},
		allowedHeaders:   map[string]bool{"content-type": true, "authorization": true, "x-request-id": true},
		exposedHeaders:   make(map[string]bool),
		allowCredentials: false,
		maxAgeSeconds:    3600,
	}
}

// AllowAnyOrigin allows requests from any origin.
func (c *CorsConfig) AllowAnyOrigin() *CorsConfig {
	c.allowAnyOrigin = true
	return c
}

// AllowOrigin adds an allowed origin.
func (c *CorsConfig) AllowOrigin(origin string) *CorsConfig {
	c.allowedOrigins[origin] = true
	return c
}

// AllowOrigins adds multiple allowed origins.
func (c *CorsConfig) AllowOrigins(origins []string) *CorsConfig {
	for _, o := range origins {
		c.allowedOrigins[o] = true
	}
	return c
}

// AllowMethod adds an allowed HTTP method.
func (c *CorsConfig) AllowMethod(method string) *CorsConfig {
	c.allowedMethods[method] = true
	return c
}

// AllowMethods sets the allowed HTTP methods.
func (c *CorsConfig) AllowMethods(methods []string) *CorsConfig {
	c.allowedMethods = make(map[string]bool)
	for _, m := range methods {
		c.allowedMethods[m] = true
	}
	return c
}

// AllowHeader adds an allowed request header.
func (c *CorsConfig) AllowHeader(header string) *CorsConfig {
	c.allowedHeaders[header] = true
	return c
}

// AllowHeaders sets the allowed request headers.
func (c *CorsConfig) AllowHeaders(headers []string) *CorsConfig {
	c.allowedHeaders = make(map[string]bool)
	for _, h := range headers {
		c.allowedHeaders[h] = true
	}
	return c
}

// ExposeHeader adds a header to expose to the browser.
func (c *CorsConfig) ExposeHeader(header string) *CorsConfig {
	c.exposedHeaders[header] = true
	return c
}

// AllowCredentials sets whether credentials are allowed.
func (c *CorsConfig) AllowCredentials(allow bool) *CorsConfig {
	c.allowCredentials = allow
	return c
}

// MaxAge sets the max age for preflight cache (in seconds).
func (c *CorsConfig) MaxAge(seconds uint32) *CorsConfig {
	c.maxAgeSeconds = seconds
	return c
}

// IsOriginAllowed checks if an origin is allowed.
func (c *CorsConfig) IsOriginAllowed(origin string) bool {
	return c.allowAnyOrigin || c.allowedOrigins[origin]
}

// IsMethodAllowed checks if a method is allowed.
func (c *CorsConfig) IsMethodAllowed(method string) bool {
	return c.allowedMethods[method]
}

// IsHeaderAllowed checks if a header is allowed.
func (c *CorsConfig) IsHeaderAllowed(header string) bool {
	return c.allowedHeaders[header]
}

// GetMaxAge returns the max age in seconds.
func (c *CorsConfig) GetMaxAge() uint32 {
	return c.maxAgeSeconds
}

// GetAllowCredentials returns whether credentials are allowed.
func (c *CorsConfig) GetAllowCredentials() bool {
	return c.allowCredentials
}

// =============================================================================
// Rate Limit Configuration
// =============================================================================

// RateLimitConfig configures rate limiting middleware.
type RateLimitConfig struct {
	requestsPerSecond float64
	burstSize         uint32
	keyExtractor      string
	exemptPaths       map[string]bool
	enabled           bool
}

// NewRateLimitConfig creates a new rate limit configuration with sensible defaults.
func NewRateLimitConfig() *RateLimitConfig {
	return &RateLimitConfig{
		requestsPerSecond: 100.0,
		burstSize:         10,
		keyExtractor:      "ip",
		exemptPaths:       map[string]bool{"/health": true, "/ready": true},
		enabled:           true,
	}
}

// RequestsPerSecond sets the requests per second limit.
func (c *RateLimitConfig) RequestsPerSecond(rps float64) *RateLimitConfig {
	c.requestsPerSecond = rps
	return c
}

// BurstSize sets the burst size (max tokens in bucket).
func (c *RateLimitConfig) BurstSize(size uint32) *RateLimitConfig {
	c.burstSize = size
	return c
}

// KeyExtractor sets the key extractor type ('ip', 'user', 'api_key', 'header:X-Custom').
func (c *RateLimitConfig) KeyExtractor(extractor string) *RateLimitConfig {
	c.keyExtractor = extractor
	return c
}

// ExemptPath adds a path to exempt from rate limiting.
func (c *RateLimitConfig) ExemptPath(path string) *RateLimitConfig {
	c.exemptPaths[path] = true
	return c
}

// ExemptPaths adds multiple paths to exempt from rate limiting.
func (c *RateLimitConfig) ExemptPaths(paths []string) *RateLimitConfig {
	for _, p := range paths {
		c.exemptPaths[p] = true
	}
	return c
}

// Enabled enables or disables rate limiting.
func (c *RateLimitConfig) Enabled(enabled bool) *RateLimitConfig {
	c.enabled = enabled
	return c
}

// IsPathExempt checks if a path is exempt from rate limiting.
func (c *RateLimitConfig) IsPathExempt(path string) bool {
	return c.exemptPaths[path]
}

// GetRequestsPerSecond returns the requests per second limit.
func (c *RateLimitConfig) GetRequestsPerSecond() float64 {
	return c.requestsPerSecond
}

// GetBurstSize returns the burst size.
func (c *RateLimitConfig) GetBurstSize() uint32 {
	return c.burstSize
}

// GetKeyExtractor returns the key extractor type.
func (c *RateLimitConfig) GetKeyExtractor() string {
	return c.keyExtractor
}

// IsEnabled returns whether rate limiting is enabled.
func (c *RateLimitConfig) IsEnabled() bool {
	return c.enabled
}

// =============================================================================
// Compression Configuration
// =============================================================================

// CompressionAlgorithm represents a compression algorithm.
type CompressionAlgorithm int

const (
	CompressionGzip CompressionAlgorithm = iota
	CompressionBrotli
	CompressionDeflate
	CompressionZstd
)

// CompressionConfig configures compression middleware.
type CompressionConfig struct {
	enableGzip      bool
	enableBrotli    bool
	enableDeflate   bool
	enableZstd      bool
	minSizeBytes    uint32
	compressionLevel uint32
	contentTypes    map[string]bool
}

// NewCompressionConfig creates a new compression configuration with sensible defaults.
func NewCompressionConfig() *CompressionConfig {
	return &CompressionConfig{
		enableGzip:      true,
		enableBrotli:    true,
		enableDeflate:   false,
		enableZstd:      false,
		minSizeBytes:    860,
		compressionLevel: 4,
		contentTypes: map[string]bool{
			"text/html":              true,
			"text/css":               true,
			"text/plain":             true,
			"text/xml":               true,
			"text/javascript":        true,
			"application/javascript": true,
			"application/json":       true,
			"application/xml":        true,
			"image/svg+xml":          true,
		},
	}
}

// EnableGzip enables or disables gzip compression.
func (c *CompressionConfig) EnableGzip(enable bool) *CompressionConfig {
	c.enableGzip = enable
	return c
}

// EnableBrotli enables or disables Brotli compression.
func (c *CompressionConfig) EnableBrotli(enable bool) *CompressionConfig {
	c.enableBrotli = enable
	return c
}

// EnableDeflate enables or disables deflate compression.
func (c *CompressionConfig) EnableDeflate(enable bool) *CompressionConfig {
	c.enableDeflate = enable
	return c
}

// EnableZstd enables or disables Zstandard compression.
func (c *CompressionConfig) EnableZstd(enable bool) *CompressionConfig {
	c.enableZstd = enable
	return c
}

// MinSize sets the minimum response size to compress (in bytes).
func (c *CompressionConfig) MinSize(bytes uint32) *CompressionConfig {
	c.minSizeBytes = bytes
	return c
}

// Level sets the compression level (1-9, higher = better compression but slower).
func (c *CompressionConfig) Level(level uint32) *CompressionConfig {
	if level < 1 {
		level = 1
	}
	if level > 9 {
		level = 9
	}
	c.compressionLevel = level
	return c
}

// AddContentType adds a content type to compress.
func (c *CompressionConfig) AddContentType(contentType string) *CompressionConfig {
	c.contentTypes[contentType] = true
	return c
}

// ContentTypes sets the content types to compress.
func (c *CompressionConfig) ContentTypes(types []string) *CompressionConfig {
	c.contentTypes = make(map[string]bool)
	for _, t := range types {
		c.contentTypes[t] = true
	}
	return c
}

// IsGzipEnabled returns whether gzip is enabled.
func (c *CompressionConfig) IsGzipEnabled() bool {
	return c.enableGzip
}

// IsBrotliEnabled returns whether Brotli is enabled.
func (c *CompressionConfig) IsBrotliEnabled() bool {
	return c.enableBrotli
}

// IsDeflateEnabled returns whether deflate is enabled.
func (c *CompressionConfig) IsDeflateEnabled() bool {
	return c.enableDeflate
}

// IsZstdEnabled returns whether Zstandard is enabled.
func (c *CompressionConfig) IsZstdEnabled() bool {
	return c.enableZstd
}

// GetMinSize returns the minimum size threshold.
func (c *CompressionConfig) GetMinSize() uint32 {
	return c.minSizeBytes
}

// GetLevel returns the compression level.
func (c *CompressionConfig) GetLevel() uint32 {
	return c.compressionLevel
}

// ShouldCompress checks if a content type should be compressed.
func (c *CompressionConfig) ShouldCompress(contentType string) bool {
	// Check exact match
	if c.contentTypes[contentType] {
		return true
	}
	// Check prefix match (e.g., "text/html; charset=utf-8")
	for ct := range c.contentTypes {
		if len(contentType) > len(ct) && contentType[:len(ct)+1] == ct+";" {
			return true
		}
	}
	return false
}

// GetEnabledAlgorithms returns enabled algorithms as strings.
func (c *CompressionConfig) GetEnabledAlgorithms() []string {
	var algos []string
	if c.enableBrotli {
		algos = append(algos, "br")
	}
	if c.enableGzip {
		algos = append(algos, "gzip")
	}
	if c.enableDeflate {
		algos = append(algos, "deflate")
	}
	if c.enableZstd {
		algos = append(algos, "zstd")
	}
	return algos
}

// =============================================================================
// Static Files Configuration
// =============================================================================

// StaticFilesConfig configures static file serving middleware.
type StaticFilesConfig struct {
	directory            string
	prefix               string
	indexFile            string
	cacheMaxAgeSeconds   uint32
	enablePrecompressed  bool
	fallbackFile         string
}

// NewStaticFilesConfig creates a new static files configuration with sensible defaults.
func NewStaticFilesConfig() *StaticFilesConfig {
	return &StaticFilesConfig{
		directory:            "./static",
		prefix:               "/static",
		indexFile:            "index.html",
		cacheMaxAgeSeconds:   86400,
		enablePrecompressed:  true,
		fallbackFile:         "",
	}
}

// Directory sets the directory to serve files from.
func (c *StaticFilesConfig) Directory(dir string) *StaticFilesConfig {
	c.directory = dir
	return c
}

// Prefix sets the URL prefix for static files.
func (c *StaticFilesConfig) Prefix(prefix string) *StaticFilesConfig {
	if prefix != "" && prefix[0] != '/' {
		prefix = "/" + prefix
	}
	c.prefix = prefix
	return c
}

// Index sets the index file name.
func (c *StaticFilesConfig) Index(file string) *StaticFilesConfig {
	c.indexFile = file
	return c
}

// CacheMaxAge sets the cache max age in seconds.
func (c *StaticFilesConfig) CacheMaxAge(seconds uint32) *StaticFilesConfig {
	c.cacheMaxAgeSeconds = seconds
	return c
}

// Precompressed enables or disables serving precompressed files (.gz, .br).
func (c *StaticFilesConfig) Precompressed(enable bool) *StaticFilesConfig {
	c.enablePrecompressed = enable
	return c
}

// Fallback sets a fallback file for SPA routing.
func (c *StaticFilesConfig) Fallback(file string) *StaticFilesConfig {
	c.fallbackFile = file
	return c
}

// GetDirectory returns the directory path.
func (c *StaticFilesConfig) GetDirectory() string {
	return c.directory
}

// GetPrefix returns the URL prefix.
func (c *StaticFilesConfig) GetPrefix() string {
	return c.prefix
}

// GetIndex returns the index file name.
func (c *StaticFilesConfig) GetIndex() string {
	return c.indexFile
}

// GetCacheMaxAge returns the cache max age in seconds.
func (c *StaticFilesConfig) GetCacheMaxAge() uint32 {
	return c.cacheMaxAgeSeconds
}

// IsPrecompressedEnabled returns whether precompressed files are enabled.
func (c *StaticFilesConfig) IsPrecompressedEnabled() bool {
	return c.enablePrecompressed
}

// GetFallback returns the fallback file if set.
func (c *StaticFilesConfig) GetFallback() string {
	return c.fallbackFile
}

// ResolvePath resolves a request path to a file path.
// Returns empty string if the path doesn't match the prefix or is invalid.
func (c *StaticFilesConfig) ResolvePath(requestPath string) string {
	if len(requestPath) < len(c.prefix) || requestPath[:len(c.prefix)] != c.prefix {
		return ""
	}

	relative := requestPath[len(c.prefix):]
	for len(relative) > 0 && relative[0] == '/' {
		relative = relative[1:]
	}

	// Prevent directory traversal
	if len(relative) >= 2 {
		for i := 0; i < len(relative)-1; i++ {
			if relative[i] == '.' && relative[i+1] == '.' {
				return ""
			}
		}
	}

	if relative == "" {
		return c.directory + "/" + c.indexFile
	}
	return c.directory + "/" + relative
}

// =============================================================================
// TestClient (Phase A15.6)
// =============================================================================

// TestClient provides an HTTP client for testing Archimedes handlers.
// It simulates HTTP requests without starting a real server.
//
// Example usage:
//
//	client := archimedes.NewTestClient(app)
//	defer client.Close()
//
//	resp := client.Get("/users/123")
//	resp.AssertStatus(200).
//	    AssertContentType("application/json").
//	    AssertBodyContains("john")
type TestClient struct {
	app            *App
	defaultHeaders map[string]string
}

// NewTestClient creates a test client for the given app.
func NewTestClient(app *App) *TestClient {
	return &TestClient{
		app:            app,
		defaultHeaders: make(map[string]string),
	}
}

// WithHeader adds a default header to all requests.
func (c *TestClient) WithHeader(name, value string) *TestClient {
	c.defaultHeaders[name] = value
	return c
}

// WithBearerToken sets the Authorization header to use a bearer token.
func (c *TestClient) WithBearerToken(token string) *TestClient {
	c.defaultHeaders["Authorization"] = "Bearer " + token
	return c
}

// Get performs a GET request.
func (c *TestClient) Get(path string) *TestResponse {
	return c.request("GET", path, nil)
}

// Post performs a POST request with a body.
func (c *TestClient) Post(path string, body []byte) *TestResponse {
	return c.request("POST", path, body)
}

// PostJSON performs a POST request with a JSON body.
func (c *TestClient) PostJSON(path string, data interface{}) *TestResponse {
	body, err := json.Marshal(data)
	if err != nil {
		return &TestResponse{
			statusCode: 0,
			headers:    make(map[string]string),
			body:       []byte{},
			err:        fmt.Errorf("failed to marshal JSON: %w", err),
		}
	}
	c.defaultHeaders["Content-Type"] = "application/json"
	resp := c.request("POST", path, body)
	delete(c.defaultHeaders, "Content-Type")
	return resp
}

// Put performs a PUT request with a body.
func (c *TestClient) Put(path string, body []byte) *TestResponse {
	return c.request("PUT", path, body)
}

// PutJSON performs a PUT request with a JSON body.
func (c *TestClient) PutJSON(path string, data interface{}) *TestResponse {
	body, err := json.Marshal(data)
	if err != nil {
		return &TestResponse{
			statusCode: 0,
			headers:    make(map[string]string),
			body:       []byte{},
			err:        fmt.Errorf("failed to marshal JSON: %w", err),
		}
	}
	c.defaultHeaders["Content-Type"] = "application/json"
	resp := c.request("PUT", path, body)
	delete(c.defaultHeaders, "Content-Type")
	return resp
}

// Patch performs a PATCH request with a body.
func (c *TestClient) Patch(path string, body []byte) *TestResponse {
	return c.request("PATCH", path, body)
}

// PatchJSON performs a PATCH request with a JSON body.
func (c *TestClient) PatchJSON(path string, data interface{}) *TestResponse {
	body, err := json.Marshal(data)
	if err != nil {
		return &TestResponse{
			statusCode: 0,
			headers:    make(map[string]string),
			body:       []byte{},
			err:        fmt.Errorf("failed to marshal JSON: %w", err),
		}
	}
	c.defaultHeaders["Content-Type"] = "application/json"
	resp := c.request("PATCH", path, body)
	delete(c.defaultHeaders, "Content-Type")
	return resp
}

// Delete performs a DELETE request.
func (c *TestClient) Delete(path string) *TestResponse {
	return c.request("DELETE", path, nil)
}

// Options performs an OPTIONS request.
func (c *TestClient) Options(path string) *TestResponse {
	return c.request("OPTIONS", path, nil)
}

// Head performs a HEAD request.
func (c *TestClient) Head(path string) *TestResponse {
	return c.request("HEAD", path, nil)
}

// request performs an HTTP request (mock implementation).
// TODO: Integrate with actual FFI test_client when available
func (c *TestClient) request(method, path string, body []byte) *TestResponse {
	// This is a mock implementation until the FFI TestClient is integrated.
	// For now, we return a placeholder response.
	// In a real implementation, this would call the FFI functions:
	// C.archimedes_test_client_request(...)
	return &TestResponse{
		statusCode: 200,
		headers:    make(map[string]string),
		body:       []byte(`{"status":"mock_response"}`),
	}
}

// Close releases resources associated with the test client.
func (c *TestClient) Close() {
	c.defaultHeaders = nil
}

// TestResponse represents an HTTP response from TestClient.
type TestResponse struct {
	statusCode int
	headers    map[string]string
	body       []byte
	err        error
}

// StatusCode returns the HTTP status code.
func (r *TestResponse) StatusCode() int {
	return r.statusCode
}

// Headers returns all response headers.
func (r *TestResponse) Headers() map[string]string {
	return r.headers
}

// Header returns a specific header value (case-insensitive).
func (r *TestResponse) Header(name string) string {
	// Try exact match first
	if val, ok := r.headers[name]; ok {
		return val
	}
	// Try case-insensitive match
	lower := strings.ToLower(name)
	for k, v := range r.headers {
		if strings.ToLower(k) == lower {
			return v
		}
	}
	return ""
}

// Body returns the raw response body.
func (r *TestResponse) Body() []byte {
	return r.body
}

// Text returns the response body as a string.
func (r *TestResponse) Text() string {
	return string(r.body)
}

// JSON unmarshals the response body into the given value.
func (r *TestResponse) JSON(v interface{}) error {
	return json.Unmarshal(r.body, v)
}

// IsSuccess returns true if status is 2xx.
func (r *TestResponse) IsSuccess() bool {
	return r.statusCode >= 200 && r.statusCode < 300
}

// IsClientError returns true if status is 4xx.
func (r *TestResponse) IsClientError() bool {
	return r.statusCode >= 400 && r.statusCode < 500
}

// IsServerError returns true if status is 5xx.
func (r *TestResponse) IsServerError() bool {
	return r.statusCode >= 500 && r.statusCode < 600
}

// Error returns any error that occurred during the request.
func (r *TestResponse) Error() error {
	return r.err
}

// AssertStatus asserts the response has the expected status code.
// Returns the response for chaining.
func (r *TestResponse) AssertStatus(expected int) *TestResponse {
	if r.statusCode != expected {
		panic(fmt.Sprintf("expected status %d, got %d", expected, r.statusCode))
	}
	return r
}

// AssertSuccess asserts the response has a 2xx status code.
// Returns the response for chaining.
func (r *TestResponse) AssertSuccess() *TestResponse {
	if !r.IsSuccess() {
		panic(fmt.Sprintf("expected success status (2xx), got %d", r.statusCode))
	}
	return r
}

// AssertHeader asserts a header has the expected value.
// Returns the response for chaining.
func (r *TestResponse) AssertHeader(name, expected string) *TestResponse {
	actual := r.Header(name)
	if actual != expected {
		panic(fmt.Sprintf("expected header %q to be %q, got %q", name, expected, actual))
	}
	return r
}

// AssertContentType asserts the Content-Type header matches.
// Returns the response for chaining.
func (r *TestResponse) AssertContentType(expected string) *TestResponse {
	return r.AssertHeader("Content-Type", expected)
}

// AssertBodyContains asserts the body contains the expected substring.
// Returns the response for chaining.
func (r *TestResponse) AssertBodyContains(expected string) *TestResponse {
	if !strings.Contains(string(r.body), expected) {
		panic(fmt.Sprintf("expected body to contain %q, got %q", expected, string(r.body)))
	}
	return r
}

// AssertBodyEquals asserts the body exactly matches the expected string.
// Returns the response for chaining.
func (r *TestResponse) AssertBodyEquals(expected string) *TestResponse {
	if string(r.body) != expected {
		panic(fmt.Sprintf("expected body %q, got %q", expected, string(r.body)))
	}
	return r
}

// AssertJSON asserts the body matches the expected JSON value.
// Returns the response for chaining.
func (r *TestResponse) AssertJSON(expected interface{}) *TestResponse {
	expectedJSON, err := json.Marshal(expected)
	if err != nil {
		panic(fmt.Sprintf("failed to marshal expected JSON: %v", err))
	}
	// Compare as normalized JSON
	var expectedVal, actualVal interface{}
	if err := json.Unmarshal(expectedJSON, &expectedVal); err != nil {
		panic(fmt.Sprintf("failed to unmarshal expected JSON: %v", err))
	}
	if err := json.Unmarshal(r.body, &actualVal); err != nil {
		panic(fmt.Sprintf("failed to unmarshal actual JSON: %v", err))
	}
	if !jsonEqual(expectedVal, actualVal) {
		panic(fmt.Sprintf("expected JSON %s, got %s", string(expectedJSON), string(r.body)))
	}
	return r
}

// jsonEqual recursively compares two JSON values.
func jsonEqual(a, b interface{}) bool {
	switch aVal := a.(type) {
	case map[string]interface{}:
		bVal, ok := b.(map[string]interface{})
		if !ok || len(aVal) != len(bVal) {
			return false
		}
		for k, v := range aVal {
			if !jsonEqual(v, bVal[k]) {
				return false
			}
		}
		return true
	case []interface{}:
		bVal, ok := b.([]interface{})
		if !ok || len(aVal) != len(bVal) {
			return false
		}
		for i, v := range aVal {
			if !jsonEqual(v, bVal[i]) {
				return false
			}
		}
		return true
	default:
		return a == b
	}
}

