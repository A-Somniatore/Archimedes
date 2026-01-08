/**
 * Archimedes Configuration
 *
 * @copyright 2024 Themis Platform Team
 * @license Apache-2.0
 */

#ifndef ARCHIMEDES_CONFIG_HPP
#define ARCHIMEDES_CONFIG_HPP

#include <cstdint>
#include <optional>
#include <string>
#include <string_view>

// Forward declare C struct
struct archimedes_config;

namespace archimedes {

/**
 * Configuration builder for Archimedes applications.
 *
 * Uses a fluent builder pattern for easy configuration:
 *
 * ```cpp
 * auto config = archimedes::Config{}
 *     .contract_path("contract.json")
 *     .policy_bundle_path("policy.bundle")
 *     .listen_port(8080)
 *     .enable_validation(true);
 * ```
 */
class Config {
public:
    /// Default constructor with sensible defaults
    Config() = default;

    // ========================================================================
    // Required Settings
    // ========================================================================

    /**
     * Set the contract file path (required).
     *
     * @param path Path to the Themis contract JSON file
     * @return Reference to this Config for chaining
     */
    Config& contract_path(std::string path) {
        contract_path_ = std::move(path);
        return *this;
    }

    // ========================================================================
    // Optional Settings
    // ========================================================================

    /**
     * Set the policy bundle path for OPA authorization.
     *
     * @param path Path to the OPA policy bundle
     * @return Reference to this Config for chaining
     */
    Config& policy_bundle_path(std::string path) {
        policy_bundle_path_ = std::move(path);
        return *this;
    }

    /**
     * Set the listen address.
     *
     * @param addr Address to bind to (default: "0.0.0.0")
     * @return Reference to this Config for chaining
     */
    Config& listen_addr(std::string addr) {
        listen_addr_ = std::move(addr);
        return *this;
    }

    /**
     * Set the listen port.
     *
     * @param port Port number (default: 8080)
     * @return Reference to this Config for chaining
     */
    Config& listen_port(uint16_t port) {
        listen_port_ = port;
        return *this;
    }

    /**
     * Set the metrics port.
     *
     * @param port Metrics port (default: 9090, 0 to disable)
     * @return Reference to this Config for chaining
     */
    Config& metrics_port(uint16_t port) {
        metrics_port_ = port;
        return *this;
    }

    /**
     * Enable or disable request validation.
     *
     * @param enable Whether to validate requests (default: true)
     * @return Reference to this Config for chaining
     */
    Config& enable_validation(bool enable) {
        enable_validation_ = enable;
        return *this;
    }

    /**
     * Enable or disable response validation.
     *
     * @param enable Whether to validate responses (default: false)
     * @return Reference to this Config for chaining
     */
    Config& enable_response_validation(bool enable) {
        enable_response_validation_ = enable;
        return *this;
    }

    /**
     * Enable or disable authorization.
     *
     * @param enable Whether to authorize requests (default: true if policy set)
     * @return Reference to this Config for chaining
     */
    Config& enable_authorization(bool enable) {
        enable_authorization_ = enable;
        return *this;
    }

    /**
     * Enable or disable OpenTelemetry tracing.
     *
     * @param enable Whether to enable tracing (default: true)
     * @return Reference to this Config for chaining
     */
    Config& enable_tracing(bool enable) {
        enable_tracing_ = enable;
        return *this;
    }

    /**
     * Set the OTLP endpoint for traces.
     *
     * @param endpoint OTLP endpoint URL
     * @return Reference to this Config for chaining
     */
    Config& otlp_endpoint(std::string endpoint) {
        otlp_endpoint_ = std::move(endpoint);
        return *this;
    }

    /**
     * Set the service name for telemetry.
     *
     * @param name Service name (default: "archimedes-service")
     * @return Reference to this Config for chaining
     */
    Config& service_name(std::string name) {
        service_name_ = std::move(name);
        return *this;
    }

    /**
     * Set the graceful shutdown timeout.
     *
     * @param secs Timeout in seconds (default: 30)
     * @return Reference to this Config for chaining
     */
    Config& shutdown_timeout(uint32_t secs) {
        shutdown_timeout_secs_ = secs;
        return *this;
    }

    /**
     * Set the maximum request body size.
     *
     * @param bytes Maximum size in bytes (default: 1MB)
     * @return Reference to this Config for chaining
     */
    Config& max_body_size(size_t bytes) {
        max_body_size_ = bytes;
        return *this;
    }

    /**
     * Set the request timeout.
     *
     * @param secs Timeout in seconds (default: 30, 0 for no timeout)
     * @return Reference to this Config for chaining
     */
    Config& request_timeout(uint32_t secs) {
        request_timeout_secs_ = secs;
        return *this;
    }

    // ========================================================================
    // Accessors
    // ========================================================================

    [[nodiscard]] const std::string& get_contract_path() const { return contract_path_; }
    [[nodiscard]] const std::optional<std::string>& get_policy_bundle_path() const { return policy_bundle_path_; }
    [[nodiscard]] const std::optional<std::string>& get_listen_addr() const { return listen_addr_; }
    [[nodiscard]] uint16_t get_listen_port() const { return listen_port_; }
    [[nodiscard]] uint16_t get_metrics_port() const { return metrics_port_; }
    [[nodiscard]] bool get_enable_validation() const { return enable_validation_; }
    [[nodiscard]] bool get_enable_response_validation() const { return enable_response_validation_; }
    [[nodiscard]] bool get_enable_authorization() const { return enable_authorization_; }
    [[nodiscard]] bool get_enable_tracing() const { return enable_tracing_; }
    [[nodiscard]] const std::optional<std::string>& get_otlp_endpoint() const { return otlp_endpoint_; }
    [[nodiscard]] const std::optional<std::string>& get_service_name() const { return service_name_; }
    [[nodiscard]] uint32_t get_shutdown_timeout() const { return shutdown_timeout_secs_; }
    [[nodiscard]] size_t get_max_body_size() const { return max_body_size_; }
    [[nodiscard]] uint32_t get_request_timeout() const { return request_timeout_secs_; }

    // ========================================================================
    // Internal: Convert to C config
    // ========================================================================

    /**
     * Convert to C API config struct.
     * @internal
     */
    [[nodiscard]] archimedes_config to_c_config() const;

private:
    std::string contract_path_;
    std::optional<std::string> policy_bundle_path_;
    std::optional<std::string> listen_addr_;
    uint16_t listen_port_ = 8080;
    uint16_t metrics_port_ = 9090;
    bool enable_validation_ = true;
    bool enable_response_validation_ = false;
    bool enable_authorization_ = true;
    bool enable_tracing_ = true;
    std::optional<std::string> otlp_endpoint_;
    std::optional<std::string> service_name_;
    uint32_t shutdown_timeout_secs_ = 30;
    size_t max_body_size_ = 1024 * 1024;  // 1MB
    uint32_t request_timeout_secs_ = 30;
};

// ============================================================================
// Inline implementation
// ============================================================================

} // namespace archimedes

// Include C header for struct definition
extern "C" {
#include "archimedes.h"
}

namespace archimedes {

inline archimedes_config Config::to_c_config() const {
    archimedes_config c = {};

    c.contract_path = contract_path_.c_str();
    c.policy_bundle_path = policy_bundle_path_ ? policy_bundle_path_->c_str() : nullptr;
    c.listen_addr = listen_addr_ ? listen_addr_->c_str() : nullptr;
    c.listen_port = listen_port_;
    c.metrics_port = metrics_port_;
    c.enable_validation = enable_validation_;
    c.enable_response_validation = enable_response_validation_;
    c.enable_authorization = enable_authorization_;
    c.enable_tracing = enable_tracing_;
    c.otlp_endpoint = otlp_endpoint_ ? otlp_endpoint_->c_str() : nullptr;
    c.service_name = service_name_ ? service_name_->c_str() : nullptr;
    c.shutdown_timeout_secs = shutdown_timeout_secs_;
    c.max_body_size = max_body_size_;
    c.request_timeout_secs = request_timeout_secs_;

    return c;
}

} // namespace archimedes

#endif // ARCHIMEDES_CONFIG_HPP
