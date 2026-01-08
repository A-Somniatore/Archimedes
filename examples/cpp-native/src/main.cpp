/**
 * C++ Native Example with Archimedes
 *
 * This example demonstrates using Archimedes C++ bindings directly
 * (no sidecar required).
 */

#include <archimedes/archimedes.hpp>

#include <atomic>
#include <chrono>
#include <iomanip>
#include <iostream>
#include <map>
#include <mutex>
#include <sstream>
#include <string>
#include <vector>

using namespace archimedes;

// =============================================================================
// Types
// =============================================================================

struct User {
    std::string id;
    std::string name;
    std::string email;
    std::string created_at;

    std::string to_json() const {
        std::ostringstream ss;
        ss << R"({"id":")" << id
           << R"(","name":")" << name
           << R"(","email":")" << email
           << R"(","created_at":")" << created_at << R"("})";
        return ss.str();
    }
};

// =============================================================================
// Helpers
// =============================================================================

std::string generate_uuid() {
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

// =============================================================================
// In-Memory Database
// =============================================================================

class UserDatabase {
public:
    UserDatabase() {
        // Add some initial users
        create("Alice", "alice@example.com");
        create("Bob", "bob@example.com");
    }

    std::vector<User> list() const {
        std::lock_guard<std::mutex> lock(mutex_);
        std::vector<User> users;
        for (const auto& [id, user] : users_) {
            users.push_back(user);
        }
        return users;
    }

    std::optional<User> get(const std::string& id) const {
        std::lock_guard<std::mutex> lock(mutex_);
        auto it = users_.find(id);
        if (it != users_.end()) {
            return it->second;
        }
        return std::nullopt;
    }

    User create(const std::string& name, const std::string& email) {
        std::lock_guard<std::mutex> lock(mutex_);
        User user{
            .id = generate_uuid(),
            .name = name,
            .email = email,
            .created_at = now_iso()
        };
        users_[user.id] = user;
        return user;
    }

    bool update(const std::string& id, const std::string& name, const std::string& email) {
        std::lock_guard<std::mutex> lock(mutex_);
        auto it = users_.find(id);
        if (it == users_.end()) {
            return false;
        }
        it->second.name = name;
        it->second.email = email;
        return true;
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
// Simple JSON Parser (for demo purposes)
// =============================================================================

std::optional<std::string> extract_json_string(const std::string& json, const std::string& key) {
    auto pos = json.find("\"" + key + "\"");
    if (pos == std::string::npos) return std::nullopt;

    auto colon = json.find(':', pos);
    if (colon == std::string::npos) return std::nullopt;

    auto start = json.find('"', colon + 1);
    if (start == std::string::npos) return std::nullopt;

    auto end = json.find('"', start + 1);
    if (end == std::string::npos) return std::nullopt;

    return json.substr(start + 1, end - start - 1);
}

// =============================================================================
// Main
// =============================================================================

int main(int argc, char* argv[]) {
    try {
        // Parse command line args
        std::string contract_path = "../contract.json";
        uint16_t port = 8080;

        for (int i = 1; i < argc; ++i) {
            std::string arg = argv[i];
            if (arg == "--contract" && i + 1 < argc) {
                contract_path = argv[++i];
            } else if (arg == "--port" && i + 1 < argc) {
                port = static_cast<uint16_t>(std::stoi(argv[++i]));
            }
        }

        std::cout << "Starting C++ Native Example\n";
        std::cout << "  Contract: " << contract_path << "\n";
        std::cout << "  Port: " << port << "\n";
        std::cout << "  Archimedes version: " << App::version() << "\n";

        // Create the database
        UserDatabase db;

        // Configure Archimedes
        auto config = Config{}
            .contract_path(contract_path)
            .listen_port(port)
            .service_name("cpp-native-example")
            .enable_tracing(true)
            .enable_validation(true);

        App app{config};

        // =====================================================================
        // Health Check
        // =====================================================================
        app.operation("healthCheck", [](const Request& req) {
            return Response::json({
                {"status", "healthy"},
                {"service", "cpp-native-example"},
                {"version", std::string(App::version())}
            });
        });

        // =====================================================================
        // List Users
        // =====================================================================
        app.operation("listUsers", [&db](const Request& req) {
            auto users = db.list();

            // Build JSON array
            std::ostringstream ss;
            ss << "[";
            bool first = true;
            for (const auto& user : users) {
                if (!first) ss << ",";
                first = false;
                ss << user.to_json();
            }
            ss << "]";

            return Response::json_raw(R"({"users":)" + ss.str() + "}");
        });

        // =====================================================================
        // Get User
        // =====================================================================
        app.operation("getUser", [&db](const Request& req) {
            auto user_id = req.path_param_or_throw("userId");
            auto user = db.get(std::string(user_id));

            if (!user) {
                return response::not_found("User not found");
            }

            return Response::json_raw(user->to_json());
        });

        // =====================================================================
        // Create User
        // =====================================================================
        app.operation("createUser", [&db](const Request& req) {
            auto body = std::string(req.body_str());

            auto name = extract_json_string(body, "name");
            auto email = extract_json_string(body, "email");

            if (!name || !email) {
                return response::bad_request("Missing name or email");
            }

            auto user = db.create(*name, *email);

            return Response::json_raw(user.to_json())
                .status(Response::Status::Created);
        });

        // =====================================================================
        // Update User
        // =====================================================================
        app.operation("updateUser", [&db](const Request& req) {
            auto user_id = req.path_param_or_throw("userId");
            auto body = std::string(req.body_str());

            auto name = extract_json_string(body, "name");
            auto email = extract_json_string(body, "email");

            if (!name || !email) {
                return response::bad_request("Missing name or email");
            }

            if (!db.update(std::string(user_id), *name, *email)) {
                return response::not_found("User not found");
            }

            auto user = db.get(std::string(user_id));
            return Response::json_raw(user->to_json());
        });

        // =====================================================================
        // Delete User
        // =====================================================================
        app.operation("deleteUser", [&db](const Request& req) {
            auto user_id = req.path_param_or_throw("userId");

            if (!db.remove(std::string(user_id))) {
                return response::not_found("User not found");
            }

            return response::no_content();
        });

        // =====================================================================
        // Start Server
        // =====================================================================
        std::cout << "\nListening on http://0.0.0.0:" << port << "\n";
        std::cout << "Press Ctrl+C to stop\n\n";

        app.run();

        return 0;
    } catch (const Error& e) {
        std::cerr << "Archimedes error: " << e.what() << "\n";
        std::cerr << "Error code: " << e.code_name() << "\n";
        return 1;
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << "\n";
        return 1;
    }
}
