//! # which.rs
//!
//! A Rust implementation of the 'which' command-line utility with multi-filesystem support.
//!
//! This library provides functionality to locate executables in the system PATH or in virtual filesystems.
//!
//! ## Features
//!
//! - **Native filesystem support**: Works with the system's native filesystem
//! - **Virtual filesystem support**: Allows searching in in-memory virtual filesystems
//! - **WASM support**: Can be compiled to WebAssembly for browser environments
//! - **Concurrent search**: Uses parallel processing for faster searches
//!
//! ## Basic Usage
//!
//! ```rust
//! use which::core::core::which_all;
//! use std::collections::HashMap;
//!
//! fn main() -> anyhow::Result<()> {
//!     let options = HashMap::new();
//!     let paths = which_all("rustc", &options)?;
//!     
//!     for path in paths {
//!         println!("{}", path.display());
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Virtual Filesystem Usage
//!
//! ```rust,ignore
//! use which::core::core::which_fs;
//! use which::core::filesystem::VirtualFileSystem;
//! use std::collections::HashMap;
//!
//! fn main() -> anyhow::Result<()> {
//!     let vfs = VirtualFileSystem::new();
//!     vfs.add_file("/usr/bin/myapp", true);
//!
//!     let options = HashMap::new();
//!     let path_var = "/usr/bin";
//!     let path = which_fs("myapp", &options, &vfs, path_var)?;
//!
//!     println!("Found at: {}", path.display());
//!     Ok(())
//! }
//! ```

pub mod core;

pub use core::core::{which, which_all, which_fs, which_all_fs};
pub use core::filesystem::{FileSystem, NativeFileSystem, VirtualFileSystem};

/// Library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
