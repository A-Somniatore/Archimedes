from conan import ConanFile
from conan.tools.files import copy
from conan.tools.cmake import CMake, cmake_layout
import os


class ArchimedesConan(ConanFile):
    name = "archimedes"
    version = "0.1.0"
    license = "Apache-2.0"
    author = "Themis Platform Team"
    url = "https://github.com/themis-platform/archimedes"
    description = "Contract-first HTTP server framework with built-in authorization and observability"
    topics = ("http", "server", "framework", "contract-first", "rust")
    settings = "os", "compiler", "build_type", "arch"
    options = {"shared": [True, False], "fPIC": [True, False]}
    default_options = {"shared": True, "fPIC": True}
    exports_sources = "include/*", "target/include/*", "target/release/*"

    def config_options(self):
        if self.settings.os == "Windows":
            del self.options.fPIC

    def layout(self):
        cmake_layout(self)

    def build(self):
        # The library is pre-built with Cargo
        # Just need to package it
        pass

    def package(self):
        # C++ headers
        copy(self, "*.hpp", src=os.path.join(self.source_folder, "include"),
             dst=os.path.join(self.package_folder, "include"))
        
        # C header
        copy(self, "archimedes.h", src=os.path.join(self.source_folder, "target", "include"),
             dst=os.path.join(self.package_folder, "include"))
        
        # Library
        if self.settings.os == "Windows":
            copy(self, "archimedes_ffi.dll", 
                 src=os.path.join(self.source_folder, "target", "release"),
                 dst=os.path.join(self.package_folder, "bin"))
            copy(self, "archimedes_ffi.dll.lib",
                 src=os.path.join(self.source_folder, "target", "release"),
                 dst=os.path.join(self.package_folder, "lib"))
        elif self.settings.os == "Macos":
            copy(self, "libarchimedes_ffi.dylib",
                 src=os.path.join(self.source_folder, "target", "release"),
                 dst=os.path.join(self.package_folder, "lib"))
        else:
            copy(self, "libarchimedes_ffi.so",
                 src=os.path.join(self.source_folder, "target", "release"),
                 dst=os.path.join(self.package_folder, "lib"))

    def package_info(self):
        self.cpp_info.libs = ["archimedes_ffi"]
        self.cpp_info.includedirs = ["include"]
        
        if self.settings.os == "Linux":
            self.cpp_info.system_libs = ["pthread", "dl", "m"]
