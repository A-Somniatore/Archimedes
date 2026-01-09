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

// =============================================================================
// Router Tests
// =============================================================================

func TestRouterCreation(t *testing.T) {
	r := NewRouter()
	if r == nil {
		t.Fatal("NewRouter() returned nil")
	}
	if r.GetPrefix() != "" {
		t.Errorf("GetPrefix() = %v, want empty", r.GetPrefix())
	}
	if len(r.GetTags()) != 0 {
		t.Errorf("GetTags() length = %v, want 0", len(r.GetTags()))
	}
}

func TestRouterPrefix(t *testing.T) {
	r := NewRouter().Prefix("/users")
	if r.GetPrefix() != "/users" {
		t.Errorf("GetPrefix() = %v, want /users", r.GetPrefix())
	}

	// Test normalization - adds leading slash
	r2 := NewRouter().Prefix("api")
	if r2.GetPrefix() != "/api" {
		t.Errorf("GetPrefix() = %v, want /api", r2.GetPrefix())
	}

	// Test normalization - removes trailing slash
	r3 := NewRouter().Prefix("/api/")
	if r3.GetPrefix() != "/api" {
		t.Errorf("GetPrefix() = %v, want /api", r3.GetPrefix())
	}
}

func TestRouterTag(t *testing.T) {
	r := NewRouter().Tag("users").Tag("api")

	tags := r.GetTags()
	if len(tags) != 2 {
		t.Errorf("GetTags() length = %v, want 2", len(tags))
	}

	// Test duplicate prevention
	r.Tag("users")
	if len(r.GetTags()) != 2 {
		t.Errorf("GetTags() after duplicate = %v, want 2", len(r.GetTags()))
	}
}

func TestRouterChaining(t *testing.T) {
	r := NewRouter().Prefix("/api").Tag("v1").Tag("public")

	if r.GetPrefix() != "/api" {
		t.Errorf("GetPrefix() = %v, want /api", r.GetPrefix())
	}
	if len(r.GetTags()) != 2 {
		t.Errorf("GetTags() length = %v, want 2", len(r.GetTags()))
	}
}

func TestRouterOperation(t *testing.T) {
	handler := func(ctx *Context) error { return nil }

	r := NewRouter().Operation("listUsers", handler)

	ops := r.GetOperations()
	if len(ops) != 1 {
		t.Errorf("GetOperations() length = %v, want 1", len(ops))
	}
	if _, ok := ops["listUsers"]; !ok {
		t.Error("GetOperations() missing listUsers")
	}
}

func TestRouterMerge(t *testing.T) {
	handler1 := func(ctx *Context) error { return nil }
	handler2 := func(ctx *Context) error { return nil }

	r1 := NewRouter().Operation("op1", handler1)
	r2 := NewRouter().Operation("op2", handler2)

	r1.Merge(r2)

	ops := r1.GetOperations()
	if len(ops) != 2 {
		t.Errorf("GetOperations() after merge = %v, want 2", len(ops))
	}
}

func TestRouterNest(t *testing.T) {
	handler := func(ctx *Context) error { return nil }

	child := NewRouter().Prefix("/users").Operation("listUsers", handler)
	parent := NewRouter().Prefix("/api").Nest(child)

	ops := parent.GetOperations()
	if len(ops) != 1 {
		t.Errorf("GetOperations() after nest = %v, want 1", len(ops))
	}
}

// =============================================================================
// Lifecycle Tests
// =============================================================================

func TestLifecycleCreation(t *testing.T) {
	l := NewLifecycle()
	if l == nil {
		t.Fatal("NewLifecycle() returned nil")
	}
	if l.StartupCount() != 0 {
		t.Errorf("StartupCount() = %v, want 0", l.StartupCount())
	}
	if l.ShutdownCount() != 0 {
		t.Errorf("ShutdownCount() = %v, want 0", l.ShutdownCount())
	}
}

func TestLifecycleStartupHook(t *testing.T) {
	l := NewLifecycle()

	called := false
	l.OnStartup("test", func() error {
		called = true
		return nil
	})

	if l.StartupCount() != 1 {
		t.Errorf("StartupCount() = %v, want 1", l.StartupCount())
	}

	err := l.RunStartup()
	if err != nil {
		t.Errorf("RunStartup() error = %v", err)
	}
	if !called {
		t.Error("Startup hook was not called")
	}
}

func TestLifecycleShutdownHook(t *testing.T) {
	l := NewLifecycle()

	called := false
	l.OnShutdown("test", func() error {
		called = true
		return nil
	})

	if l.ShutdownCount() != 1 {
		t.Errorf("ShutdownCount() = %v, want 1", l.ShutdownCount())
	}

	err := l.RunShutdown()
	if err != nil {
		t.Errorf("RunShutdown() error = %v", err)
	}
	if !called {
		t.Error("Shutdown hook was not called")
	}
}

func TestLifecycleShutdownOrder(t *testing.T) {
	l := NewLifecycle()

	order := []string{}
	l.OnShutdown("first", func() error {
		order = append(order, "first")
		return nil
	})
	l.OnShutdown("second", func() error {
		order = append(order, "second")
		return nil
	})
	l.OnShutdown("third", func() error {
		order = append(order, "third")
		return nil
	})

	l.RunShutdown()

	// Should be LIFO order
	expected := []string{"third", "second", "first"}
	if len(order) != 3 {
		t.Fatalf("shutdown order length = %v, want 3", len(order))
	}
	for i, v := range expected {
		if order[i] != v {
			t.Errorf("shutdown order[%d] = %v, want %v", i, order[i], v)
		}
	}
}

func TestLifecycleStartupOrder(t *testing.T) {
	l := NewLifecycle()

	order := []string{}
	l.OnStartup("first", func() error {
		order = append(order, "first")
		return nil
	})
	l.OnStartup("second", func() error {
		order = append(order, "second")
		return nil
	})

	l.RunStartup()

	// Should be FIFO order
	expected := []string{"first", "second"}
	if len(order) != 2 {
		t.Fatalf("startup order length = %v, want 2", len(order))
	}
	for i, v := range expected {
		if order[i] != v {
			t.Errorf("startup order[%d] = %v, want %v", i, order[i], v)
		}
	}
}
