/**
 * Archimedes C++ Bindings
 *
 * Modern C++17 wrapper for the Archimedes contract-first HTTP server framework.
 *
 * Usage:
 * ```cpp
 * #include <archimedes/archimedes.hpp>
 *
 * int main() {
 *     archimedes::App app{"contract.json"};
 *
 *     app.operation("listUsers", [](const archimedes::Request& req) {
 *         return archimedes::Response::json({{"users", {}}});
 *     });
 *
 *     app.run(8080);
 * }
 * ```
 *
 * @copyright 2024 Themis Platform Team
 * @license Apache-2.0
 */

#ifndef ARCHIMEDES_HPP
#define ARCHIMEDES_HPP

#include <archimedes/config.hpp>
#include <archimedes/error.hpp>
#include <archimedes/request.hpp>
#include <archimedes/response.hpp>

#include <functional>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <unordered_map>
#include <vector>

// Include the C header (expects archimedes.h in include path)
extern "C" {
#include "archimedes.h"
}

namespace archimedes {

/**
 * Archimedes Application
 *
 * RAII wrapper for the Archimedes server. Automatically manages resources
 * and provides a modern C++ API for registering handlers.
 *
 * Example:
 * ```cpp
 * archimedes::App app{archimedes::Config{}.contract_path("contract.json")};
 *
 * app.operation("getUser", [](const Request& req) -> Response {
 *     auto user_id = req.path_param("userId");
 *     // ... fetch user ...
 *     return Response::json({{"id", user_id}, {"name", "Alice"}});
 * });
 *
 * app.run();  // Blocks until shutdown
 * ```
 */
class App {
public:
    /// Handler function type: takes Request, returns Response
    using Handler = std::function<Response(const Request&)>;

    /**
     * Create an application with a contract file path.
     *
     * @param contract_path Path to the Themis contract JSON file
     * @throws Error if the contract cannot be loaded
     */
    explicit App(std::string_view contract_path);

    /**
     * Create an application with full configuration.
     *
     * @param config Configuration options
     * @throws Error if initialization fails
     */
    explicit App(const Config& config);

    /// Destructor - stops the server if running
    ~App();

    // Non-copyable
    App(const App&) = delete;
    App& operator=(const App&) = delete;

    // Movable
    App(App&& other) noexcept;
    App& operator=(App&& other) noexcept;

    /**
     * Register a handler for an operation.
     *
     * The handler is called when a request matches the operation's route.
     * Request validation and authorization happen before the handler is called.
     *
     * @param operation_id The operation ID from the contract
     * @param handler The handler function
     * @return Reference to this App for chaining
     * @throws Error if registration fails
     *
     * Example:
     * ```cpp
     * app.operation("listUsers", [](const Request& req) {
     *     return Response::json({{"users", {}}});
     * })
     * .operation("getUser", [](const Request& req) {
     *     return Response::json({{"id", req.path_param("userId")}});
     * });
     * ```
     */
    App& operation(std::string_view operation_id, Handler handler);

    /**
     * Start the server and block until shutdown.
     *
     * @throws Error if the server fails to start
     */
    void run();

    /**
     * Start the server on a specific port and block until shutdown.
     *
     * @param port Port number to listen on
     * @throws Error if the server fails to start
     */
    void run(uint16_t port);

    /**
     * Request graceful shutdown.
     *
     * This signals the server to stop accepting new connections and
     * wait for existing requests to complete.
     */
    void stop();

    /**
     * Check if the server is currently running.
     *
     * @return true if running, false otherwise
     */
    [[nodiscard]] bool is_running() const;

    /**
     * Get the Archimedes version string.
     *
     * @return Version string (e.g., "0.1.0")
     */
    [[nodiscard]] static std::string_view version();

private:
    /// Internal: dispatch from C callback to C++ handler
    static archimedes_response_data dispatch_handler(
        const archimedes_request_context* ctx,
        const uint8_t* body,
        size_t body_len,
        void* user_data
    );

    /// Internal: convert C context to C++ Request
    static Request make_request(
        const archimedes_request_context* ctx,
        const uint8_t* body,
        size_t body_len
    );

    /// Internal: convert C++ Response to C response data
    static archimedes_response_data make_response_data(const Response& response);

    /// Application handle
    archimedes_app* app_ = nullptr;

    /// Port override (0 means use config)
    uint16_t port_override_ = 0;

    /// Registered handlers (keyed by operation_id)
    std::unordered_map<std::string, Handler> handlers_;
};

// ============================================================================
// Inline implementations
// ============================================================================

inline App::App(std::string_view contract_path)
    : App(Config{}.contract_path(std::string(contract_path))) {}

inline App::App(const Config& config) {
    archimedes_config c_config = config.to_c_config();
    app_ = archimedes_new(&c_config);
    if (!app_) {
        throw Error(archimedes_last_error());
    }
}

inline App::~App() {
    if (app_) {
        archimedes_free(app_);
        app_ = nullptr;
    }
}

inline App::App(App&& other) noexcept
    : app_(other.app_)
    , port_override_(other.port_override_)
    , handlers_(std::move(other.handlers_)) {
    other.app_ = nullptr;
}

inline App& App::operator=(App&& other) noexcept {
    if (this != &other) {
        if (app_) {
            archimedes_free(app_);
        }
        app_ = other.app_;
        port_override_ = other.port_override_;
        handlers_ = std::move(other.handlers_);
        other.app_ = nullptr;
    }
    return *this;
}

inline App& App::operation(std::string_view operation_id, Handler handler) {
    std::string op_id_str(operation_id);

    // Store handler
    handlers_[op_id_str] = std::move(handler);

    // Register with C API (user_data points to the handler in our map)
    auto it = handlers_.find(op_id_str);
    auto error = archimedes_register_handler(
        app_,
        op_id_str.c_str(),
        dispatch_handler,
        const_cast<Handler*>(&it->second)
    );

    if (error != ARCHIMEDES_ERROR_OK) {
        handlers_.erase(op_id_str);
        throw Error(archimedes_last_error());
    }

    return *this;
}

inline void App::run() {
    auto error = archimedes_run(app_);
    if (error != ARCHIMEDES_ERROR_OK) {
        throw Error(archimedes_last_error());
    }
}

inline void App::run(uint16_t port) {
    port_override_ = port;
    run();
}

inline void App::stop() {
    archimedes_stop(app_);
}

inline bool App::is_running() const {
    return archimedes_is_running(app_) != 0;
}

inline std::string_view App::version() {
    return archimedes_version();
}

inline archimedes_response_data App::dispatch_handler(
    const archimedes_request_context* ctx,
    const uint8_t* body,
    size_t body_len,
    void* user_data
) {
    try {
        auto* handler = static_cast<Handler*>(user_data);
        Request request = make_request(ctx, body, body_len);
        Response response = (*handler)(request);
        return make_response_data(response);
    } catch (const std::exception& e) {
        // Return 500 error on exception
        archimedes_response_data error_response = {};
        error_response.status_code = 500;
        // Note: We need to allocate this string - for now use a static
        static thread_local std::string error_body;
        error_body = R"({"error":")" + std::string(e.what()) + R"("})";
        error_response.body = error_body.c_str();
        error_response.body_len = error_body.size();
        error_response.body_owned = false;
        return error_response;
    } catch (...) {
        archimedes_response_data error_response = {};
        error_response.status_code = 500;
        error_response.body = R"({"error":"Unknown error"})";
        error_response.body_len = 24;
        error_response.body_owned = false;
        return error_response;
    }
}

inline Request App::make_request(
    const archimedes_request_context* ctx,
    const uint8_t* body,
    size_t body_len
) {
    Request req;
    req.set_request_id(ctx->request_id ? ctx->request_id : "");
    req.set_trace_id(ctx->trace_id ? ctx->trace_id : "");
    req.set_span_id(ctx->span_id ? ctx->span_id : "");
    req.set_operation_id(ctx->operation_id ? ctx->operation_id : "");
    req.set_method(ctx->method ? ctx->method : "");
    req.set_path(ctx->path ? ctx->path : "");
    req.set_query(ctx->query ? ctx->query : "");

    // Parse caller identity
    if (ctx->caller_identity_json && ctx->caller_identity_json[0]) {
        req.set_caller_identity_json(ctx->caller_identity_json);
    }

    // Copy path parameters
    for (size_t i = 0; i < ctx->path_params_count; ++i) {
        req.add_path_param(
            ctx->path_param_names[i],
            ctx->path_param_values[i]
        );
    }

    // Copy headers
    for (size_t i = 0; i < ctx->headers_count; ++i) {
        req.add_header(
            ctx->header_names[i],
            ctx->header_values[i]
        );
    }

    // Copy body
    if (body && body_len > 0) {
        req.set_body(std::vector<uint8_t>(body, body + body_len));
    }

    return req;
}

inline archimedes_response_data App::make_response_data(const Response& response) {
    archimedes_response_data data = {};
    data.status_code = static_cast<int32_t>(response.status_code());

    // Store body - the Response owns this memory
    const auto& body = response.body();
    if (!body.empty()) {
        // We need to copy the body data - use thread_local storage
        static thread_local std::string body_storage;
        body_storage.assign(body.begin(), body.end());
        data.body = body_storage.c_str();
        data.body_len = body_storage.size();
        data.body_owned = false;
    }

    // Content type
    auto content_type = response.content_type();
    if (!content_type.empty()) {
        static thread_local std::string content_type_storage;
        content_type_storage = std::string(content_type);
        data.content_type = content_type_storage.c_str();
    }

    return data;
}

} // namespace archimedes

#endif // ARCHIMEDES_HPP
