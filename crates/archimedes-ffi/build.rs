//! Build script for archimedes-ffi
//!
//! Generates the archimedes.h C header file using cbindgen.

use std::env;
use std::path::PathBuf;

fn main() {
    // Only generate header if we're building the crate (not running tests)
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let target_dir = env::var("CARGO_TARGET_DIR")
        .unwrap_or_else(|_| {
            let manifest_dir = PathBuf::from(&crate_dir);
            manifest_dir.parent().unwrap().parent().unwrap().join("target").to_string_lossy().to_string()
        });

    // Output header path
    let header_path = PathBuf::from(&target_dir).join("include").join("archimedes.h");

    // Create include directory if it doesn't exist
    if let Some(parent) = header_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // Generate header using cbindgen
    let config = cbindgen::Config::from_file(PathBuf::from(&crate_dir).join("cbindgen.toml"))
        .expect("Failed to load cbindgen.toml");

    match cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
    {
        Ok(bindings) => {
            bindings.write_to_file(&header_path);
            println!("cargo:warning=Generated header: {}", header_path.display());
        }
        Err(e) => {
            // Don't fail the build, just warn
            // This allows the crate to build even if cbindgen has issues
            println!("cargo:warning=cbindgen failed (this is okay during development): {e}");
        }
    }

    // Rerun if cbindgen.toml changes
    println!("cargo:rerun-if-changed=cbindgen.toml");
}
