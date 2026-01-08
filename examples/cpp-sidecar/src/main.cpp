/**
 * C++ Example Service with Archimedes Sidecar
 *
 * This service demonstrates how to build a C++ microservice that works with
 * the Archimedes sidecar for contract validation, authorization, and observability.
 */

#include <httplib.h>
#include <nlohmann/json.hpp>

#include <chrono>
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include <map>
#include <mutex>
#include <optional>
#include <sstream>
#include <string>

using json = nlohmann::json;

// =============================================================================
// Types
// =============================================================================

struct CallerIdentity {
    std::string type;
    std::optional<std::string> id;
    std::optional<std::string> trust_domain;
    std::optional<std::string> path;
    std::optional<std::string> user_id;
    std::optional<std::vector<std::string>> roles;
    std::optional<std::string> key_id;
};

struct User {
    std::string id;
    std::string name;
    std::string email;
    std::string created_at;
};

struct RequestContext {
    std::string request_id;
    std::optional<CallerIdentity> caller;
    std::optional<std::string> operation_id;
};

// JSON serialization for User
void to_json(json& j, const User& u) {
    j = json{
        {"id", u.id},
        {"name", u.name},
        {"email", u.email},
        {"created_at", u.created_at}
    };
}

// =============================================================================
// Helpers
// =============================================================================

std::string generate_uuid() {
    // Simple UUID-like string generation
    static std::atomic<uint64_t> counter{0};
    auto now = std::chrono::system_clock::now().time_since_epoch().count();
    std::stringstream ss;
    ss << std::hex << now << "-" << ++counter;
    return ss.str();
}

std::string now_iso() {
    auto now = std::chrono::system_clock::now();
    auto time = std::chrono::system_clock::to_time_t(now);
    std::stringstream ss;
    ss << std::put_time(std::gmtime(&time), "%Y-%m-%dT%H:%M:%SZ");
    return ss.str();
}

std::optional<CallerIdentity> parse_caller_identity(const std::string& header_value) {
    if (header_value.empty()) {
        return std::nullopt;
    }
    try {
        auto j = json::parse(header_value);
        CallerIdentity caller;
        caller.type = j.value("type", "");
        if (j.contains("id")) caller.id = j["id"].get<std::string>();
        if (j.contains("trust_domain")) caller.trust_domain = j["trust_domain"].get<std::string>();
        if (j.contains("path")) caller.path = j["path"].get<std::string>();
        if (j.contains("user_id")) caller.user_id = j["user_id"].get<std::string>();
        if (j.contains("key_id")) caller.key_id = j["key_id"].get<std::string>();
        if (j.contains("roles")) {
            caller.roles = j["roles"].get<std::vector<std::string>>();
        }
        return caller;
    } catch (const std::exception& e) {
        std::cerr << "Failed to parse caller identity: " << e.what() << std::endl;
        return std::nullopt;
    }
}

RequestContext get_request_context(const httplib::Request& req) {
    RequestContext ctx;
    
    if (req.has_header("X-Request-Id")) {
        ctx.request_id = req.get_header_value("X-Request-Id");
    } else {
        ctx.request_id = generate_uuid();
    }
    
    if (req.has_header("X-Caller-Identity")) {
        ctx.caller = parse_caller_identity(req.get_header_value("X-Caller-Identity"));
    }
    
    if (req.has_header("X-Operation-Id")) {
        ctx.operation_id = req.get_header_value("X-Operation-Id");
    }
    
    return ctx;
}

json error_response(const std::string& code, const std::string& message, 
                   const std::optional<std::string>& request_id = std::nullopt) {
    json j = {
        {"code", code},
        {"message", message}
    };
    if (request_id.has_value()) {
        j["request_id"] = request_id.value();
    }
    return j;
}

// =============================================================================
// User Store
// =============================================================================

class UserStore {
public:
    UserStore() {
        // Seed with initial users
        users_["1"] = User{"1", "Alice Smith", "alice@example.com", "2026-01-01T00:00:00Z"};
        users_["2"] = User{"2", "Bob Johnson", "bob@example.com", "2026-01-02T00:00:00Z"};
    }
    
    std::vector<User> list() const {
        std::lock_guard<std::mutex> lock(mutex_);
        std::vector<User> result;
        for (const auto& [_, user] : users_) {
            result.push_back(user);
        }
        return result;
    }
    
    std::optional<User> get(const std::string& id) const {
        std::lock_guard<std::mutex> lock(mutex_);
        auto it = users_.find(id);
        if (it != users_.end()) {
            return it->second;
        }
        return std::nullopt;
    }
    
    std::optional<std::string> find_by_email(const std::string& email) const {
        std::lock_guard<std::mutex> lock(mutex_);
        for (const auto& [id, user] : users_) {
            if (user.email == email) {
                return id;
            }
        }
        return std::nullopt;
    }
    
    User create(const std::string& name, const std::string& email) {
        std::lock_guard<std::mutex> lock(mutex_);
        User user;
        user.id = generate_uuid();
        user.name = name;
        user.email = email;
        user.created_at = now_iso();
        users_[user.id] = user;
        return user;
    }
    
    std::optional<User> update(const std::string& id, 
                               const std::optional<std::string>& name,
                               const std::optional<std::string>& email) {
        std::lock_guard<std::mutex> lock(mutex_);
        auto it = users_.find(id);
        if (it == users_.end()) {
            return std::nullopt;
        }
        if (name.has_value()) {
            it->second.name = name.value();
        }
        if (email.has_value()) {
            it->second.email = email.value();
        }
        return it->second;
    }
    
    bool remove(const std::string& id) {
        std::lock_guard<std::mutex> lock(mutex_);
        return users_.erase(id) > 0;
    }
    
private:
    mutable std::mutex mutex_;
    std::map<std::string, User> users_;
};

// =============================================================================
// Main
// =============================================================================

int main() {
    // Configuration
    const char* port_env = std::getenv("PORT");
    int port = port_env ? std::atoi(port_env) : 3000;
    
    const char* host_env = std::getenv("HOST");
    std::string host = host_env ? host_env : "0.0.0.0";
    
    // Create server and user store
    httplib::Server svr;
    UserStore store;
    
    // Health endpoint
    svr.Get("/health", [](const httplib::Request&, httplib::Response& res) {
        json response = {
            {"status", "healthy"},
            {"service", "example-cpp"},
            {"timestamp", now_iso()}
        };
        res.set_content(response.dump(), "application/json");
    });
    
    // List users
    svr.Get("/users", [&store](const httplib::Request& req, httplib::Response& res) {
        auto ctx = get_request_context(req);
        std::cout << "[" << ctx.request_id << "] Listing users" << std::endl;
        
        auto users = store.list();
        json response = {
            {"users", json::array()},
            {"total", users.size()}
        };
        for (const auto& user : users) {
            response["users"].push_back(user);
        }
        res.set_content(response.dump(), "application/json");
    });
    
    // Get user by ID
    svr.Get(R"(/users/(\w+))", [&store](const httplib::Request& req, httplib::Response& res) {
        auto ctx = get_request_context(req);
        std::string user_id = req.matches[1];
        std::cout << "[" << ctx.request_id << "] Getting user " << user_id << std::endl;
        
        auto user = store.get(user_id);
        if (!user.has_value()) {
            res.status = 404;
            res.set_content(
                error_response("USER_NOT_FOUND", 
                              "User with ID '" + user_id + "' not found",
                              ctx.request_id).dump(),
                "application/json"
            );
            return;
        }
        
        json response;
        to_json(response, user.value());
        res.set_content(response.dump(), "application/json");
    });
    
    // Create user
    svr.Post("/users", [&store](const httplib::Request& req, httplib::Response& res) {
        auto ctx = get_request_context(req);
        std::cout << "[" << ctx.request_id << "] Creating user" << std::endl;
        
        json body;
        try {
            body = json::parse(req.body);
        } catch (const std::exception& e) {
            res.status = 400;
            res.set_content(
                error_response("INVALID_REQUEST", "Invalid JSON body", ctx.request_id).dump(),
                "application/json"
            );
            return;
        }
        
        if (!body.contains("name") || !body.contains("email")) {
            res.status = 400;
            res.set_content(
                error_response("INVALID_REQUEST", "Name and email are required", ctx.request_id).dump(),
                "application/json"
            );
            return;
        }
        
        std::string name = body["name"].get<std::string>();
        std::string email = body["email"].get<std::string>();
        
        // Check for duplicate email
        if (store.find_by_email(email).has_value()) {
            res.status = 400;
            res.set_content(
                error_response("EMAIL_EXISTS", 
                              "User with email '" + email + "' already exists",
                              ctx.request_id).dump(),
                "application/json"
            );
            return;
        }
        
        auto user = store.create(name, email);
        std::cout << "[" << ctx.request_id << "] Created user " << user.id << std::endl;
        
        json response;
        to_json(response, user);
        res.status = 201;
        res.set_content(response.dump(), "application/json");
    });
    
    // Update user
    svr.Put(R"(/users/(\w+))", [&store](const httplib::Request& req, httplib::Response& res) {
        auto ctx = get_request_context(req);
        std::string user_id = req.matches[1];
        std::cout << "[" << ctx.request_id << "] Updating user " << user_id << std::endl;
        
        json body;
        try {
            body = json::parse(req.body);
        } catch (const std::exception& e) {
            res.status = 400;
            res.set_content(
                error_response("INVALID_REQUEST", "Invalid JSON body", ctx.request_id).dump(),
                "application/json"
            );
            return;
        }
        
        std::optional<std::string> name, email;
        if (body.contains("name")) {
            name = body["name"].get<std::string>();
        }
        if (body.contains("email")) {
            email = body["email"].get<std::string>();
        }
        
        auto user = store.update(user_id, name, email);
        if (!user.has_value()) {
            res.status = 404;
            res.set_content(
                error_response("USER_NOT_FOUND",
                              "User with ID '" + user_id + "' not found",
                              ctx.request_id).dump(),
                "application/json"
            );
            return;
        }
        
        std::cout << "[" << ctx.request_id << "] Updated user " << user_id << std::endl;
        
        json response;
        to_json(response, user.value());
        res.set_content(response.dump(), "application/json");
    });
    
    // Delete user
    svr.Delete(R"(/users/(\w+))", [&store](const httplib::Request& req, httplib::Response& res) {
        auto ctx = get_request_context(req);
        std::string user_id = req.matches[1];
        std::cout << "[" << ctx.request_id << "] Deleting user " << user_id << std::endl;
        
        if (!store.remove(user_id)) {
            res.status = 404;
            res.set_content(
                error_response("USER_NOT_FOUND",
                              "User with ID '" + user_id + "' not found",
                              ctx.request_id).dump(),
                "application/json"
            );
            return;
        }
        
        std::cout << "[" << ctx.request_id << "] Deleted user " << user_id << std::endl;
        res.status = 204;
    });
    
    std::cout << "C++ example service listening on " << host << ":" << port << std::endl;
    svr.listen(host, port);
    
    return 0;
}
