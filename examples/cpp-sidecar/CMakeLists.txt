cmake_minimum_required(VERSION 3.16)
project(example-cpp-sidecar VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Fetch dependencies
include(FetchContent)

# cpp-httplib (header-only HTTP library)
FetchContent_Declare(
    httplib
    GIT_REPOSITORY https://github.com/yhirose/cpp-httplib.git
    GIT_TAG v0.15.3
)
FetchContent_MakeAvailable(httplib)

# nlohmann/json (header-only JSON library)
FetchContent_Declare(
    json
    GIT_REPOSITORY https://github.com/nlohmann/json.git
    GIT_TAG v3.11.3
)
FetchContent_MakeAvailable(json)

# Main executable
add_executable(example-cpp src/main.cpp)

target_link_libraries(example-cpp PRIVATE httplib::httplib nlohmann_json::nlohmann_json)

# Install
install(TARGETS example-cpp DESTINATION bin)
