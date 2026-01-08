// Package main implements a Go example service demonstrating Archimedes sidecar integration.
//
// This service shows how to build a Go microservice that works with the Archimedes sidecar
// for contract validation, authorization, and observability.
package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"strings"
	"sync"
	"time"

	"github.com/google/uuid"
)

// =============================================================================
// Models
// =============================================================================

// CallerIdentity represents the caller identity from X-Caller-Identity header.
type CallerIdentity struct {
	Type        string   `json:"type"`
	ID          string   `json:"id,omitempty"`
	TrustDomain string   `json:"trust_domain,omitempty"`
	Path        string   `json:"path,omitempty"`
	UserID      string   `json:"user_id,omitempty"`
	Roles       []string `json:"roles,omitempty"`
	KeyID       string   `json:"key_id,omitempty"`
}

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

// RequestContext contains request metadata from sidecar headers.
type RequestContext struct {
	RequestID   string
	Caller      *CallerIdentity
	OperationID string
}

// =============================================================================
// In-Memory Database
// =============================================================================

type userStore struct {
	mu    sync.RWMutex
	users map[string]User
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
}

// =============================================================================
// Helper Functions
// =============================================================================

func parseCallerIdentity(headerValue string) *CallerIdentity {
	if headerValue == "" {
		return nil
	}
	var caller CallerIdentity
	if err := json.Unmarshal([]byte(headerValue), &caller); err != nil {
		log.Printf("Failed to parse caller identity: %v", err)
		return nil
	}
	return &caller
}

func getRequestContext(r *http.Request) *RequestContext {
	requestID := r.Header.Get("X-Request-Id")
	if requestID == "" {
		requestID = uuid.New().String()
	}

	return &RequestContext{
		RequestID:   requestID,
		Caller:      parseCallerIdentity(r.Header.Get("X-Caller-Identity")),
		OperationID: r.Header.Get("X-Operation-Id"),
	}
}

func writeJSON(w http.ResponseWriter, status int, data interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(data)
}

func writeError(w http.ResponseWriter, status int, code, message, requestID string) {
	writeJSON(w, status, ErrorResponse{
		Code:      code,
		Message:   message,
		RequestID: requestID,
	})
}

func extractUserID(path string) string {
	// Path is /users/{userId}
	parts := strings.Split(path, "/")
	if len(parts) >= 3 {
		return parts[2]
	}
	return ""
}

// =============================================================================
// Handlers
// =============================================================================

func healthHandler(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, HealthResponse{
		Status:    "healthy",
		Service:   "example-go",
		Timestamp: time.Now().UTC().Format(time.RFC3339),
	})
}

func listUsersHandler(w http.ResponseWriter, r *http.Request) {
	ctx := getRequestContext(r)
	log.Printf("[%s] Listing users, caller: %+v", ctx.RequestID, ctx.Caller)

	store.mu.RLock()
	users := make([]User, 0, len(store.users))
	for _, u := range store.users {
		users = append(users, u)
	}
	store.mu.RUnlock()

	writeJSON(w, http.StatusOK, UsersResponse{
		Users: users,
		Total: len(users),
	})
}

func getUserHandler(w http.ResponseWriter, r *http.Request) {
	ctx := getRequestContext(r)
	userID := extractUserID(r.URL.Path)
	log.Printf("[%s] Getting user %s, caller: %+v", ctx.RequestID, userID, ctx.Caller)

	store.mu.RLock()
	user, exists := store.users[userID]
	store.mu.RUnlock()

	if !exists {
		writeError(w, http.StatusNotFound, "USER_NOT_FOUND",
			fmt.Sprintf("User with ID '%s' not found", userID), ctx.RequestID)
		return
	}

	writeJSON(w, http.StatusOK, user)
}

func createUserHandler(w http.ResponseWriter, r *http.Request) {
	ctx := getRequestContext(r)
	log.Printf("[%s] Creating user, caller: %+v", ctx.RequestID, ctx.Caller)

	var req CreateUserRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "INVALID_REQUEST",
			"Invalid request body", ctx.RequestID)
		return
	}

	store.mu.Lock()
	defer store.mu.Unlock()

	// Check for duplicate email
	for _, u := range store.users {
		if u.Email == req.Email {
			writeError(w, http.StatusBadRequest, "EMAIL_EXISTS",
				fmt.Sprintf("User with email '%s' already exists", req.Email), ctx.RequestID)
			return
		}
	}

	user := User{
		ID:        uuid.New().String(),
		Name:      req.Name,
		Email:     req.Email,
		CreatedAt: time.Now().UTC().Format(time.RFC3339),
	}
	store.users[user.ID] = user

	log.Printf("[%s] Created user %s", ctx.RequestID, user.ID)
	writeJSON(w, http.StatusCreated, user)
}

func updateUserHandler(w http.ResponseWriter, r *http.Request) {
	ctx := getRequestContext(r)
	userID := extractUserID(r.URL.Path)
	log.Printf("[%s] Updating user %s, caller: %+v", ctx.RequestID, userID, ctx.Caller)

	var req UpdateUserRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "INVALID_REQUEST",
			"Invalid request body", ctx.RequestID)
		return
	}

	store.mu.Lock()
	defer store.mu.Unlock()

	user, exists := store.users[userID]
	if !exists {
		writeError(w, http.StatusNotFound, "USER_NOT_FOUND",
			fmt.Sprintf("User with ID '%s' not found", userID), ctx.RequestID)
		return
	}

	if req.Name != nil {
		user.Name = *req.Name
	}
	if req.Email != nil {
		user.Email = *req.Email
	}
	store.users[userID] = user

	log.Printf("[%s] Updated user %s", ctx.RequestID, userID)
	writeJSON(w, http.StatusOK, user)
}

func deleteUserHandler(w http.ResponseWriter, r *http.Request) {
	ctx := getRequestContext(r)
	userID := extractUserID(r.URL.Path)
	log.Printf("[%s] Deleting user %s, caller: %+v", ctx.RequestID, userID, ctx.Caller)

	store.mu.Lock()
	defer store.mu.Unlock()

	if _, exists := store.users[userID]; !exists {
		writeError(w, http.StatusNotFound, "USER_NOT_FOUND",
			fmt.Sprintf("User with ID '%s' not found", userID), ctx.RequestID)
		return
	}

	delete(store.users, userID)
	log.Printf("[%s] Deleted user %s", ctx.RequestID, userID)
	w.WriteHeader(http.StatusNoContent)
}

func usersHandler(w http.ResponseWriter, r *http.Request) {
	// Handle /users and /users/{id}
	switch r.Method {
	case http.MethodGet:
		if r.URL.Path == "/users" {
			listUsersHandler(w, r)
		} else {
			getUserHandler(w, r)
		}
	case http.MethodPost:
		if r.URL.Path == "/users" {
			createUserHandler(w, r)
		} else {
			http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		}
	case http.MethodPut:
		if r.URL.Path != "/users" {
			updateUserHandler(w, r)
		} else {
			http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		}
	case http.MethodDelete:
		if r.URL.Path != "/users" {
			deleteUserHandler(w, r)
		} else {
			http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		}
	default:
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
	}
}

// =============================================================================
// Main
// =============================================================================

func main() {
	port := os.Getenv("PORT")
	if port == "" {
		port = "3000"
	}

	host := os.Getenv("HOST")
	if host == "" {
		host = "0.0.0.0"
	}

	http.HandleFunc("/health", healthHandler)
	http.HandleFunc("/users", usersHandler)
	http.HandleFunc("/users/", usersHandler)

	addr := fmt.Sprintf("%s:%s", host, port)
	log.Printf("Starting Go example service on %s", addr)

	if err := http.ListenAndServe(addr, nil); err != nil {
		log.Fatalf("Server failed: %v", err)
	}
}
