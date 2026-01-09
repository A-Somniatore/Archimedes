// Package main implements a Go example service using native Archimedes bindings.
//
// This service demonstrates using Archimedes Go bindings directly instead of
// net/http, Gin, Chi, or other Go web frameworks.
//
// Features demonstrated:
// - Contract-first API
// - Built-in middleware
// - Sub-routers with prefix and tag
// - Lifecycle hooks (startup/shutdown)
package main

import (
	"fmt"
	"log"
	"sync"
	"time"

	"github.com/themis-platform/archimedes-go/archimedes"
)

// =============================================================================
// Models
// =============================================================================

// User represents a user in our system.
type User struct {
	ID        string `json:"id"`
	Name      string `json:"name"`
	Email     string `json:"email"`
	CreatedAt string `json:"created_at"`
}

// CreateUserRequest is the request body for creating a user.
type CreateUserRequest struct {
	Name  string `json:"name"`
	Email string `json:"email"`
}

// UpdateUserRequest is the request body for updating a user.
type UpdateUserRequest struct {
	Name  *string `json:"name,omitempty"`
	Email *string `json:"email,omitempty"`
}

// HealthResponse is the health check response.
type HealthResponse struct {
	Status    string `json:"status"`
	Service   string `json:"service"`
	Timestamp string `json:"timestamp"`
}

// UsersResponse is the list users response.
type UsersResponse struct {
	Users []User `json:"users"`
	Total int    `json:"total"`
}

// ErrorResponse is the error response format.
type ErrorResponse struct {
	Code      string `json:"code"`
	Message   string `json:"message"`
	RequestID string `json:"request_id,omitempty"`
}

// =============================================================================
// In-Memory Database
// =============================================================================

type userStore struct {
	mu     sync.RWMutex
	users  map[string]User
	nextID int
}

var store = &userStore{
	users: map[string]User{
		"1": {
			ID:        "1",
			Name:      "Alice Smith",
			Email:     "alice@example.com",
			CreatedAt: "2026-01-01T00:00:00Z",
		},
		"2": {
			ID:        "2",
			Name:      "Bob Johnson",
			Email:     "bob@example.com",
			CreatedAt: "2026-01-02T00:00:00Z",
		},
	},
	nextID: 3,
}

func (s *userStore) List() []User {
	s.mu.RLock()
	defer s.mu.RUnlock()
	users := make([]User, 0, len(s.users))
	for _, u := range s.users {
		users = append(users, u)
	}
	return users
}

func (s *userStore) Get(id string) (User, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	u, ok := s.users[id]
	return u, ok
}

func (s *userStore) Create(name, email string) User {
	s.mu.Lock()
	defer s.mu.Unlock()
	id := fmt.Sprintf("%d", s.nextID)
	s.nextID++
	user := User{
		ID:        id,
		Name:      name,
		Email:     email,
		CreatedAt: time.Now().UTC().Format(time.RFC3339),
	}
	s.users[id] = user
	return user
}

func (s *userStore) Update(id string, name, email *string) (User, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	u, ok := s.users[id]
	if !ok {
		return User{}, false
	}
	if name != nil {
		u.Name = *name
	}
	if email != nil {
		u.Email = *email
	}
	s.users[id] = u
	return u, true
}

func (s *userStore) Delete(id string) bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	_, ok := s.users[id]
	if ok {
		delete(s.users, id)
	}
	return ok
}

func (s *userStore) EmailExists(email, excludeID string) bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	for id, u := range s.users {
		if u.Email == email && id != excludeID {
			return true
		}
	}
	return false
}

// =============================================================================
// Main
// =============================================================================

func main() {
	log.Println("Starting Go Native Example Service...")
	log.Printf("Archimedes version: %s", archimedes.Version())

	// Create application with configuration
	app, err := archimedes.New(archimedes.Config{
		Contract:         "../contract.json",
		Port:             8003,
		ServiceName:      "go-native-example",
		EnableValidation: true,
		EnableTracing:    true,
	})
	if err != nil {
		log.Fatalf("Failed to create app: %v", err)
	}
	defer app.Close()

	// =========================================================================
	// Lifecycle Hooks
	// =========================================================================
	
	// Startup hooks run in registration order
	app.OnStartup("database_init", func() error {
		log.Println("[Lifecycle] Initializing database connection...")
		// In a real app: return db.Connect()
		return nil
	})

	app.OnStartup("cache_warmup", func() error {
		log.Println("[Lifecycle] Warming up cache...")
		// In a real app: return cache.Warmup()
		return nil
	})

	// Shutdown hooks run in reverse order (LIFO)
	app.OnShutdown("metrics_flush", func() error {
		log.Println("[Lifecycle] Flushing metrics...")
		// In a real app: return metrics.Flush()
		return nil
	})

	app.OnShutdown("database_close", func() error {
		log.Println("[Lifecycle] Closing database connection...")
		// In a real app: return db.Close()
		return nil
	})

	// Register handlers
	registerHandlers(app)

	// Start server
	log.Println("Server starting on :8003")
	if err := app.Run(":8003"); err != nil {
		log.Fatalf("Server error: %v", err)
	}
}

func registerHandlers(app *archimedes.App) {
	// Health check
	app.Operation("healthCheck", func(ctx *archimedes.Context) error {
		return ctx.JSON(200, HealthResponse{
			Status:    "healthy",
			Service:   "go-native-example",
			Timestamp: time.Now().UTC().Format(time.RFC3339),
		})
	})

	// List users
	app.Operation("listUsers", func(ctx *archimedes.Context) error {
		users := store.List()
		return ctx.JSON(200, UsersResponse{
			Users: users,
			Total: len(users),
		})
	})

	// Get user
	app.Operation("getUser", func(ctx *archimedes.Context) error {
		userID := ctx.PathParam("userId")
		if userID == "" {
			return ctx.JSON(400, ErrorResponse{
				Code:      "MISSING_USER_ID",
				Message:   "User ID is required",
				RequestID: ctx.RequestID,
			})
		}

		user, ok := store.Get(userID)
		if !ok {
			return ctx.JSON(404, ErrorResponse{
				Code:      "USER_NOT_FOUND",
				Message:   fmt.Sprintf("User with ID %s not found", userID),
				RequestID: ctx.RequestID,
			})
		}

		return ctx.JSON(200, user)
	})

	// Create user
	app.Operation("createUser", func(ctx *archimedes.Context) error {
		var req CreateUserRequest
		if err := ctx.Bind(&req); err != nil {
			return ctx.JSON(400, ErrorResponse{
				Code:      "INVALID_REQUEST",
				Message:   "Invalid request body",
				RequestID: ctx.RequestID,
			})
		}

		if req.Name == "" || req.Email == "" {
			return ctx.JSON(400, ErrorResponse{
				Code:      "INVALID_REQUEST",
				Message:   "Name and email are required",
				RequestID: ctx.RequestID,
			})
		}

		// Check for duplicate email
		if store.EmailExists(req.Email, "") {
			return ctx.JSON(409, ErrorResponse{
				Code:      "DUPLICATE_EMAIL",
				Message:   fmt.Sprintf("User with email %s already exists", req.Email),
				RequestID: ctx.RequestID,
			})
		}

		user := store.Create(req.Name, req.Email)
		return ctx.JSON(201, user)
	})

	// Update user
	app.Operation("updateUser", func(ctx *archimedes.Context) error {
		userID := ctx.PathParam("userId")
		if userID == "" {
			return ctx.JSON(400, ErrorResponse{
				Code:      "MISSING_USER_ID",
				Message:   "User ID is required",
				RequestID: ctx.RequestID,
			})
		}

		var req UpdateUserRequest
		if err := ctx.Bind(&req); err != nil {
			return ctx.JSON(400, ErrorResponse{
				Code:      "INVALID_REQUEST",
				Message:   "Invalid request body",
				RequestID: ctx.RequestID,
			})
		}

		// Check for duplicate email
		if req.Email != nil && store.EmailExists(*req.Email, userID) {
			return ctx.JSON(409, ErrorResponse{
				Code:      "DUPLICATE_EMAIL",
				Message:   fmt.Sprintf("User with email %s already exists", *req.Email),
				RequestID: ctx.RequestID,
			})
		}

		user, ok := store.Update(userID, req.Name, req.Email)
		if !ok {
			return ctx.JSON(404, ErrorResponse{
				Code:      "USER_NOT_FOUND",
				Message:   fmt.Sprintf("User with ID %s not found", userID),
				RequestID: ctx.RequestID,
			})
		}

		return ctx.JSON(200, user)
	})

	// Delete user
	app.Operation("deleteUser", func(ctx *archimedes.Context) error {
		userID := ctx.PathParam("userId")
		if userID == "" {
			return ctx.JSON(400, ErrorResponse{
				Code:      "MISSING_USER_ID",
				Message:   "User ID is required",
				RequestID: ctx.RequestID,
			})
		}

		if !store.Delete(userID) {
			return ctx.JSON(404, ErrorResponse{
				Code:      "USER_NOT_FOUND",
				Message:   fmt.Sprintf("User with ID %s not found", userID),
				RequestID: ctx.RequestID,
			})
		}

		return ctx.NoContent()
	})

	// =========================================================================
	// Admin Router (sub-router example)
	// =========================================================================
	adminRouter := archimedes.NewRouter().
		Prefix("/admin").
		Tag("admin").
		Tag("internal")

	adminRouter.Operation("getStats", func(ctx *archimedes.Context) error {
		users := store.List()
		return ctx.JSON(200, map[string]any{
			"total_users": len(users),
		})
	})

	// Merge admin router into main app
	app.Merge(adminRouter)
}
