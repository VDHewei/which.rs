use anyhow::{Error, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// 文件系统抽象 trait
pub trait FileSystem: Send + Sync {
    /// 检查路径是否存在
    #[allow(dead_code)]
    fn exists(&self, path: &Path) -> bool;
    
    /// 检查路径是否为文件
    fn is_file(&self, path: &Path) -> bool;
    
    /// 检查文件是否可执行
    fn is_executable(&self, path: &Path) -> bool;
    
    /// 规范化路径
    fn canonicalize(&self, path: &Path) -> Result<PathBuf>;
    
    /// 读取文件元数据
    #[allow(dead_code)]
    fn metadata(&self, path: &Path) -> Result<Metadata>;
}

/// 文件元数据
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Metadata {
    pub is_file: bool,
    pub is_executable: bool,
}

/// 本地文件系统实现
#[derive(Debug, Clone)]
pub struct NativeFileSystem;

impl NativeFileSystem {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NativeFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for NativeFileSystem {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
    
    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }
    
    fn is_executable(&self, path: &Path) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(path) {
                let mode = metadata.permissions().mode();
                return mode & 0o111 != 0;
            }
            return false;
        }

        #[cfg(windows)]
        {
            // Windows 不需要特别的可执行检查，文件扩展名决定了
            path.exists()
        }

        #[cfg(not(any(unix, windows)))]
        {
            // 其他平台（如 WASM）默认返回 false
            let _path = path; // 避免未使用变量警告
            false
        }
    }
    
    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        path.canonicalize().map_err(|e| Error::msg(e.to_string()))
    }
    
    fn metadata(&self, path: &Path) -> Result<Metadata> {
        Ok(Metadata {
            is_file: self.is_file(path),
            is_executable: self.is_executable(path),
        })
    }
}

/// 虚拟文件系统实现（内存中）
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct VirtualFileSystem {
    files: Arc<Mutex<HashMap<String, VirtualFile>>>,
    current_dir: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct VirtualFile {
    executable: bool,
}

#[allow(dead_code)]
impl VirtualFileSystem {
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            current_dir: "/".to_string(),
        }
    }
    
    /// 添加一个虚拟文件
    pub fn add_file(&self, path: &str, executable: bool) {
        let mut files = self.files.lock().unwrap();
        // 统一使用正斜杠
        let normalized_path = path.replace('\\', "/");
        files.insert(normalized_path, VirtualFile {
            executable,
        });
    }
    
    /// 添加多个虚拟文件
    pub fn add_files(&self, files: Vec<(&str, bool)>) {
        for (path, executable) in files {
            self.add_file(path, executable);
        }
    }
    
    /// 设置当前工作目录
    pub fn set_current_dir(&mut self, dir: &str) {
        self.current_dir = dir.replace('\\', "/");
    }
    
    /// 规范化路径（统一使用正斜杠）
    fn normalize_path(&self, path: &Path) -> String {
        let path_str = path.to_string_lossy().replace('\\', "/");
        
        if path.is_absolute() {
            path_str
        } else {
            // 相对路径，添加当前目录
            format!("{}/{}", self.current_dir.trim_end_matches('/'), path_str.trim_start_matches('/'))
        }
    }
}

impl Default for VirtualFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for VirtualFileSystem {
    fn exists(&self, path: &Path) -> bool {
        let files = self.files.lock().unwrap();
        let normalized = self.normalize_path(path);
        files.contains_key(&normalized)
    }
    
    fn is_file(&self, path: &Path) -> bool {
        self.exists(path)
    }
    
    fn is_executable(&self, path: &Path) -> bool {
        let files = self.files.lock().unwrap();
        let normalized = self.normalize_path(path);
        if let Some(file) = files.get(&normalized) {
            return file.executable;
        }
        false
    }
    
    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        let normalized = self.normalize_path(path);
        Ok(PathBuf::from(normalized))
    }
    
    fn metadata(&self, path: &Path) -> Result<Metadata> {
        let files = self.files.lock().map_err(|e| Error::msg(format!("Mutex poisoned: {}", e)))?;
        let normalized = self.normalize_path(path);
        
        if let Some(file) = files.get(&normalized) {
            Ok(Metadata {
                is_file: true,
                is_executable: file.executable,
            })
        } else {
            Err(Error::msg("File not found"))
        }
    }
}

/// WASM 文件系统实现
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WasmFileSystem;

#[cfg(target_arch = "wasm32")]
impl WasmFileSystem {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for WasmFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
impl FileSystem for WasmFileSystem {
    fn exists(&self, _path: &Path) -> bool {
        // WASM 环境下的实现，这里简化处理
        // 实际使用时需要集成 wasm-bindgen 的 fs API
        false
    }
    
    fn is_file(&self, path: &Path) -> bool {
        self.exists(path)
    }
    
    fn is_executable(&self, _path: &Path) -> bool {
        // WASM 环境下可执行性的概念不太适用
        false
    }
    
    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        Ok(path.to_path_buf())
    }
    
    fn metadata(&self, path: &Path) -> Result<Metadata> {
        Ok(Metadata {
            is_file: self.is_file(path),
            is_executable: false,
        })
    }
}

/// 获取平台的可执行文件扩展名列表
#[cfg(target_os = "windows")]
pub fn get_executable_extensions() -> Vec<String> {
    std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD;.VBS;.VBE;.JS;.JSE;.WSF;.WSH;.MSC".to_string())
        .split(';')
        .map(|s| s.to_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_virtual_filesystem_add_file() {
        let vfs = VirtualFileSystem::new();
        vfs.add_file("/usr/bin/test", true);
        
        assert!(vfs.exists(Path::new("/usr/bin/test")));
        assert!(vfs.is_file(Path::new("/usr/bin/test")));
        assert!(vfs.is_executable(Path::new("/usr/bin/test")));
    }
    
    #[test]
    fn test_virtual_filesystem_add_files() {
        let vfs = VirtualFileSystem::new();
        vfs.add_files(vec![
            ("/bin/ls", true),
            ("/bin/cat", true),
            ("/bin/sh", true),
        ]);
        
        assert!(vfs.exists(Path::new("/bin/ls")));
        assert!(vfs.exists(Path::new("/bin/cat")));
        assert!(vfs.exists(Path::new("/bin/sh")));
    }
    
    #[test]
    fn test_virtual_filesystem_nonexistent() {
        let vfs = VirtualFileSystem::new();
        assert!(!vfs.exists(Path::new("/nonexistent")));
    }
    
    #[test]
    fn test_virtual_filesystem_canonicalize() {
        let vfs = VirtualFileSystem::new();
        vfs.add_file("/usr/bin/test", true);
        
        let result = vfs.canonicalize(Path::new("/usr/bin/test"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/usr/bin/test"));
    }
    
    #[test]
    fn test_virtual_filesystem_relative_path() {
        let mut vfs = VirtualFileSystem::new();
        vfs.set_current_dir("/home/user");
        vfs.add_file("/home/user/script.sh", true);
        
        assert!(vfs.exists(Path::new("script.sh")));
        assert!(vfs.is_executable(Path::new("script.sh")));
    }
    
    #[test]
    fn test_native_filesystem() {
        let _fs = NativeFileSystem::new();
        
        // 测试一个存在的文件（如果存在）
        #[cfg(unix)]
        if Path::new("/bin").exists() {
            assert!(_fs.exists(Path::new("/bin")));
        }
    }
}
