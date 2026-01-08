/**
 * Tests for archimedes::Config
 */

#include <archimedes/config.hpp>
#include <gtest/gtest.h>

using namespace archimedes;

// =============================================================================
// Default Values
// =============================================================================

TEST(ConfigTest, DefaultValues) {
    Config config;

    EXPECT_TRUE(config.get_contract_path().empty());
    EXPECT_FALSE(config.get_policy_bundle_path().has_value());
    EXPECT_FALSE(config.get_listen_addr().has_value());
    EXPECT_EQ(config.get_listen_port(), 8080);
    EXPECT_EQ(config.get_metrics_port(), 9090);
    EXPECT_TRUE(config.get_enable_validation());
    EXPECT_FALSE(config.get_enable_response_validation());
    EXPECT_TRUE(config.get_enable_authorization());
    EXPECT_TRUE(config.get_enable_tracing());
    EXPECT_FALSE(config.get_otlp_endpoint().has_value());
    EXPECT_FALSE(config.get_service_name().has_value());
    EXPECT_EQ(config.get_shutdown_timeout(), 30);
    EXPECT_EQ(config.get_max_body_size(), 1024 * 1024);
    EXPECT_EQ(config.get_request_timeout(), 30);
}

// =============================================================================
// Fluent Builder
// =============================================================================

TEST(ConfigTest, ContractPath) {
    auto config = Config{}.contract_path("contract.json");
    EXPECT_EQ(config.get_contract_path(), "contract.json");
}

TEST(ConfigTest, PolicyBundlePath) {
    auto config = Config{}.policy_bundle_path("policy.bundle");
    ASSERT_TRUE(config.get_policy_bundle_path().has_value());
    EXPECT_EQ(*config.get_policy_bundle_path(), "policy.bundle");
}

TEST(ConfigTest, ListenAddr) {
    auto config = Config{}.listen_addr("127.0.0.1");
    ASSERT_TRUE(config.get_listen_addr().has_value());
    EXPECT_EQ(*config.get_listen_addr(), "127.0.0.1");
}

TEST(ConfigTest, ListenPort) {
    auto config = Config{}.listen_port(3000);
    EXPECT_EQ(config.get_listen_port(), 3000);
}

TEST(ConfigTest, MetricsPort) {
    auto config = Config{}.metrics_port(9100);
    EXPECT_EQ(config.get_metrics_port(), 9100);
}

TEST(ConfigTest, DisableMetrics) {
    auto config = Config{}.metrics_port(0);
    EXPECT_EQ(config.get_metrics_port(), 0);
}

TEST(ConfigTest, EnableValidation) {
    auto config = Config{}.enable_validation(false);
    EXPECT_FALSE(config.get_enable_validation());
}

TEST(ConfigTest, EnableResponseValidation) {
    auto config = Config{}.enable_response_validation(true);
    EXPECT_TRUE(config.get_enable_response_validation());
}

TEST(ConfigTest, EnableAuthorization) {
    auto config = Config{}.enable_authorization(false);
    EXPECT_FALSE(config.get_enable_authorization());
}

TEST(ConfigTest, EnableTracing) {
    auto config = Config{}.enable_tracing(false);
    EXPECT_FALSE(config.get_enable_tracing());
}

TEST(ConfigTest, OtlpEndpoint) {
    auto config = Config{}.otlp_endpoint("http://jaeger:4317");
    ASSERT_TRUE(config.get_otlp_endpoint().has_value());
    EXPECT_EQ(*config.get_otlp_endpoint(), "http://jaeger:4317");
}

TEST(ConfigTest, ServiceName) {
    auto config = Config{}.service_name("my-service");
    ASSERT_TRUE(config.get_service_name().has_value());
    EXPECT_EQ(*config.get_service_name(), "my-service");
}

TEST(ConfigTest, ShutdownTimeout) {
    auto config = Config{}.shutdown_timeout(60);
    EXPECT_EQ(config.get_shutdown_timeout(), 60);
}

TEST(ConfigTest, MaxBodySize) {
    auto config = Config{}.max_body_size(10 * 1024 * 1024);
    EXPECT_EQ(config.get_max_body_size(), 10 * 1024 * 1024);
}

TEST(ConfigTest, RequestTimeout) {
    auto config = Config{}.request_timeout(120);
    EXPECT_EQ(config.get_request_timeout(), 120);
}

// =============================================================================
// Chaining
// =============================================================================

TEST(ConfigTest, FluentChaining) {
    auto config = Config{}
        .contract_path("contract.json")
        .policy_bundle_path("policy.bundle")
        .listen_addr("0.0.0.0")
        .listen_port(8080)
        .metrics_port(9090)
        .enable_validation(true)
        .enable_response_validation(false)
        .enable_authorization(true)
        .enable_tracing(true)
        .otlp_endpoint("http://jaeger:4317")
        .service_name("test-service")
        .shutdown_timeout(30)
        .max_body_size(1024 * 1024)
        .request_timeout(30);

    EXPECT_EQ(config.get_contract_path(), "contract.json");
    EXPECT_EQ(*config.get_policy_bundle_path(), "policy.bundle");
    EXPECT_EQ(*config.get_listen_addr(), "0.0.0.0");
    EXPECT_EQ(config.get_listen_port(), 8080);
    EXPECT_EQ(config.get_metrics_port(), 9090);
    EXPECT_TRUE(config.get_enable_validation());
    EXPECT_FALSE(config.get_enable_response_validation());
    EXPECT_TRUE(config.get_enable_authorization());
    EXPECT_TRUE(config.get_enable_tracing());
    EXPECT_EQ(*config.get_otlp_endpoint(), "http://jaeger:4317");
    EXPECT_EQ(*config.get_service_name(), "test-service");
    EXPECT_EQ(config.get_shutdown_timeout(), 30);
    EXPECT_EQ(config.get_max_body_size(), 1024 * 1024);
    EXPECT_EQ(config.get_request_timeout(), 30);
}

// =============================================================================
// C Config Conversion
// =============================================================================

TEST(ConfigTest, ToCConfigPopulatesFields) {
    auto config = Config{}
        .contract_path("contract.json")
        .listen_port(3000)
        .service_name("test");

    auto c_config = config.to_c_config();

    EXPECT_STREQ(c_config.contract_path, "contract.json");
    EXPECT_EQ(c_config.listen_port, 3000);
    EXPECT_STREQ(c_config.service_name, "test");
}

TEST(ConfigTest, ToCConfigNullForUnset) {
    auto config = Config{}.contract_path("contract.json");
    auto c_config = config.to_c_config();

    EXPECT_EQ(c_config.policy_bundle_path, nullptr);
    EXPECT_EQ(c_config.listen_addr, nullptr);
    EXPECT_EQ(c_config.otlp_endpoint, nullptr);
}
