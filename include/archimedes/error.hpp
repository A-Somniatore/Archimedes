/**
 * Archimedes Error Types
 *
 * @copyright 2024 Themis Platform Team
 * @license Apache-2.0
 */

#ifndef ARCHIMEDES_ERROR_HPP
#define ARCHIMEDES_ERROR_HPP

#include <stdexcept>
#include <string>
#include <string_view>

namespace archimedes {

/**
 * Error codes matching the C API.
 */
enum class ErrorCode {
    Ok = 0,
    InvalidConfig = 1,
    ContractLoadError = 2,
    PolicyLoadError = 3,
    HandlerRegistrationError = 4,
    ServerStartError = 5,
    InvalidOperation = 6,
    HandlerError = 7,
    ValidationError = 8,
    AuthorizationError = 9,
    NullPointer = 10,
    InvalidUtf8 = 11,
    Internal = 99,
};

/**
 * Exception thrown by Archimedes operations.
 *
 * Provides error code and message for diagnosing failures.
 */
class Error : public std::runtime_error {
public:
    /**
     * Construct from error message.
     *
     * @param message Error description
     */
    explicit Error(const char* message)
        : std::runtime_error(message ? message : "Unknown error")
        , code_(ErrorCode::Internal) {}

    /**
     * Construct from error message string.
     *
     * @param message Error description
     */
    explicit Error(const std::string& message)
        : std::runtime_error(message)
        , code_(ErrorCode::Internal) {}

    /**
     * Construct from error code and message.
     *
     * @param code Error code
     * @param message Error description
     */
    Error(ErrorCode code, const std::string& message)
        : std::runtime_error(message)
        , code_(code) {}

    /**
     * Get the error code.
     *
     * @return Error code
     */
    [[nodiscard]] ErrorCode code() const noexcept { return code_; }

    /**
     * Get string representation of error code.
     *
     * @return Error code name
     */
    [[nodiscard]] const char* code_name() const noexcept {
        switch (code_) {
            case ErrorCode::Ok: return "Ok";
            case ErrorCode::InvalidConfig: return "InvalidConfig";
            case ErrorCode::ContractLoadError: return "ContractLoadError";
            case ErrorCode::PolicyLoadError: return "PolicyLoadError";
            case ErrorCode::HandlerRegistrationError: return "HandlerRegistrationError";
            case ErrorCode::ServerStartError: return "ServerStartError";
            case ErrorCode::InvalidOperation: return "InvalidOperation";
            case ErrorCode::HandlerError: return "HandlerError";
            case ErrorCode::ValidationError: return "ValidationError";
            case ErrorCode::AuthorizationError: return "AuthorizationError";
            case ErrorCode::NullPointer: return "NullPointer";
            case ErrorCode::InvalidUtf8: return "InvalidUtf8";
            case ErrorCode::Internal: return "Internal";
            default: return "Unknown";
        }
    }

private:
    ErrorCode code_;
};

/**
 * Exception for validation failures.
 */
class ValidationError : public Error {
public:
    explicit ValidationError(const std::string& message)
        : Error(ErrorCode::ValidationError, message) {}
};

/**
 * Exception for authorization failures.
 */
class AuthorizationError : public Error {
public:
    explicit AuthorizationError(const std::string& message)
        : Error(ErrorCode::AuthorizationError, message) {}
};

/**
 * Exception for configuration errors.
 */
class ConfigError : public Error {
public:
    explicit ConfigError(const std::string& message)
        : Error(ErrorCode::InvalidConfig, message) {}
};

} // namespace archimedes

#endif // ARCHIMEDES_ERROR_HPP
