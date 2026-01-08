/**
 * Tests for archimedes::Response
 */

#include <archimedes/response.hpp>
#include <gtest/gtest.h>

using namespace archimedes;

// =============================================================================
// Default Response
// =============================================================================

TEST(ResponseTest, DefaultIsOkWithJsonContentType) {
    Response resp;
    EXPECT_EQ(resp.status_code(), 200);
    EXPECT_EQ(resp.content_type(), "application/json");
    EXPECT_TRUE(resp.body().empty());
}

// =============================================================================
// JSON Responses
// =============================================================================

TEST(ResponseTest, JsonCreatesJsonBody) {
    auto resp = Response::json({
        {"message", "hello"},
        {"count", "42"}
    });

    EXPECT_EQ(resp.status_code(), 200);
    EXPECT_EQ(resp.content_type(), "application/json");

    auto body = std::string(resp.body_str());
    EXPECT_TRUE(body.find("\"message\":\"hello\"") != std::string::npos);
    EXPECT_TRUE(body.find("\"count\":42") != std::string::npos);
}

TEST(ResponseTest, JsonHandlesNumericValues) {
    auto resp = Response::json({{"num", "123"}});
    auto body = std::string(resp.body_str());
    // Numeric values should not be quoted
    EXPECT_TRUE(body.find("\"num\":123") != std::string::npos);
}

TEST(ResponseTest, JsonHandlesBooleanValues) {
    auto resp = Response::json({
        {"active", "true"},
        {"deleted", "false"}
    });
    auto body = std::string(resp.body_str());
    EXPECT_TRUE(body.find("\"active\":true") != std::string::npos);
    EXPECT_TRUE(body.find("\"deleted\":false") != std::string::npos);
}

TEST(ResponseTest, JsonHandlesNullValue) {
    auto resp = Response::json({{"data", "null"}});
    auto body = std::string(resp.body_str());
    EXPECT_TRUE(body.find("\"data\":null") != std::string::npos);
}

TEST(ResponseTest, JsonHandlesArrayValue) {
    auto resp = Response::json({{"items", "[1,2,3]"}});
    auto body = std::string(resp.body_str());
    EXPECT_TRUE(body.find("\"items\":[1,2,3]") != std::string::npos);
}

TEST(ResponseTest, JsonHandlesObjectValue) {
    auto resp = Response::json({{"nested", R"({"a":1})"}});
    auto body = std::string(resp.body_str());
    EXPECT_TRUE(body.find("\"nested\":{\"a\":1}") != std::string::npos);
}

TEST(ResponseTest, JsonEscapesStrings) {
    auto resp = Response::json({{"text", "hello \"world\""}});
    auto body = std::string(resp.body_str());
    EXPECT_TRUE(body.find("\\\"world\\\"") != std::string::npos);
}

TEST(ResponseTest, JsonRawUsesExactString) {
    auto resp = Response::json_raw(R"({"custom":true})");
    EXPECT_EQ(resp.body_str(), R"({"custom":true})");
}

// =============================================================================
// Text Responses
// =============================================================================

TEST(ResponseTest, TextCreatesTextBody) {
    auto resp = Response::text("Hello, World!");
    EXPECT_EQ(resp.status_code(), 200);
    EXPECT_EQ(resp.content_type(), "text/plain; charset=utf-8");
    EXPECT_EQ(resp.body_str(), "Hello, World!");
}

// =============================================================================
// HTML Responses
// =============================================================================

TEST(ResponseTest, HtmlCreatesHtmlBody) {
    auto resp = Response::html("<h1>Hello</h1>");
    EXPECT_EQ(resp.status_code(), 200);
    EXPECT_EQ(resp.content_type(), "text/html; charset=utf-8");
    EXPECT_EQ(resp.body_str(), "<h1>Hello</h1>");
}

// =============================================================================
// Binary Responses
// =============================================================================

TEST(ResponseTest, BinaryCreatesBinaryBody) {
    std::vector<uint8_t> data = {0x89, 0x50, 0x4E, 0x47};  // PNG magic
    auto resp = Response::binary(data, "image/png");

    EXPECT_EQ(resp.status_code(), 200);
    EXPECT_EQ(resp.content_type(), "image/png");
    EXPECT_EQ(resp.body().size(), 4);
    EXPECT_EQ(resp.body()[0], 0x89);
}

// =============================================================================
// Empty Responses
// =============================================================================

TEST(ResponseTest, EmptyWithStatus) {
    auto resp = Response::empty(Response::Status::NoContent);
    EXPECT_EQ(resp.status_code(), 204);
    EXPECT_TRUE(resp.body().empty());
}

TEST(ResponseTest, EmptyWithNumericStatus) {
    auto resp = Response::empty(204);
    EXPECT_EQ(resp.status_code(), 204);
}

// =============================================================================
// Status Chaining
// =============================================================================

TEST(ResponseTest, StatusSetsStatusCode) {
    auto resp = Response::json({{"created", "true"}}).status(201);
    EXPECT_EQ(resp.status_code(), 201);
}

TEST(ResponseTest, StatusEnumWorks) {
    auto resp = Response::json({}).status(Response::Status::BadRequest);
    EXPECT_EQ(resp.status_code(), 400);
}

// =============================================================================
// Headers
// =============================================================================

TEST(ResponseTest, HeaderAddsHeader) {
    auto resp = Response::json({})
        .header("X-Custom", "value")
        .header("X-Another", "test");

    const auto& headers = resp.headers();
    EXPECT_EQ(headers.at("X-Custom"), "value");
    EXPECT_EQ(headers.at("X-Another"), "test");
}

// =============================================================================
// Response Helpers
// =============================================================================

TEST(ResponseHelpersTest, OkCreates200) {
    auto resp = response::ok({{"status", "ok"}});
    EXPECT_EQ(resp.status_code(), 200);
}

TEST(ResponseHelpersTest, CreatedCreates201) {
    auto resp = response::created({{"id", "123"}});
    EXPECT_EQ(resp.status_code(), 201);
}

TEST(ResponseHelpersTest, NoContentCreates204) {
    auto resp = response::no_content();
    EXPECT_EQ(resp.status_code(), 204);
    EXPECT_TRUE(resp.body().empty());
}

TEST(ResponseHelpersTest, BadRequestCreates400) {
    auto resp = response::bad_request("Invalid input");
    EXPECT_EQ(resp.status_code(), 400);
    EXPECT_TRUE(std::string(resp.body_str()).find("Invalid input") != std::string::npos);
}

TEST(ResponseHelpersTest, UnauthorizedCreates401) {
    auto resp = response::unauthorized();
    EXPECT_EQ(resp.status_code(), 401);
}

TEST(ResponseHelpersTest, ForbiddenCreates403) {
    auto resp = response::forbidden();
    EXPECT_EQ(resp.status_code(), 403);
}

TEST(ResponseHelpersTest, NotFoundCreates404) {
    auto resp = response::not_found("Resource not found");
    EXPECT_EQ(resp.status_code(), 404);
}

TEST(ResponseHelpersTest, InternalErrorCreates500) {
    auto resp = response::internal_error("Something went wrong");
    EXPECT_EQ(resp.status_code(), 500);
}
