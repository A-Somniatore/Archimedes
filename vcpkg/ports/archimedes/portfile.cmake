vcpkg_from_github(
    OUT_SOURCE_PATH SOURCE_PATH
    REPO themis-platform/archimedes
    REF "v${VERSION}"
    SHA512 0  # Will be updated when publishing
    HEAD_REF main
)

# Build the Rust library first
vcpkg_execute_required_process(
    COMMAND cargo build --release -p archimedes-ffi
    WORKING_DIRECTORY "${SOURCE_PATH}"
    LOGNAME build-rust-${TARGET_TRIPLET}
)

# Install headers
file(INSTALL "${SOURCE_PATH}/include/archimedes"
    DESTINATION "${CURRENT_PACKAGES_DIR}/include"
)

# Install C header from build output
file(INSTALL "${SOURCE_PATH}/target/include/archimedes.h"
    DESTINATION "${CURRENT_PACKAGES_DIR}/include"
)

# Install library
if(VCPKG_TARGET_IS_WINDOWS)
    file(INSTALL "${SOURCE_PATH}/target/release/archimedes_ffi.dll"
        DESTINATION "${CURRENT_PACKAGES_DIR}/bin"
    )
    file(INSTALL "${SOURCE_PATH}/target/release/archimedes_ffi.dll.lib"
        DESTINATION "${CURRENT_PACKAGES_DIR}/lib"
        RENAME archimedes_ffi.lib
    )
else()
    file(INSTALL "${SOURCE_PATH}/target/release/libarchimedes_ffi.so"
        DESTINATION "${CURRENT_PACKAGES_DIR}/lib"
    )
endif()

# Install license
vcpkg_install_copyright(FILE_LIST "${SOURCE_PATH}/LICENSE")

# Create CMake config
file(WRITE "${CURRENT_PACKAGES_DIR}/share/${PORT}/archimedes-config.cmake" [[
include(CMakeFindDependencyMacro)

if(NOT TARGET archimedes::archimedes)
    add_library(archimedes::archimedes SHARED IMPORTED)
    
    get_filename_component(_IMPORT_PREFIX "${CMAKE_CURRENT_LIST_FILE}" PATH)
    get_filename_component(_IMPORT_PREFIX "${_IMPORT_PREFIX}" PATH)
    get_filename_component(_IMPORT_PREFIX "${_IMPORT_PREFIX}" PATH)
    
    set_target_properties(archimedes::archimedes PROPERTIES
        INTERFACE_INCLUDE_DIRECTORIES "${_IMPORT_PREFIX}/include"
    )
    
    if(WIN32)
        set_target_properties(archimedes::archimedes PROPERTIES
            IMPORTED_LOCATION "${_IMPORT_PREFIX}/bin/archimedes_ffi.dll"
            IMPORTED_IMPLIB "${_IMPORT_PREFIX}/lib/archimedes_ffi.lib"
        )
    else()
        set_target_properties(archimedes::archimedes PROPERTIES
            IMPORTED_LOCATION "${_IMPORT_PREFIX}/lib/libarchimedes_ffi.so"
        )
    endif()
endif()
]])

file(WRITE "${CURRENT_PACKAGES_DIR}/share/${PORT}/archimedes-config-version.cmake" [[
set(PACKAGE_VERSION "@VERSION@")
if(PACKAGE_VERSION VERSION_LESS PACKAGE_FIND_VERSION)
    set(PACKAGE_VERSION_COMPATIBLE FALSE)
else()
    set(PACKAGE_VERSION_COMPATIBLE TRUE)
    if(PACKAGE_FIND_VERSION STREQUAL PACKAGE_VERSION)
        set(PACKAGE_VERSION_EXACT TRUE)
    endif()
endif()
]])
