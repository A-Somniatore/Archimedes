cmake_minimum_required(VERSION 3.16)
project(cpp-native-example VERSION 0.1.0 LANGUAGES CXX)

# Require C++17
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Compiler warnings
if(MSVC)
    add_compile_options(/W4 /WX)
else()
    add_compile_options(-Wall -Wextra -Wpedantic -Werror)
endif()

# =============================================================================
# Find Archimedes
# =============================================================================

# Look for libarchimedes in various locations
find_library(ARCHIMEDES_LIBRARY
    NAMES archimedes_ffi archimedes
    HINTS
        ${CMAKE_SOURCE_DIR}/../../target/release
        ${CMAKE_SOURCE_DIR}/../../target/debug
        /usr/local/lib
        /usr/lib
)

# Include paths
set(ARCHIMEDES_INCLUDE_DIR ${CMAKE_SOURCE_DIR}/../../include)
set(ARCHIMEDES_C_HEADER_DIR ${CMAKE_SOURCE_DIR}/../../target/include)

# Check if found
if(NOT ARCHIMEDES_LIBRARY)
    message(FATAL_ERROR "libarchimedes not found. Please build archimedes-ffi first:\n"
        "  cargo build --release -p archimedes-ffi\n")
endif()

message(STATUS "Found archimedes: ${ARCHIMEDES_LIBRARY}")
message(STATUS "Include dirs: ${ARCHIMEDES_INCLUDE_DIR}, ${ARCHIMEDES_C_HEADER_DIR}")

# Create imported target
add_library(archimedes::archimedes SHARED IMPORTED)
set_target_properties(archimedes::archimedes PROPERTIES
    IMPORTED_LOCATION ${ARCHIMEDES_LIBRARY}
    INTERFACE_INCLUDE_DIRECTORIES "${ARCHIMEDES_INCLUDE_DIR};${ARCHIMEDES_C_HEADER_DIR}"
)

# =============================================================================
# Main Application
# =============================================================================

add_executable(cpp-native-example
    src/main.cpp
)

target_link_libraries(cpp-native-example PRIVATE
    archimedes::archimedes
)

target_include_directories(cpp-native-example PRIVATE
    ${ARCHIMEDES_INCLUDE_DIR}
    ${ARCHIMEDES_C_HEADER_DIR}
)

# =============================================================================
# Tests (optional, requires Google Test)
# =============================================================================

option(BUILD_TESTS "Build tests" OFF)

if(BUILD_TESTS)
    find_package(GTest REQUIRED)
    enable_testing()

    add_executable(cpp-native-tests
        tests/test_request.cpp
        tests/test_response.cpp
        tests/test_config.cpp
    )

    target_link_libraries(cpp-native-tests PRIVATE
        archimedes::archimedes
        GTest::gtest_main
    )

    target_include_directories(cpp-native-tests PRIVATE
        ${ARCHIMEDES_INCLUDE_DIR}
        ${ARCHIMEDES_C_HEADER_DIR}
    )

    include(GoogleTest)
    gtest_discover_tests(cpp-native-tests)
endif()

# =============================================================================
# Installation
# =============================================================================

install(TARGETS cpp-native-example
    RUNTIME DESTINATION bin
)

# Copy contract file
install(FILES ${CMAKE_SOURCE_DIR}/../contract.json
    DESTINATION share/cpp-native-example
)
