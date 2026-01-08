/**
 * Tests for archimedes::Error types
 */

#include <archimedes/error.hpp>
#include <gtest/gtest.h>

using namespace archimedes;

// =============================================================================
// Error Class
// =============================================================================

TEST(ErrorTest, ConstructFromCString) {
    Error err("Something went wrong");
    EXPECT_STREQ(err.what(), "Something went wrong");
    EXPECT_EQ(err.code(), ErrorCode::Internal);
}

TEST(ErrorTest, ConstructFromString) {
    std::string msg = "Custom error message";
    Error err(msg);
    EXPECT_STREQ(err.what(), "Custom error message");
}

TEST(ErrorTest, ConstructWithCode) {
    Error err(ErrorCode::ValidationError, "Invalid input");
    EXPECT_STREQ(err.what(), "Invalid input");
    EXPECT_EQ(err.code(), ErrorCode::ValidationError);
}

TEST(ErrorTest, NullptrMessageSaysUnknown) {
    Error err(nullptr);
    EXPECT_STREQ(err.what(), "Unknown error");
}

TEST(ErrorTest, CodeNameReturnsCorrectStrings) {
    EXPECT_STREQ(Error(ErrorCode::Ok, "").code_name(), "Ok");
    EXPECT_STREQ(Error(ErrorCode::InvalidConfig, "").code_name(), "InvalidConfig");
    EXPECT_STREQ(Error(ErrorCode::ContractLoadError, "").code_name(), "ContractLoadError");
    EXPECT_STREQ(Error(ErrorCode::PolicyLoadError, "").code_name(), "PolicyLoadError");
    EXPECT_STREQ(Error(ErrorCode::HandlerRegistrationError, "").code_name(), "HandlerRegistrationError");
    EXPECT_STREQ(Error(ErrorCode::ServerStartError, "").code_name(), "ServerStartError");
    EXPECT_STREQ(Error(ErrorCode::InvalidOperation, "").code_name(), "InvalidOperation");
    EXPECT_STREQ(Error(ErrorCode::HandlerError, "").code_name(), "HandlerError");
    EXPECT_STREQ(Error(ErrorCode::ValidationError, "").code_name(), "ValidationError");
    EXPECT_STREQ(Error(ErrorCode::AuthorizationError, "").code_name(), "AuthorizationError");
    EXPECT_STREQ(Error(ErrorCode::NullPointer, "").code_name(), "NullPointer");
    EXPECT_STREQ(Error(ErrorCode::InvalidUtf8, "").code_name(), "InvalidUtf8");
    EXPECT_STREQ(Error(ErrorCode::Internal, "").code_name(), "Internal");
}

TEST(ErrorTest, UnknownCodeNameReturnsUnknown) {
    Error err(static_cast<ErrorCode>(999), "");
    EXPECT_STREQ(err.code_name(), "Unknown");
}

TEST(ErrorTest, IsStdException) {
    try {
        throw Error("test error");
    } catch (const std::exception& e) {
        EXPECT_STREQ(e.what(), "test error");
    }
}

// =============================================================================
// ValidationError
// =============================================================================

TEST(ValidationErrorTest, HasCorrectCode) {
    ValidationError err("Field 'name' is required");
    EXPECT_EQ(err.code(), ErrorCode::ValidationError);
    EXPECT_STREQ(err.what(), "Field 'name' is required");
}

TEST(ValidationErrorTest, IsBaseError) {
    try {
        throw ValidationError("invalid");
    } catch (const Error& e) {
        EXPECT_EQ(e.code(), ErrorCode::ValidationError);
    }
}

// =============================================================================
// AuthorizationError
// =============================================================================

TEST(AuthorizationErrorTest, HasCorrectCode) {
    AuthorizationError err("Access denied");
    EXPECT_EQ(err.code(), ErrorCode::AuthorizationError);
    EXPECT_STREQ(err.what(), "Access denied");
}

TEST(AuthorizationErrorTest, IsBaseError) {
    try {
        throw AuthorizationError("forbidden");
    } catch (const Error& e) {
        EXPECT_EQ(e.code(), ErrorCode::AuthorizationError);
    }
}

// =============================================================================
// ConfigError
// =============================================================================

TEST(ConfigErrorTest, HasCorrectCode) {
    ConfigError err("Missing contract_path");
    EXPECT_EQ(err.code(), ErrorCode::InvalidConfig);
    EXPECT_STREQ(err.what(), "Missing contract_path");
}

TEST(ConfigErrorTest, IsBaseError) {
    try {
        throw ConfigError("bad config");
    } catch (const Error& e) {
        EXPECT_EQ(e.code(), ErrorCode::InvalidConfig);
    }
}
