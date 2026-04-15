use crate::core::filesystem::FileSystem;
use crate::core::filesystem::NativeFileSystem;

#[cfg(target_os = "windows")]
use crate::core::filesystem::get_executable_extensions;
use anyhow::{Error, anyhow};
use rayon::prelude::*;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, atomic::{AtomicBool, Ordering}};

/// 检查选项是否存在于 options 中且值为期望值
fn check_option(options: &HashMap<String, bool>, keys: Vec<&str>, expected: bool) -> bool {
    for key in keys {
        if let Some(&value) = options.get(key) {
            return value == expected;
        }
    }
    false
}

/// 检查目录下的命令（带 level 信息，用于并发优化）
fn check_dir_with_level<F: FileSystem>(
    result: &Mutex<HashMap<String, (PathBuf, usize)>>,
    fs: &F,
    cmd: &str,
    dir_path: &Path,
    level: usize,
) {
    #[cfg(not(target_os = "windows"))]
    {
        let candidate = dir_path.join(cmd);
        try_add_path_with_level(result, fs, &candidate, level);
    }

    #[cfg(target_os = "windows")]
    {
        let extensions = get_executable_extensions();

        // 尝试所有常见的扩展名
        for ext in &extensions {
            let candidate_ext = dir_path.join(format!("{}{}", cmd, ext));
            try_add_path_with_level(result, fs, &candidate_ext, level);
        }
    }
}

/// 检查目录下的命令（带 level 信息，用于并发优化，找到后立即停止）
fn check_dir_with_level_and_stop<F: FileSystem>(
    result: &Mutex<HashMap<String, (PathBuf, usize)>>,
    fs: &F,
    cmd: &str,
    dir_path: &Path,
    level: usize,
    found: &AtomicBool,
) {
    #[cfg(not(target_os = "windows"))]
    {
        let candidate = dir_path.join(cmd);
        try_add_path_with_level_and_stop(result, fs, &candidate, level, found);
    }

    #[cfg(target_os = "windows")]
    {
        let extensions = get_executable_extensions();

        // 尝试所有常见的扩展名
        for ext in &extensions {
            let candidate_ext = dir_path.join(format!("{}{}", cmd, ext));
            try_add_path_with_level_and_stop(result, fs, &candidate_ext, level, found);
        }
    }
}

/// 尝试添加文件路径到结果列表（带 level 信息，用于并发优化）
fn try_add_path_with_level<F: FileSystem>(
    result: &Mutex<HashMap<String, (PathBuf, usize)>>,
    fs: &F,
    candidate: &Path,
    level: usize,
) {
    if fs.is_file(candidate) && fs.is_executable(candidate)
        && let Ok(canonical) = fs.canonicalize(candidate) {
        let path_str = canonical.to_string_lossy().to_string();
        let mut results = result.lock().unwrap();
        // 避免重复，只保留最小的 level
        results.entry(path_str)
            .and_modify(|existing| {
                if level < existing.1 {
                    existing.1 = level;
                }
            })
            .or_insert((canonical, level));
    }
}

/// 尝试添加文件路径到结果列表（带 level 信息，找到后立即停止）
fn try_add_path_with_level_and_stop<F: FileSystem>(
    result: &Mutex<HashMap<String, (PathBuf, usize)>>,
    fs: &F,
    candidate: &Path,
    level: usize,
    found: &AtomicBool,
) {
    if found.load(Ordering::Relaxed) {
        return;
    }

    if fs.is_file(candidate) && fs.is_executable(candidate)
        && let Ok(canonical) = fs.canonicalize(candidate) {
        let path_str = canonical.to_string_lossy().to_string();
        let mut results = result.lock().unwrap();

        // 避免重复，只保留最小的 level
        results.entry(path_str)
            .and_modify(|existing| {
                if level < existing.1 {
                    existing.1 = level;
                    found.store(true, Ordering::Relaxed);
                }
            })
            .or_insert((canonical, level));

        // 标记已找到结果
        found.store(true, Ordering::Relaxed);
    }
}

/// 查找所有匹配的命令路径（使用文件系统抽象）
///
/// # 参数
/// * `cmd` - 要查找的命令名称
/// * `options` - 选项映射表
/// * `fs` - 文件系统实现
/// * `path_var` - PATH 环境变量的值
///
/// # 选项
/// * `all` / `-a` - 显示所有匹配项
///
/// # 返回值
/// 成功时返回找到的路径列表，失败时返回错误信息
pub fn which_all_fs<F: FileSystem>(
    cmd: &str,
    options: &HashMap<String, bool>,
    fs: &F,
    path_var: &str,
) -> Result<Vec<PathBuf>, Error> {
    // 1. 如果命令本身包含路径分隔符（如 ./my_app 或 /usr/bin/git），直接检查
    if cmd.contains('/') || cmd.contains('\\') {
        let path = PathBuf::from(cmd);
        if fs.is_file(&path) {
            if fs.is_executable(&path) {
                return Ok(vec![fs.canonicalize(&path)?]);
            }
            return Err(anyhow!("{} is not executable", cmd));
        }
        return Err(anyhow!("{} not found", cmd));
    }

    // 2. 根据操作系统确定分隔符 (Windows 是 ';'，Linux/macOS 是 ':')
    let separator = if cfg!(windows) { ';' } else { ':' };
    let need_all = check_option(options, vec!["all", "-a"], true);

    // 3. 遍历 PATH 中的每个目录
    let path_dirs: Vec<&str> = path_var
        .split(separator)
        .filter(|d| !d.is_empty())
        .collect();

    // 1. 保证输出顺序与 PATH 一致 [最重要逻辑]
    // 2. need_all == true ,并发 遍历， 输出结果 也许保证 按 PATH 顺序排序
    // 3. need_all == false, 并发遍历 结果只输出 PATH 顺序找到第一个
    // 4. need_all == false 并发优化, 优先级查找 结果 Map<{Path:Level},Result>, level 安装path 排序 [越考前越值小，最小值为0]，
    // 5.  need_all == false  并优化: 已经找到 小 level  结果，停止遍历查找，输出结果

    if need_all {
        // need_all == true: 并发遍历，输出结果保证按 PATH 顺序排序
        // 使用 Map 存储结果：key 为路径字符串，value 为 (PathBuf, level)
        let results_map: Mutex<HashMap<String, (PathBuf, usize)>> = Mutex::new(HashMap::new());

        path_dirs.par_iter().enumerate().for_each(|(level, dir)| {
            if dir.is_empty() {
                return;
            }

            let dir_path = PathBuf::from(dir);
            check_dir_with_level(&results_map, fs, cmd, &dir_path, level);
        });

        // 按 PATH 顺序排序结果
        let final_result = results_map.into_inner()?;
        let mut sorted_results: Vec<(PathBuf, usize)> = final_result.into_values().collect();
        sorted_results.sort_by_key(|(_, level)| *level);

        if !sorted_results.is_empty() {
            let paths: Vec<PathBuf> = sorted_results.into_iter().map(|(path, _)| path).collect();
            return Ok(paths);
        }
    } else {
        // need_all == false: 并发遍历，只返回第一个找到的结果（按 PATH 顺序）
        // 使用 Map 存储结果：key 为路径字符串，value 为 (PathBuf, level)
        let results_map: Mutex<HashMap<String, (PathBuf, usize)>> = Mutex::new(HashMap::new());
        // 使用原子布尔值标记是否已找到结果
        let found = AtomicBool::new(false);

        path_dirs.par_iter().enumerate().for_each(|(level, dir)| {
            // 如果已经找到结果，提前退出
            if found.load(Ordering::Relaxed) {
                return;
            }

            if dir.is_empty() {
                return;
            }

            let dir_path = PathBuf::from(dir);
            check_dir_with_level_and_stop(&results_map, fs, cmd, &dir_path, level, &found);
        });

        // 按 PATH 顺序排序，返回第一个结果
        let final_result = results_map.into_inner()?;
        if !final_result.is_empty() {
            let mut sorted_results: Vec<(PathBuf, usize)> = final_result.into_values().collect();
            sorted_results.sort_by_key(|(_, level)| *level);
            return Ok(vec![sorted_results[0].0.clone()]);
        }
    }

    Err(anyhow!("{} not found", cmd))
}

/// 查找所有匹配的命令路径（使用本地文件系统）
///
/// # 参数
/// * `cmd` - 要查找的命令名称
/// * `options` - 选项映射表
///
/// # 选项
/// * `all` / `-a` - 显示所有匹配项
///
/// # 返回值
/// 成功时返回找到的路径列表，失败时返回错误信息
pub fn which_all(cmd: &str, options: &HashMap<String, bool>) -> Result<Vec<PathBuf>, Error> {
    let fs = NativeFileSystem::new();
    let path_var = env::var("PATH").unwrap_or_default();
    which_all_fs(cmd, options, &fs, &path_var)
}

/// 查找第一个匹配的命令路径（使用文件系统抽象）
///
/// # 参数
/// * `cmd` - 要查找的命令名称
/// * `options` - 选项映射表
/// * `fs` - 文件系统实现
/// * `path_var` - PATH 环境变量的值
///
/// # 返回值
/// 成功时返回找到的第一个路径，失败时返回错误信息
pub fn which_fs<F: FileSystem>(
    cmd: &str,
    options: &HashMap<String, bool>,
    fs: &F,
    path_var: &str,
) -> Result<PathBuf, Error> {
    let paths = which_all_fs(cmd, options, fs, path_var)?;
    paths
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("{} not found", cmd))
}

/// 查找第一个匹配的命令路径（默认行为，使用本地文件系统）
///
/// # 参数
/// * `cmd` - 要查找的命令名称
/// * `options` - 选项映射表（目前未使用，保留用于未来扩展）
///
/// # 返回值
/// 成功时返回找到的第一个路径，失败时返回错误信息
#[allow(dead_code)]
pub fn which(cmd: &str, options: &HashMap<String, bool>) -> Result<PathBuf, Error> {
    let fs = NativeFileSystem::new();
    let path_var = env::var("PATH").unwrap_or_default();
    which_fs(cmd, options, &fs, &path_var)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::filesystem::VirtualFileSystem;

    #[test]
    fn test_which_all_existing_command() {
        // 使用 rustc 作为测试命令，这在开发环境中应该总是存在
        let options = HashMap::new();
        let result = which_all("rustc", &options);
        if let Err(e) = &result {
            eprintln!("Error: {:?}", e);
        }
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_which_all_with_all_option() {
        let mut options = HashMap::new();
        options.insert("all".to_string(), true);
        let result = which_all("rustc", &options);
        if let Err(e) = &result {
            eprintln!("Error: {:?}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_which_nonexistent_command() {
        let options = HashMap::new();
        let result = which_all("nonexistent_command_xyz123", &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_which_with_path() {
        let test_cmd = if cfg!(windows) {
            r"C:\Windows\System32\cmd.exe"
        } else {
            "/bin/ls"
        };
        let options = HashMap::new();
        let result = which_all(test_cmd, &options);

        // 在 CI 环境中可能找不到这些路径，所以只检查错误类型
        if let Err(e) = result {
            // 路径不存在是可能的
            assert!(e.to_string().contains("not found"));
        }
    }

    #[test]
    fn test_which_single() {
        let options = HashMap::new();
        let result = which("rustc", &options);
        if let Err(e) = &result {
            eprintln!("Error: {:?}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_option() {
        let mut options = HashMap::new();
        options.insert("all".to_string(), true);

        assert!(check_option(&options, vec!["all", "-a"], true));
        assert!(!check_option(&options, vec!["all", "-a"], false));
        assert!(!check_option(&options, vec!["other"], true));
    }

    #[test]
    fn test_which_all_empty_command() {
        let options = HashMap::new();
        let result = which_all("", &options);
        assert!(result.is_err());
    }

    // 虚拟文件系统测试
    #[test]
    fn test_virtual_filesystem_which() {
        let vfs = VirtualFileSystem::new();
        // 根据平台添加不同扩展名的文件
        if cfg!(windows) {
            vfs.add_files(vec![
                ("/usr/bin/ls.EXE", true),
                ("/usr/bin/cat.EXE", true),
                ("/usr/bin/not-executable.EXE", false),
            ]);
        } else {
            vfs.add_files(vec![
                ("/usr/bin/ls", true),
                ("/usr/bin/cat", true),
                ("/usr/bin/not-executable", false),
            ]);
        }

        let options = HashMap::new();
        let path_var = "/usr/bin";

        // 测试找到可执行文件
        let result = which_fs("ls", &options, &vfs, path_var);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("ls"));
    }

    #[test]
    fn test_virtual_filesystem_which_all() {
        let vfs = VirtualFileSystem::new();
        // 根据平台添加不同扩展名的文件
        if cfg!(windows) {
            vfs.add_files(vec![("/usr/bin/test.EXE", true), ("/bin/test.EXE", true)]);
        } else {
            vfs.add_files(vec![("/usr/bin/test", true), ("/bin/test", true)]);
        }

        let mut options = HashMap::new();
        options.insert("all".to_string(), true);
        // 根据平台使用正确的分隔符
        let path_var = if cfg!(windows) {
            "/usr/bin;/bin"
        } else {
            "/usr/bin:/bin"
        };

        let result = which_all_fs("test", &options, &vfs, path_var);
        assert!(result.is_ok());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_virtual_filesystem_non_executable() {
        let vfs = VirtualFileSystem::new();
        vfs.add_file("/usr/bin/not-exec", false);

        let options = HashMap::new();
        let path_var = "/usr/bin";

        let result = which_fs("not-exec", &options, &vfs, path_var);
        assert!(result.is_err());
    }

    #[test]
    fn test_virtual_filesystem_with_path() {
        let vfs = VirtualFileSystem::new();
        vfs.add_file("/usr/bin/ls", true);

        let options = HashMap::new();
        let path_var = "/usr/bin";

        // 测试带路径的命令
        let result = which_all_fs("/usr/bin/ls", &options, &vfs, path_var);
        assert!(result.is_ok());
    }
}
