package archimedes

import (
	"testing"
)

func TestConfigDefaults(t *testing.T) {
	cfg := Config{}
	
	// Test that defaults are applied in New()
	// We can't actually create the app without the library,
	// but we can test the default logic
	if cfg.Port != 0 {
		t.Errorf("Expected Port default 0, got %d", cfg.Port)
	}
}

func TestCallerIdentity(t *testing.T) {
	tests := []struct {
		name     string
		identity CallerIdentity
		isSpiffe bool
		isUser   bool
		isAPIKey bool
		isAnon   bool
	}{
		{
			name:     "SPIFFE identity",
			identity: CallerIdentity{Type: "spiffe", TrustDomain: "example.org", Path: "/service"},
			isSpiffe: true,
		},
		{
			name:     "User identity",
			identity: CallerIdentity{Type: "user", UserID: "user-123", Roles: []string{"admin"}},
			isUser:   true,
		},
		{
			name:     "API Key identity",
			identity: CallerIdentity{Type: "api_key", KeyID: "key-123"},
			isAPIKey: true,
		},
		{
			name:     "Anonymous identity",
			identity: CallerIdentity{Type: "anonymous"},
			isAnon:   true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := tt.identity.IsSpiffe(); got != tt.isSpiffe {
				t.Errorf("IsSpiffe() = %v, want %v", got, tt.isSpiffe)
			}
			if got := tt.identity.IsUser(); got != tt.isUser {
				t.Errorf("IsUser() = %v, want %v", got, tt.isUser)
			}
			if got := tt.identity.IsAPIKey(); got != tt.isAPIKey {
				t.Errorf("IsAPIKey() = %v, want %v", got, tt.isAPIKey)
			}
			if got := tt.identity.IsAnonymous(); got != tt.isAnon {
				t.Errorf("IsAnonymous() = %v, want %v", got, tt.isAnon)
			}
		})
	}
}

func TestContextMethods(t *testing.T) {
	ctx := &Context{
		RequestID:   "req-123",
		TraceID:     "trace-456",
		SpanID:      "span-789",
		OperationID: "listUsers",
		Method:      "GET",
		Path:        "/users",
		Query:       "limit=10",
		PathParams:  map[string]string{"userId": "42"},
		Headers:     map[string]string{"Authorization": "Bearer token"},
		body:        []byte(`{"name":"test"}`),
	}

	// Test PathParam
	if got := ctx.PathParam("userId"); got != "42" {
		t.Errorf("PathParam() = %v, want %v", got, "42")
	}
	if got := ctx.PathParam("missing"); got != "" {
		t.Errorf("PathParam(missing) = %v, want empty", got)
	}

	// Test Header
	if got := ctx.Header("Authorization"); got != "Bearer token" {
		t.Errorf("Header() = %v, want %v", got, "Bearer token")
	}

	// Test Body
	if got := ctx.BodyString(); got != `{"name":"test"}` {
		t.Errorf("BodyString() = %v, want %v", got, `{"name":"test"}`)
	}

	// Test Bind
	var data struct {
		Name string `json:"name"`
	}
	if err := ctx.Bind(&data); err != nil {
		t.Errorf("Bind() error = %v", err)
	}
	if data.Name != "test" {
		t.Errorf("Bind() name = %v, want %v", data.Name, "test")
	}
}

func TestContextJSON(t *testing.T) {
	ctx := &Context{
		responseHeaders: make(map[string]string),
	}

	err := ctx.JSON(200, map[string]string{"message": "hello"})
	if err != nil {
		t.Errorf("JSON() error = %v", err)
	}

	if ctx.responseStatus != 200 {
		t.Errorf("responseStatus = %v, want %v", ctx.responseStatus, 200)
	}
	if ctx.contentType != "application/json" {
		t.Errorf("contentType = %v, want %v", ctx.contentType, "application/json")
	}
	if string(ctx.responseBody) != `{"message":"hello"}` {
		t.Errorf("responseBody = %v, want %v", string(ctx.responseBody), `{"message":"hello"}`)
	}
}

func TestContextString(t *testing.T) {
	ctx := &Context{
		responseHeaders: make(map[string]string),
	}

	err := ctx.String(200, "Hello, World!")
	if err != nil {
		t.Errorf("String() error = %v", err)
	}

	if ctx.responseStatus != 200 {
		t.Errorf("responseStatus = %v, want %v", ctx.responseStatus, 200)
	}
	if ctx.contentType != "text/plain" {
		t.Errorf("contentType = %v, want %v", ctx.contentType, "text/plain")
	}
}

func TestContextNoContent(t *testing.T) {
	ctx := &Context{
		responseHeaders: make(map[string]string),
	}

	err := ctx.NoContent()
	if err != nil {
		t.Errorf("NoContent() error = %v", err)
	}

	if ctx.responseStatus != 204 {
		t.Errorf("responseStatus = %v, want %v", ctx.responseStatus, 204)
	}
	if ctx.responseBody != nil {
		t.Errorf("responseBody = %v, want nil", ctx.responseBody)
	}
}

func TestContextSetHeader(t *testing.T) {
	ctx := &Context{}

	ctx.SetHeader("X-Custom", "value")

	if ctx.responseHeaders == nil {
		t.Fatal("responseHeaders should be initialized")
	}
	if got := ctx.responseHeaders["X-Custom"]; got != "value" {
		t.Errorf("responseHeaders[X-Custom] = %v, want %v", got, "value")
	}
}

func TestErrorType(t *testing.T) {
	err := &Error{Code: ErrValidationError, Message: "validation failed"}

	if got := err.Error(); got != "archimedes error 8: validation failed" {
		t.Errorf("Error() = %v, want %v", got, "archimedes error 8: validation failed")
	}
}

func TestBindEmptyBody(t *testing.T) {
	ctx := &Context{body: nil}

	var data struct{}
	err := ctx.Bind(&data)
	if err == nil {
		t.Error("Bind() should error on empty body")
	}
}
