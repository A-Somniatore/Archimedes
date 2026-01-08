/**
 * Tests for archimedes::Request
 */

#include <archimedes/request.hpp>
#include <gtest/gtest.h>

using namespace archimedes;

class RequestTest : public ::testing::Test {
protected:
    Request req;
};

// =============================================================================
// Basic Accessors
// =============================================================================

TEST_F(RequestTest, DefaultValuesAreEmpty) {
    EXPECT_TRUE(req.request_id().empty());
    EXPECT_TRUE(req.trace_id().empty());
    EXPECT_TRUE(req.span_id().empty());
    EXPECT_TRUE(req.operation_id().empty());
    EXPECT_TRUE(req.method().empty());
    EXPECT_TRUE(req.path().empty());
    EXPECT_TRUE(req.query().empty());
    EXPECT_FALSE(req.has_body());
    EXPECT_FALSE(req.has_caller());
}

TEST_F(RequestTest, SetAndGetBasicFields) {
    req.set_request_id("req-123");
    req.set_trace_id("trace-456");
    req.set_span_id("span-789");
    req.set_operation_id("listUsers");
    req.set_method("GET");
    req.set_path("/users");
    req.set_query("limit=10");

    EXPECT_EQ(req.request_id(), "req-123");
    EXPECT_EQ(req.trace_id(), "trace-456");
    EXPECT_EQ(req.span_id(), "span-789");
    EXPECT_EQ(req.operation_id(), "listUsers");
    EXPECT_EQ(req.method(), "GET");
    EXPECT_EQ(req.path(), "/users");
    EXPECT_EQ(req.query(), "limit=10");
}

// =============================================================================
// Body
// =============================================================================

TEST_F(RequestTest, SetAndGetBody) {
    std::vector<uint8_t> body = {'h', 'e', 'l', 'l', 'o'};
    req.set_body(body);

    EXPECT_TRUE(req.has_body());
    EXPECT_EQ(req.body().size(), 5);
    EXPECT_EQ(req.body_str(), "hello");
}

TEST_F(RequestTest, EmptyBodyHasNoContent) {
    req.set_body({});
    EXPECT_FALSE(req.has_body());
    EXPECT_TRUE(req.body_str().empty());
}

// =============================================================================
// Path Parameters
// =============================================================================

TEST_F(RequestTest, AddAndGetPathParams) {
    req.add_path_param("userId", "123");
    req.add_path_param("postId", "456");

    auto user_id = req.path_param("userId");
    ASSERT_TRUE(user_id.has_value());
    EXPECT_EQ(*user_id, "123");

    auto post_id = req.path_param("postId");
    ASSERT_TRUE(post_id.has_value());
    EXPECT_EQ(*post_id, "456");
}

TEST_F(RequestTest, MissingPathParamReturnsNullopt) {
    EXPECT_FALSE(req.path_param("missing").has_value());
}

TEST_F(RequestTest, PathParamOrThrowThrowsOnMissing) {
    EXPECT_THROW(req.path_param_or_throw("missing"), std::runtime_error);
}

TEST_F(RequestTest, PathParamOrThrowReturnsValue) {
    req.add_path_param("id", "42");
    EXPECT_EQ(req.path_param_or_throw("id"), "42");
}

// =============================================================================
// Headers
// =============================================================================

TEST_F(RequestTest, AddAndGetHeaders) {
    req.add_header("Content-Type", "application/json");
    req.add_header("X-Request-Id", "abc123");

    // Headers are case-insensitive
    auto content_type = req.header("content-type");
    ASSERT_TRUE(content_type.has_value());
    EXPECT_EQ(*content_type, "application/json");

    auto request_id = req.header("X-REQUEST-ID");
    ASSERT_TRUE(request_id.has_value());
    EXPECT_EQ(*request_id, "abc123");
}

TEST_F(RequestTest, MissingHeaderReturnsNullopt) {
    EXPECT_FALSE(req.header("missing").has_value());
}

// =============================================================================
// Caller Identity
// =============================================================================

TEST_F(RequestTest, ParseCallerIdentityFromJson) {
    req.set_caller_identity_json(R"({"type":"service","id":"user-service"})");

    EXPECT_TRUE(req.has_caller());
    EXPECT_EQ(req.caller().type(), "service");

    auto id = req.caller().id();
    ASSERT_TRUE(id.has_value());
    EXPECT_EQ(*id, "user-service");
}

TEST_F(RequestTest, ParseUserIdentityFromJson) {
    req.set_caller_identity_json(R"({"type":"user","user_id":"user-123"})");

    EXPECT_TRUE(req.has_caller());
    EXPECT_EQ(req.caller().type(), "user");

    auto user_id = req.caller().user_id();
    ASSERT_TRUE(user_id.has_value());
    EXPECT_EQ(*user_id, "user-123");
}

TEST_F(RequestTest, EmptyJsonDoesNotSetCaller) {
    req.set_caller_identity_json("");
    EXPECT_FALSE(req.has_caller());
}
