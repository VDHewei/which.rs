//! WebAssembly example for the which library
//!
//! This example demonstrates how to use the which library in a WebAssembly environment.

#[cfg(target_arch = "wasm32")]
use which::core::filesystem::VirtualFileSystem;
#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WhichResult {
    found: bool,
    paths: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WhichResult {
    #[wasm_bindgen(constructor)]
    pub fn new(found: bool, paths: Vec<String>) -> Self {
        Self { found, paths }
    }

    #[wasm_bindgen(getter)]
    pub fn found(&self) -> bool {
        self.found
    }

    #[wasm_bindgen(getter)]
    pub fn paths(&self) -> Vec<String> {
        self.paths.clone()
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn find_command(command: &str, path_var: &str, show_all: bool) -> WhichResult {
    let vfs = VirtualFileSystem::new();
    let mut options = HashMap::new();
    options.insert("all".to_string(), show_all);
    
    match which::core::core::which_fs(command, &options, &vfs, path_var) {
        Ok(path) => WhichResult::new(true, vec![path.to_string_lossy().to_string()]),
        Err(_) => WhichResult::new(false, vec![]),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn find_all_commands(command: &str, path_var: &str) -> WhichResult {
    let vfs = VirtualFileSystem::new();
    let mut options = HashMap::new();
    options.insert("all".to_string(), true);
    
    match which::core::core::which_all_fs(command, &options, &vfs, path_var) {
        Ok(paths) => {
            let path_strings: Vec<String> = paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            WhichResult::new(true, path_strings)
        }
        Err(_) => WhichResult::new(false, vec![]),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn add_virtual_file(path: &str, executable: bool) {
    let vfs = VirtualFileSystem::new();
    vfs.add_file(path, executable);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("This example is for WebAssembly compilation.");
    println!("To build for wasm, use: cargo build --target wasm32-unknown-unknown --example wasm --features wasm");
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // WASM 目标不需要 main 函数，但这是编译器要求的
    // 实际的导出函数在 #[wasm_bindgen] 属性下定义
}
