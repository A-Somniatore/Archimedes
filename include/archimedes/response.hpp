/**
 * Archimedes Response
 *
 * @copyright 2024 Themis Platform Team
 * @license Apache-2.0
 */

#ifndef ARCHIMEDES_RESPONSE_HPP
#define ARCHIMEDES_RESPONSE_HPP

#include <cstdint>
#include <initializer_list>
#include <sstream>
#include <string>
#include <string_view>
#include <unordered_map>
#include <utility>
#include <vector>

namespace archimedes {

/**
 * HTTP Response builder.
 *
 * Provides a fluent API for building HTTP responses with support for
 * JSON serialization.
 *
 * Examples:
 * ```cpp
 * // JSON response
 * return Response::json({{"message", "Hello, World!"}});
 *
 * // Custom status
 * return Response::json({{"error", "Not found"}}).status(404);
 *
 * // Plain text
 * return Response::text("Hello, World!");
 *
 * // With headers
 * return Response::json({{"ok", true}})
 *     .header("X-Custom", "value");
 * ```
 */
class Response {
public:
    /// HTTP status codes
    enum class Status : uint16_t {
        Ok = 200,
        Created = 201,
        Accepted = 202,
        NoContent = 204,
        MovedPermanently = 301,
        Found = 302,
        NotModified = 304,
        BadRequest = 400,
        Unauthorized = 401,
        Forbidden = 403,
        NotFound = 404,
        MethodNotAllowed = 405,
        Conflict = 409,
        UnprocessableEntity = 422,
        TooManyRequests = 429,
        InternalServerError = 500,
        BadGateway = 502,
        ServiceUnavailable = 503,
        GatewayTimeout = 504,
    };

    /// Default constructor - 200 OK with empty body
    Response()
        : status_code_(200)
        , content_type_("application/json") {}

    // ========================================================================
    // Static constructors
    // ========================================================================

    /**
     * Create a JSON response.
     *
     * Uses a simple JSON serialization for basic types.
     *
     * @param data Key-value pairs to serialize as JSON object
     * @return Response with JSON body
     *
     * Example:
     * ```cpp
     * return Response::json({
     *     {"id", "123"},
     *     {"name", "Alice"},
     *     {"active", "true"}
     * });
     * ```
     */
    static Response json(std::initializer_list<std::pair<std::string, std::string>> data);

    /**
     * Create a JSON response from a pre-serialized string.
     *
     * @param json_str Pre-serialized JSON string
     * @return Response with JSON body
     */
    static Response json_raw(std::string json_str);

    /**
     * Create a plain text response.
     *
     * @param text Response text
     * @return Response with text body
     */
    static Response text(std::string text);

    /**
     * Create an HTML response.
     *
     * @param html HTML content
     * @return Response with HTML body
     */
    static Response html(std::string html);

    /**
     * Create a binary response.
     *
     * @param data Binary data
     * @param content_type MIME type
     * @return Response with binary body
     */
    static Response binary(std::vector<uint8_t> data, std::string content_type);

    /**
     * Create an empty response with status code.
     *
     * @param status HTTP status code
     * @return Empty response with status
     */
    static Response empty(Status status);

    /**
     * Create an empty response with numeric status code.
     *
     * @param status HTTP status code
     * @return Empty response with status
     */
    static Response empty(uint16_t status);

    // ========================================================================
    // Builder methods
    // ========================================================================

    /**
     * Set the status code.
     *
     * @param code HTTP status code
     * @return Reference to this Response for chaining
     */
    Response& status(uint16_t code) {
        status_code_ = code;
        return *this;
    }

    /**
     * Set the status code using enum.
     *
     * @param code HTTP status code
     * @return Reference to this Response for chaining
     */
    Response& status(Status code) {
        status_code_ = static_cast<uint16_t>(code);
        return *this;
    }

    /**
     * Add a header.
     *
     * @param name Header name
     * @param value Header value
     * @return Reference to this Response for chaining
     */
    Response& header(std::string name, std::string value) {
        headers_[std::move(name)] = std::move(value);
        return *this;
    }

    // ========================================================================
    // Accessors
    // ========================================================================

    /// Get the status code
    [[nodiscard]] uint16_t status_code() const { return status_code_; }

    /// Get the response body
    [[nodiscard]] const std::vector<uint8_t>& body() const { return body_; }

    /// Get the response body as string
    [[nodiscard]] std::string_view body_str() const {
        return {reinterpret_cast<const char*>(body_.data()), body_.size()};
    }

    /// Get the content type
    [[nodiscard]] std::string_view content_type() const { return content_type_; }

    /// Get headers
    [[nodiscard]] const std::unordered_map<std::string, std::string>& headers() const {
        return headers_;
    }

private:
    uint16_t status_code_;
    std::vector<uint8_t> body_;
    std::string content_type_;
    std::unordered_map<std::string, std::string> headers_;
};

// ============================================================================
// Inline implementations
// ============================================================================

inline Response Response::json(std::initializer_list<std::pair<std::string, std::string>> data) {
    Response resp;
    resp.content_type_ = "application/json";

    // Simple JSON object serialization
    std::ostringstream ss;
    ss << "{";
    bool first = true;
    for (const auto& [key, value] : data) {
        if (!first) ss << ",";
        first = false;
        ss << "\"" << key << "\":";
        // Check if value looks like a JSON value (array, object, number, bool, null)
        if (value.empty() ||
            value == "true" || value == "false" || value == "null" ||
            value[0] == '[' || value[0] == '{' ||
            (value[0] >= '0' && value[0] <= '9') ||
            value[0] == '-') {
            ss << value;
        } else {
            // Escape string
            ss << "\"";
            for (char c : value) {
                switch (c) {
                    case '"': ss << "\\\""; break;
                    case '\\': ss << "\\\\"; break;
                    case '\n': ss << "\\n"; break;
                    case '\r': ss << "\\r"; break;
                    case '\t': ss << "\\t"; break;
                    default: ss << c;
                }
            }
            ss << "\"";
        }
    }
    ss << "}";

    std::string json_str = ss.str();
    resp.body_.assign(json_str.begin(), json_str.end());

    return resp;
}

inline Response Response::json_raw(std::string json_str) {
    Response resp;
    resp.content_type_ = "application/json";
    resp.body_.assign(json_str.begin(), json_str.end());
    return resp;
}

inline Response Response::text(std::string text) {
    Response resp;
    resp.content_type_ = "text/plain; charset=utf-8";
    resp.body_.assign(text.begin(), text.end());
    return resp;
}

inline Response Response::html(std::string html) {
    Response resp;
    resp.content_type_ = "text/html; charset=utf-8";
    resp.body_.assign(html.begin(), html.end());
    return resp;
}

inline Response Response::binary(std::vector<uint8_t> data, std::string content_type) {
    Response resp;
    resp.content_type_ = std::move(content_type);
    resp.body_ = std::move(data);
    return resp;
}

inline Response Response::empty(Status status) {
    Response resp;
    resp.status_code_ = static_cast<uint16_t>(status);
    resp.body_.clear();
    return resp;
}

inline Response Response::empty(uint16_t status) {
    Response resp;
    resp.status_code_ = status;
    resp.body_.clear();
    return resp;
}

// ============================================================================
// Common response helpers
// ============================================================================

namespace response {

/// Create a 200 OK response with JSON body
inline Response ok(std::initializer_list<std::pair<std::string, std::string>> data) {
    return Response::json(data);
}

/// Create a 201 Created response with JSON body
inline Response created(std::initializer_list<std::pair<std::string, std::string>> data) {
    return Response::json(data).status(Response::Status::Created);
}

/// Create a 204 No Content response
inline Response no_content() {
    return Response::empty(Response::Status::NoContent);
}

/// Create a 400 Bad Request response
inline Response bad_request(const std::string& message) {
    return Response::json({{"error", message}}).status(Response::Status::BadRequest);
}

/// Create a 401 Unauthorized response
inline Response unauthorized(const std::string& message = "Unauthorized") {
    return Response::json({{"error", message}}).status(Response::Status::Unauthorized);
}

/// Create a 403 Forbidden response
inline Response forbidden(const std::string& message = "Forbidden") {
    return Response::json({{"error", message}}).status(Response::Status::Forbidden);
}

/// Create a 404 Not Found response
inline Response not_found(const std::string& message = "Not found") {
    return Response::json({{"error", message}}).status(Response::Status::NotFound);
}

/// Create a 500 Internal Server Error response
inline Response internal_error(const std::string& message = "Internal server error") {
    return Response::json({{"error", message}}).status(Response::Status::InternalServerError);
}

} // namespace response

} // namespace archimedes

#endif // ARCHIMEDES_RESPONSE_HPP
