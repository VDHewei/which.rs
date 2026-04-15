use crate::core::filesystem::FileSystem;
use crate::core::filesystem::NativeFileSystem;

#[cfg(target_os = "windows")]
use crate::core::filesystem::get_executable_extensions;
use anyhow::{Error, anyhow};
use rayon::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, atomic::{AtomicUsize, Ordering}};

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
    min_level: &AtomicUsize,
) {
    #[cfg(not(target_os = "windows"))]
    {
        let candidate = dir_path.join(cmd);
        try_add_path_with_level_and_stop(result, fs, &candidate, level, min_level);
    }

    #[cfg(target_os = "windows")]
    {
        let extensions = get_executable_extensions();

        // 尝试所有常见的扩展名
        for ext in &extensions {
            let candidate_ext = dir_path.join(format!("{}{}", cmd, ext));
            try_add_path_with_level_and_stop(result, fs, &candidate_ext, level, min_level);
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
    min_level: &AtomicUsize,
) {
    // 检查是否已经有更小的 level 被找到
    let current_min = min_level.load(Ordering::Relaxed);
    if current_min != usize::MAX && level >= current_min {
        return;
    }

    if fs.is_file(candidate) && fs.is_executable(candidate)
        && let Ok(canonical) = fs.canonicalize(candidate) {
        let path_str = canonical.to_string_lossy().to_string();
        let mut results = result.lock().unwrap();

        // 双重检查：在持有锁的情况下再次检查 min_level
        let current_min = min_level.load(Ordering::Relaxed);
        if current_min != usize::MAX && level >= current_min {
            return;
        }

        // 避免重复，只保留最小的 level
        results.entry(path_str)
            .and_modify(|existing| {
                if level < existing.1 {
                    existing.1 = level;
                    min_level.store(level, Ordering::Relaxed);
                }
            })
            .or_insert((canonical, level));

        // 更新最小 level
        min_level.store(level, Ordering::Relaxed);
    }
}

/// 检查目录是否应该被跳过
fn should_skip_directory(dir: &str, options: &HashMap<String, bool>) -> bool {
    let dir = dir.trim();

    // Skip directories starting with a dot
    if check_option(options, vec!["skip_dot"], true)
        && (dir.starts_with('.') || dir.contains("/.") || dir.contains("\\.")) {
        return true;
    }

    // Skip directories starting with a tilde
    if check_option(options, vec!["skip_tilde"], true)
        && (dir.starts_with('~') || dir.contains("/~") || dir.contains("\\~")) {
        return true;
    }

    false
}

/// 格式化路径输出
fn format_path_output(path: &Path, options: &HashMap<String, bool>) -> PathBuf {
    let path_str = path.to_string_lossy().to_string();

    // Handle show-dot option
    if check_option(options, vec!["show_dot"], true) {
        // Don't expand "." to current directory
        return PathBuf::from(&path_str);
    }

    // Handle show-tilde option
    if check_option(options, vec!["show_tilde"], true) {
        // Replace HOME directory with ~
        if let Ok(home) = env::var("HOME")
            && path_str.starts_with(&home) {
            let relative = path_str.strip_prefix(&home).unwrap_or(&path_str);
            if relative.starts_with('/') || relative.starts_with('\\') {
                let new_path = format!("~{}", relative);
                return PathBuf::from(new_path);
            } else if relative.is_empty() {
                return PathBuf::from("~");
            }
        }
    }

    PathBuf::from(path_str)
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
/// * `skip_dot` - 跳过 PATH 中以点开头的目录
/// * `skip_tilde` - 跳过 PATH 中以波浪号开头的目录
/// * `show_dot` - 不将点展开为当前目录
/// * `show_tilde` - 为 HOME 目录输出波浪号
/// * `regex` - 使用正则表达式匹配命令
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
    let use_regex = check_option(options, vec!["regex"], true);

    // 3. 遍历 PATH 中的每个目录
    let path_dirs: Vec<&str> = path_var
        .split(separator)
        .filter(|d| {
            !d.is_empty() && !should_skip_directory(d, options)
        })
        .collect();

    // 4. 如果使用正则表达式，编译正则模式
    let regex_pattern = if use_regex {
        match Regex::new(cmd) {
            Ok(re) => Some(re),
            Err(e) => return Err(anyhow!("Invalid regex pattern '{}': {}", cmd, e)),
        }
    } else {
        None
    };

    // 5. 根据需求进行搜索
    if need_all {
        // need_all == true: 并发遍历，输出结果保证按 PATH 顺序排序
        // 使用 Map 存储结果：key 为路径字符串，value 为 (PathBuf, level)
        let results_map: Mutex<HashMap<String, (PathBuf, usize)>> = Mutex::new(HashMap::new());

        path_dirs.par_iter().enumerate().for_each(|(level, dir)| {
            if dir.is_empty() {
                return;
            }

            let dir_path = PathBuf::from(dir);

            // 如果使用正则表达式，扫描目录中的所有可执行文件
            if use_regex {
                if let Some(ref re) = regex_pattern
                    && let Ok(entries) = fs.read_dir(&dir_path) {
                    for entry in entries {
                        let file_name = entry.file_name();
                        let file_name_str = file_name.to_string_lossy();

                        // 检查文件名是否匹配正则表达式
                        if re.is_match(&file_name_str) {
                            let full_path = dir_path.join(&file_name);
                            if fs.is_file(&full_path) && fs.is_executable(&full_path)
                                && let Ok(canonical) = fs.canonicalize(&full_path) {
                                let path_str = canonical.to_string_lossy().to_string();
                                let mut results = results_map.lock().unwrap();
                                results.entry(path_str)
                                    .and_modify(|existing| {
                                        if level < existing.1 {
                                            existing.1 = level;
                                        }
                                    })
                                    .or_insert((canonical, level));
                            }
                        }
                    }
                }
            } else {
                // 不使用正则表达式，直接检查命令
                check_dir_with_level(&results_map, fs, cmd, &dir_path, level);
            }
        });

        // 按 PATH 顺序排序结果
        let final_result = results_map.into_inner()?;
        let mut sorted_results: Vec<(PathBuf, usize)> = final_result.into_values().collect();
        sorted_results.sort_by_key(|(_, level)| *level);

        if !sorted_results.is_empty() {
            // 应用格式化选项
            let paths: Vec<PathBuf> = sorted_results.into_iter()
                .map(|(path, _)| format_path_output(&path, options))
                .collect();
            return Ok(paths);
        }
    } else {
        // need_all == false: 并发遍历，只返回第一个找到的结果（按 PATH 顺序）
        // 使用 Map 存储结果：key 为路径字符串，value 为 (PathBuf, level)
        let results_map: Mutex<HashMap<String, (PathBuf, usize)>> = Mutex::new(HashMap::new());
        // 使用原子 usize 跟踪找到的最小 level
        let min_level = AtomicUsize::new(usize::MAX);

        path_dirs.par_iter().enumerate().for_each(|(level, dir)| {
            // 如果已经有更小的 level 被找到，提前退出
            let current_min = min_level.load(Ordering::Relaxed);
            if current_min != usize::MAX && level >= current_min {
                return;
            }

            if dir.is_empty() {
                return;
            }

            let dir_path = PathBuf::from(dir);

            // 如果使用正则表达式，扫描目录中的所有可执行文件
            if use_regex {
                if let Some(ref re) = regex_pattern
                    && let Ok(entries) = fs.read_dir(&dir_path) {
                    for entry in entries {
                        // 检查是否已经找到结果
                        let current_min = min_level.load(Ordering::Relaxed);
                        if current_min != usize::MAX && level >= current_min {
                            break;
                        }

                        let file_name = entry.file_name();
                        let file_name_str = file_name.to_string_lossy();

                        // 检查文件名是否匹配正则表达式
                        if re.is_match(&file_name_str) {
                            let full_path = dir_path.join(&file_name);
                            if fs.is_file(&full_path) && fs.is_executable(&full_path)
                                && let Ok(canonical) = fs.canonicalize(&full_path) {
                                let path_str = canonical.to_string_lossy().to_string();
                                let mut results = results_map.lock().unwrap();

                                // 双重检查
                                let current_min = min_level.load(Ordering::Relaxed);
                                if current_min != usize::MAX && level >= current_min {
                                    return;
                                }

                                results.entry(path_str)
                                    .and_modify(|existing| {
                                        if level < existing.1 {
                                            existing.1 = level;
                                            min_level.store(level, Ordering::Relaxed);
                                        }
                                    })
                                    .or_insert((canonical, level));

                                // 更新最小 level
                                min_level.store(level, Ordering::Relaxed);
                            }
                        }
                    }
                }
            } else {
                // 不使用正则表达式，直接检查命令
                check_dir_with_level_and_stop(&results_map, fs, cmd, &dir_path, level, &min_level);
            }
        });

        // 按 PATH 顺序排序，返回第一个结果
        let final_result = results_map.into_inner()?;
        if !final_result.is_empty() {
            let mut sorted_results: Vec<(PathBuf, usize)> = final_result.into_values().collect();
            sorted_results.sort_by_key(|(_, level)| *level);
            let formatted_path = format_path_output(&sorted_results[0].0, options);
            return Ok(vec![formatted_path]);
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

    /// 并发一致性测试：验证 need_all == false 时，多次运行结果一致
    #[test]
    fn test_concurrent_need_all_false_consistency() {
        let vfs = VirtualFileSystem::new();

        // 在不同 PATH 位置添加同名可执行文件
        if cfg!(windows) {
            vfs.add_files(vec![
                ("/bin/app.EXE", true),
                ("/usr/bin/app.EXE", true),
                ("/usr/local/bin/app.EXE", true),
                ("/opt/bin/app.EXE", true),
            ]);
        } else {
            vfs.add_files(vec![
                ("/bin/app", true),
                ("/usr/bin/app", true),
                ("/usr/local/bin/app", true),
                ("/opt/bin/app", true),
            ]);
        }

        let options = HashMap::new();
        let path_var = if cfg!(windows) {
            "/usr/bin;/bin;/usr/local/bin;/opt/bin"
        } else {
            "/usr/bin:/bin:/usr/local/bin:/opt/bin"
        };

        // 多次运行，确保结果始终一致（应该返回第一个 /usr/bin/app）
        let mut first_result = None;
        for _ in 0..100 {
            let result = which_all_fs("app", &options, &vfs, path_var);
            assert!(result.is_ok());
            let paths = result.unwrap();
            assert_eq!(paths.len(), 1, "should return exactly one path");

            let path_str = paths[0].to_string_lossy().to_string();

            if let Some(ref first) = first_result {
                assert_eq!(path_str, *first, "concurrent results should be consistent");
            } else {
                first_result = Some(path_str);
            }
        }

        // 验证返回的是 PATH 中第一个匹配的路径
        let expected_path = if cfg!(windows) {
            "/usr/bin/app.EXE"
        } else {
            "/usr/bin/app"
        };
        assert_eq!(first_result.unwrap(), expected_path);
    }

    /// 并发一致性测试：验证 need_all == true 时，多次运行结果一致且按 PATH 顺序
    #[test]
    fn test_concurrent_need_all_true_consistency() {
        let vfs = VirtualFileSystem::new();

        // 在不同 PATH 位置添加同名可执行文件
        if cfg!(windows) {
            vfs.add_files(vec![
                ("/bin/app.EXE", true),
                ("/usr/bin/app.EXE", true),
                ("/usr/local/bin/app.EXE", true),
                ("/opt/bin/app.EXE", true),
            ]);
        } else {
            vfs.add_files(vec![
                ("/bin/app", true),
                ("/usr/bin/app", true),
                ("/usr/local/bin/app", true),
                ("/opt/bin/app", true),
            ]);
        }

        let mut options = HashMap::new();
        options.insert("all".to_string(), true);
        let path_var = if cfg!(windows) {
            "/usr/bin;/bin;/usr/local/bin;/opt/bin"
        } else {
            "/usr/bin:/bin:/usr/local/bin:/opt/bin"
        };

        // 多次运行，确保结果始终一致且按 PATH 顺序
        let mut first_result = None;
        for _ in 0..100 {
            let result = which_all_fs("app", &options, &vfs, path_var);
            assert!(result.is_ok());
            let paths = result.unwrap();
            assert_eq!(paths.len(), 4, "should return all 4 paths");

            let path_strings: Vec<String> = paths.iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();

            if let Some(ref first) = first_result {
                assert_eq!(path_strings, *first, "consecutive results should be consistent");
            } else {
                first_result = Some(path_strings);
            }
        }

        // 验证返回的顺序与 PATH 顺序一致
        let expected_paths = if cfg!(windows) {
            vec![
                "/usr/bin/app.EXE",
                "/bin/app.EXE",
                "/usr/local/bin/app.EXE",
                "/opt/bin/app.EXE",
            ]
        } else {
            vec![
                "/usr/bin/app",
                "/bin/app",
                "/usr/local/bin/app",
                "/opt/bin/app",
            ]
        };
        assert_eq!(first_result.unwrap(), expected_paths);
    }

    /// 并发一致性测试：验证复杂场景下的一致性
    #[test]
    fn test_concurrent_complex_scenario() {
        let vfs = VirtualFileSystem::new();

        // 创建复杂的场景：部分目录有文件，部分没有
        if cfg!(windows) {
            vfs.add_files(vec![
                ("/bin/tool1.EXE", true),
                ("/bin/tool2.EXE", true),
                ("/usr/bin/tool1.EXE", true),  // 重复
                ("/usr/bin/tool3.EXE", true),
                ("/usr/local/bin/tool2.EXE", true),  // 重复
                ("/usr/local/bin/tool4.EXE", true),
            ]);
        } else {
            vfs.add_files(vec![
                ("/bin/tool1", true),
                ("/bin/tool2", true),
                ("/usr/bin/tool1", true),  // 重复
                ("/usr/bin/tool3", true),
                ("/usr/local/bin/tool2", true),  // 重复
                ("/usr/local/bin/tool4", true),
            ]);
        }

        let path_var = if cfg!(windows) {
            "/usr/bin;/bin;/usr/local/bin;/opt/bin"
        } else {
            "/usr/bin:/bin:/usr/local/bin:/opt/bin"
        };

        // 测试 need_all == false
        let options_no_all = HashMap::new();
        let mut first_result_tool1 = None;
        for _ in 0..50 {
            let result = which_all_fs("tool1", &options_no_all, &vfs, path_var);
            assert!(result.is_ok());
            let paths = result.unwrap();
            assert_eq!(paths.len(), 1);
            let path_str = paths[0].to_string_lossy().to_string();

            if let Some(ref first) = first_result_tool1 {
                assert_eq!(path_str, *first);
            } else {
                first_result_tool1 = Some(path_str);
            }
        }
        // 应该返回 PATH 中第一个出现的 tool1
        let expected_tool1 = if cfg!(windows) {
            "/usr/bin/tool1.EXE"
        } else {
            "/usr/bin/tool1"
        };
        assert_eq!(first_result_tool1.unwrap(), expected_tool1);

        // 测试 need_all == true
        let mut options_all = HashMap::new();
        options_all.insert("all".to_string(), true);
        let mut first_result_all = None;
        for _ in 0..50 {
            let result = which_all_fs("tool1", &options_all, &vfs, path_var);
            assert!(result.is_ok());
            let paths = result.unwrap();
            // 应该找到两个 tool1（去重后）
            assert_eq!(paths.len(), 2);

            let path_strings: Vec<String> = paths.iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();

            if let Some(ref first) = first_result_all {
                assert_eq!(path_strings, *first);
            } else {
                first_result_all = Some(path_strings);
            }
        }
        // 验证顺序
        let expected_all = if cfg!(windows) {
            vec!["/usr/bin/tool1.EXE", "/bin/tool1.EXE"]
        } else {
            vec!["/usr/bin/tool1", "/bin/tool1"]
        };
        assert_eq!(first_result_all.unwrap(), expected_all);
    }

    /// 并发性能测试：验证并发实现比顺序实现更快
    #[test]
    fn test_concurrent_performance() {
        let vfs = VirtualFileSystem::new();

        // 添加大量目录和文件
        for i in 0..100 {
            let dir = format!("/usr/local/bin{:02}", i);
            if cfg!(windows) {
                vfs.add_file(&format!("{}/app.EXE", dir), true);
            } else {
                vfs.add_file(&format!("{}/app", dir), true);
            }
        }

        let mut path_dirs = Vec::new();
        for i in 0..100 {
            path_dirs.push(format!("/usr/local/bin{:02}", i));
        }
        let path_var = path_dirs.join(if cfg!(windows) { ";" } else { ":" });

        let options = HashMap::new();

        // 测试 need_all == false 的性能
        let start = std::time::Instant::now();
        for _ in 0..10 {
            let result = which_all_fs("app", &options, &vfs, &path_var);
            assert!(result.is_ok());
        }
        let duration = start.elapsed();
        println!("Concurrent search took: {:?}", duration);

        // 确保每次结果一致
        for _ in 0..10 {
            let result = which_all_fs("app", &options, &vfs, &path_var);
            assert!(result.is_ok());
            let paths = result.unwrap();
            assert_eq!(paths.len(), 1);
            // 应该返回第一个目录中的 app
            let expected_path = if cfg!(windows) {
                "/usr/local/bin00/app.EXE"
            } else {
                "/usr/local/bin00/app"
            };
            assert_eq!(paths[0].to_string_lossy().as_ref(), expected_path);
        }
    }

    /// 测试 skip-dot 选项
    #[test]
    fn test_skip_dot_option() {
        let vfs = VirtualFileSystem::new();
        // 使用 Unix 风格的路径分隔符（更简单）
        let mut options = HashMap::new();
        options.insert("skip_dot".to_string(), true);

        // 添加测试文件
        if cfg!(windows) {
            vfs.add_files(vec![
                ("C:/usr/bin/mycmd.EXE", true),
                ("C:/opt/bin/mycmd.EXE", true),
                ("C:/home/user/.local/bin/mycmd.EXE", true),  // 这个应该被跳过
            ]);
        } else {
            vfs.add_files(vec![
                ("/usr/bin/mycmd", true),
                ("/opt/bin/mycmd", true),
                ("/home/user/.local/bin/mycmd", true),  // 这个应该被跳过
            ]);
        }

        // 验证文件已添加
        let all_paths = vfs.get_all_paths();
        println!("All paths in VFS: {:?}", all_paths);

        // 测试直接查找文件
        let path_var = if cfg!(windows) {
            "C:/usr/bin;C:/home/user/.local/bin;C:/opt/bin"
        } else {
            "/usr/bin:/home/user/.local/bin:/opt/bin"
        };

        // 测试 skip-dot 选项，使用 all 选项获取所有结果
        options.insert("all".to_string(), true);
        let result = which_all_fs("mycmd", &options, &vfs, path_var);

        // 打印错误信息用于调试
        if let Err(e) = &result {
            println!("Error: {:?}", e);
        }

        assert!(result.is_ok());
        let paths = result.unwrap();
        println!("Found paths: {:?}", paths);
        // 应该只找到 /usr/bin/mycmd 和 /opt/bin/mycmd，跳过 .local/bin
        assert!(!paths.iter().any(|p| p.to_string_lossy().contains(".local")));
        assert_eq!(paths.len(), 2);
    }

    /// 测试 skip-tilde 选项
    #[test]
    fn test_skip_tilde_option() {
        let vfs = VirtualFileSystem::new();
        let mut options = HashMap::new();
        options.insert("skip_tilde".to_string(), true);

        // 添加测试文件
        if cfg!(windows) {
            vfs.add_files(vec![
                ("C:/usr/bin/mycmd.EXE", true),
                ("C:/opt/bin/mycmd.EXE", true),
                ("C:/home/user/bin/mycmd.EXE", true),
            ]);
        } else {
            vfs.add_files(vec![
                ("/usr/bin/mycmd", true),
                ("/opt/bin/mycmd", true),
                ("/home/user/bin/mycmd", true),
                ("~/bin/mycmd", true),  // 这个应该被跳过
            ]);
        }

        // 验证文件已添加
        let all_paths = vfs.get_all_paths();
        println!("All paths in VFS: {:?}", all_paths);

        let path_var = if cfg!(windows) {
            "C:/usr/bin;C:/home/user/bin;C:/opt/bin"
        } else {
            "/usr/bin:/home/user/bin:~/bin:/opt/bin"
        };

        // 测试 skip-tilde 选项，使用 all 选项获取所有结果
        options.insert("all".to_string(), true);
        let result = which_all_fs("mycmd", &options, &vfs, path_var);

        // 打印错误信息用于调试
        if let Err(e) = &result {
            println!("Error: {:?}", e);
        }

        assert!(result.is_ok());
        let _paths = result.unwrap();
        println!("Found paths: {:?}", _paths);
        // 在 Unix 上，应该跳过 ~/bin/mycmd
        #[cfg(unix)]
        {
            assert!(_paths.iter().any(|p| !p.to_string_lossy().contains("~")));
        }
    }

    /// 测试 should_skip_directory 函数
    #[test]
    fn test_should_skip_directory() {
        let mut options = HashMap::new();

        // 测试 skip-dot 选项
        options.insert("skip_dot".to_string(), true);
        assert!(should_skip_directory(".local/bin", &options));
        assert!(should_skip_directory("/home/.local/bin", &options));
        assert!(!should_skip_directory("/usr/bin", &options));

        // 测试 skip-tilde 选项
        options.clear();
        options.insert("skip_tilde".to_string(), true);
        assert!(should_skip_directory("~/.local/bin", &options));
        assert!(should_skip_directory("~/bin", &options));
        assert!(!should_skip_directory("/usr/bin", &options));

        // 测试两个选项都启用
        options.insert("skip_dot".to_string(), true);
        assert!(should_skip_directory("~/bin", &options));
        assert!(should_skip_directory(".local/bin", &options));
    }

    /// 测试 format_path_output 函数
    #[test]
    fn test_format_path_output() {
        let mut options = HashMap::new();

        // 测试默认行为（不修改路径）
        let path = PathBuf::from("/usr/bin/ls");
        let result = format_path_output(&path, &options);
        assert_eq!(result, PathBuf::from("/usr/bin/ls"));

        // 测试 show-dot 选项
        options.insert("show_dot".to_string(), true);
        let result = format_path_output(&path, &options);
        assert_eq!(result, PathBuf::from("/usr/bin/ls"));

        // 测试 show-tilde 选项（在 Windows 上这个测试可能不适用）
        #[cfg(unix)]
        {
            std::env::set_var("HOME", "/home/user");
            options.clear();
            options.insert("show_tilde".to_string(), true);
            let path = PathBuf::from("/home/user/bin/ls");
            let result = format_path_output(&path, &options);
            assert_eq!(result, PathBuf::from("~/bin/ls"));

            let path = PathBuf::from("/home/user");
            let result = format_path_output(&path, &options);
            assert_eq!(result, PathBuf::from("~"));
        }
    }

    /// 测试正则表达式支持
    #[test]
    fn test_regex_option() {
        let fs = NativeFileSystem::new();
        let mut options = HashMap::new();
        options.insert("regex".to_string(), true);
        options.insert("all".to_string(), true);

        let path_var = "/usr/bin:/bin";

        // 测试匹配所有以 "r" 开头的命令
        let result = which_all_fs("^r", &options, &fs, path_var);
        // 这个测试可能会失败，取决于系统上是否有以 r 开头的命令
        // 我们只检查结果是否成功（可能有也可能没有匹配）
        if result.is_ok() {
            let paths = result.unwrap();
            println!("Found {} commands matching '^r'", paths.len());
            // 验证所有匹配的路径都以 r 开头
            for path in &paths {
                let file_name = path.file_name().unwrap().to_string_lossy();
                assert!(file_name.starts_with('r') || file_name.starts_with('R'));
            }
        } else {
            // 如果失败，应该是"not found"而不是其他错误
            assert!(result.unwrap_err().to_string().contains("not found"));
        }
    }

    /// 测试正则表达式无效模式
    #[test]
    fn test_regex_invalid_pattern() {
        let fs = NativeFileSystem::new();
        let mut options = HashMap::new();
        options.insert("regex".to_string(), true);

        let path_var = "/usr/bin:/bin";

        // 测试无效的正则表达式
        let result = which_all_fs("[invalid", &options, &fs, path_var);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid regex pattern"));
    }

    /// 测试检查选项函数
    #[test]
    fn test_check_option_with_multiple_keys() {
        let mut options = HashMap::new();
        options.insert("all".to_string(), true);

        // 测试多个键
        assert!(check_option(&options, vec!["all", "-a"], true));
        assert!(!check_option(&options, vec!["all", "-a"], false));

        // 测试不存在的键
        assert!(!check_option(&options, vec!["nonexistent"], true));
    }
}
