/**
 * Archimedes Request
 *
 * @copyright 2024 Themis Platform Team
 * @license Apache-2.0
 */

#ifndef ARCHIMEDES_REQUEST_HPP
#define ARCHIMEDES_REQUEST_HPP

#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_map>
#include <vector>

namespace archimedes {

/**
 * Caller identity information.
 *
 * Represents the authenticated caller making the request.
 * Identity is validated and provided by the authorization middleware.
 */
class CallerIdentity {
public:
    /// Identity type (e.g., "service", "user", "api_key")
    [[nodiscard]] std::string_view type() const { return type_; }

    /// Service/user ID (optional)
    [[nodiscard]] std::optional<std::string_view> id() const {
        return id_.empty() ? std::nullopt : std::make_optional<std::string_view>(id_);
    }

    /// SPIFFE trust domain (for service identities)
    [[nodiscard]] std::optional<std::string_view> trust_domain() const {
        return trust_domain_.empty() ? std::nullopt : std::make_optional<std::string_view>(trust_domain_);
    }

    /// SPIFFE path (for service identities)
    [[nodiscard]] std::optional<std::string_view> path() const {
        return path_.empty() ? std::nullopt : std::make_optional<std::string_view>(path_);
    }

    /// User ID (for user identities)
    [[nodiscard]] std::optional<std::string_view> user_id() const {
        return user_id_.empty() ? std::nullopt : std::make_optional<std::string_view>(user_id_);
    }

    /// User roles (for user identities)
    [[nodiscard]] const std::vector<std::string>& roles() const { return roles_; }

    /// Check if identity has a specific role
    [[nodiscard]] bool has_role(std::string_view role) const {
        for (const auto& r : roles_) {
            if (r == role) return true;
        }
        return false;
    }

    /// API key ID (for API key identities)
    [[nodiscard]] std::optional<std::string_view> key_id() const {
        return key_id_.empty() ? std::nullopt : std::make_optional<std::string_view>(key_id_);
    }

    // Builder methods for internal use
    void set_type(std::string type) { type_ = std::move(type); }
    void set_id(std::string id) { id_ = std::move(id); }
    void set_trust_domain(std::string domain) { trust_domain_ = std::move(domain); }
    void set_path(std::string path) { path_ = std::move(path); }
    void set_user_id(std::string user_id) { user_id_ = std::move(user_id); }
    void set_roles(std::vector<std::string> roles) { roles_ = std::move(roles); }
    void set_key_id(std::string key_id) { key_id_ = std::move(key_id); }

private:
    std::string type_;
    std::string id_;
    std::string trust_domain_;
    std::string path_;
    std::string user_id_;
    std::vector<std::string> roles_;
    std::string key_id_;
};

/**
 * HTTP Request wrapper.
 *
 * Provides read-only access to request data including:
 * - Request metadata (ID, trace info, operation)
 * - HTTP method, path, query
 * - Path parameters and headers
 * - Request body
 * - Caller identity
 *
 * All validation has already been performed before the handler receives
 * the request.
 */
class Request {
public:
    Request() = default;

    // ========================================================================
    // Request Metadata
    // ========================================================================

    /// Unique request ID (UUID v7)
    [[nodiscard]] std::string_view request_id() const { return request_id_; }

    /// OpenTelemetry trace ID
    [[nodiscard]] std::string_view trace_id() const { return trace_id_; }

    /// OpenTelemetry span ID
    [[nodiscard]] std::string_view span_id() const { return span_id_; }

    /// Operation ID from contract
    [[nodiscard]] std::string_view operation_id() const { return operation_id_; }

    // ========================================================================
    // HTTP Request Data
    // ========================================================================

    /// HTTP method (GET, POST, etc.)
    [[nodiscard]] std::string_view method() const { return method_; }

    /// Request path (e.g., "/users/123")
    [[nodiscard]] std::string_view path() const { return path_; }

    /// Query string (without leading ?)
    [[nodiscard]] std::string_view query() const { return query_; }

    /// Request body as bytes
    [[nodiscard]] const std::vector<uint8_t>& body() const { return body_; }

    /// Request body as string (assumes UTF-8)
    [[nodiscard]] std::string_view body_str() const {
        return {reinterpret_cast<const char*>(body_.data()), body_.size()};
    }

    /// Check if body is present
    [[nodiscard]] bool has_body() const { return !body_.empty(); }

    // ========================================================================
    // Path Parameters
    // ========================================================================

    /// Get a path parameter by name
    [[nodiscard]] std::optional<std::string_view> path_param(std::string_view name) const {
        auto it = path_params_.find(std::string(name));
        if (it != path_params_.end()) {
            return it->second;
        }
        return std::nullopt;
    }

    /// Get a path parameter or throw if not found
    [[nodiscard]] std::string_view path_param_or_throw(std::string_view name) const {
        auto value = path_param(name);
        if (!value) {
            throw std::runtime_error("Missing path parameter: " + std::string(name));
        }
        return *value;
    }

    /// Get all path parameters
    [[nodiscard]] const std::unordered_map<std::string, std::string>& path_params() const {
        return path_params_;
    }

    // ========================================================================
    // Headers
    // ========================================================================

    /// Get a header by name (case-insensitive)
    [[nodiscard]] std::optional<std::string_view> header(std::string_view name) const {
        // Headers are stored lowercase
        std::string lower_name;
        lower_name.reserve(name.size());
        for (char c : name) {
            lower_name.push_back(static_cast<char>(std::tolower(static_cast<unsigned char>(c))));
        }
        auto it = headers_.find(lower_name);
        if (it != headers_.end()) {
            return it->second;
        }
        return std::nullopt;
    }

    /// Get all headers
    [[nodiscard]] const std::unordered_map<std::string, std::string>& headers() const {
        return headers_;
    }

    // ========================================================================
    // Caller Identity
    // ========================================================================

    /// Get the authenticated caller identity
    [[nodiscard]] const CallerIdentity& caller() const { return caller_; }

    /// Check if caller is authenticated
    [[nodiscard]] bool has_caller() const { return !caller_.type().empty(); }

    // ========================================================================
    // Internal: Builder methods
    // ========================================================================

    void set_request_id(std::string id) { request_id_ = std::move(id); }
    void set_trace_id(std::string id) { trace_id_ = std::move(id); }
    void set_span_id(std::string id) { span_id_ = std::move(id); }
    void set_operation_id(std::string id) { operation_id_ = std::move(id); }
    void set_method(std::string m) { method_ = std::move(m); }
    void set_path(std::string p) { path_ = std::move(p); }
    void set_query(std::string q) { query_ = std::move(q); }
    void set_body(std::vector<uint8_t> b) { body_ = std::move(b); }
    void set_caller_identity_json(const std::string& json);

    void add_path_param(std::string name, std::string value) {
        path_params_[std::move(name)] = std::move(value);
    }

    void add_header(std::string name, std::string value) {
        // Store headers lowercase
        for (char& c : name) {
            c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
        }
        headers_[std::move(name)] = std::move(value);
    }

private:
    std::string request_id_;
    std::string trace_id_;
    std::string span_id_;
    std::string operation_id_;
    std::string method_;
    std::string path_;
    std::string query_;
    std::vector<uint8_t> body_;
    std::unordered_map<std::string, std::string> path_params_;
    std::unordered_map<std::string, std::string> headers_;
    CallerIdentity caller_;
};

// ============================================================================
// Inline implementations
// ============================================================================

inline void Request::set_caller_identity_json(const std::string& json) {
    // Simple JSON parsing for caller identity
    // In a real implementation, use a proper JSON library
    if (json.empty()) return;

    // Extract "type" field
    auto type_pos = json.find("\"type\"");
    if (type_pos != std::string::npos) {
        auto start = json.find(':', type_pos);
        if (start != std::string::npos) {
            start = json.find('"', start + 1);
            if (start != std::string::npos) {
                auto end = json.find('"', start + 1);
                if (end != std::string::npos) {
                    caller_.set_type(json.substr(start + 1, end - start - 1));
                }
            }
        }
    }

    // Extract "id" field
    auto id_pos = json.find("\"id\"");
    if (id_pos != std::string::npos) {
        auto start = json.find(':', id_pos);
        if (start != std::string::npos) {
            start = json.find('"', start + 1);
            if (start != std::string::npos) {
                auto end = json.find('"', start + 1);
                if (end != std::string::npos) {
                    caller_.set_id(json.substr(start + 1, end - start - 1));
                }
            }
        }
    }

    // Extract "user_id" field
    auto user_id_pos = json.find("\"user_id\"");
    if (user_id_pos != std::string::npos) {
        auto start = json.find(':', user_id_pos);
        if (start != std::string::npos) {
            start = json.find('"', start + 1);
            if (start != std::string::npos) {
                auto end = json.find('"', start + 1);
                if (end != std::string::npos) {
                    caller_.set_user_id(json.substr(start + 1, end - start - 1));
                }
            }
        }
    }
}

} // namespace archimedes

#endif // ARCHIMEDES_REQUEST_HPP
