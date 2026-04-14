use anyhow::{Error, anyhow};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

/// 检查选项是否存在于 options 中且值为期望值
fn check_option(options: &HashMap<String, bool>, keys: Vec<&str>, expected: bool) -> bool {
    for key in keys {
        if let Some(&value) = options.get(key) {
            return value == expected;
        }
    }
    false
}

/// 获取 Windows 平台的可执行文件后缀列表
#[cfg(target_os = "windows")]
fn get_executable_extensions() -> Vec<String> {
    let pathext = env::var("PATHEXT").unwrap_or_else(|_| {
        ".COM;.EXE;.BAT;.CMD;.VBS;.VBE;.JS;.JSE;.WSF;.WSH;.MSC".to_string()
    });
    pathext.split(';')
        .map(|s| s.to_uppercase())
        .collect()
}

/// 检查文件是否可执行（Unix 系统）
#[cfg(unix)]
fn is_executable(path: &PathBuf) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match fs::metadata(path) {
        Ok(metadata) => {
            let mode = metadata.permissions().mode();
            mode & 0o111 != 0
        }
        Err(_) => false,
    }
}

/// 查找所有匹配的命令路径
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
    // 1. 如果命令本身包含路径分隔符（如 ./my_app 或 /usr/bin/git），直接检查
    if cmd.contains('/') || cmd.contains('\\') {
        let path = PathBuf::from(cmd);
        if path.is_file() {
            #[cfg(unix)]
            {
                if is_executable(&path) {
                    return Ok(vec![path.canonicalize()?]);
                }
                return Err(anyhow!("{} is not executable", cmd));
            }
            #[cfg(windows)]
            {
                if path.canonicalize().is_ok() {
                    return Ok(vec![path.canonicalize()?]);
                }
                return Err(anyhow!("{} not found", cmd));
            }
        }
        return Err(anyhow!("{} not found", cmd));
    }

    // 2. 读取 PATH 环境变量
    let mut result = vec![];
    let path_var = env::var("PATH").unwrap_or_default();
    // 根据操作系统确定分隔符 (Windows 是 ';'，Linux/macOS 是 ':')
    let separator = if cfg!(windows) { ';' } else { ':' };

    // 3. 遍历 PATH 中的每个目录
    for dir in path_var.split(separator) {
        let dir_path = PathBuf::from(dir);
        let candidate = dir_path.join(cmd);

        // 4. 检查文件是否存在且可执行
        #[cfg(unix)]
        {
            if candidate.is_file() && is_executable(&candidate) {
                if let Ok(canonical) = candidate.canonicalize() {
                    result.push(canonical);
                    // 如果没有 --all 选项，找到第一个就返回
                    if !check_option(options, vec!["all", "-a"], true) {
                        return Ok(result);
                    }
                }
            }
        }

        #[cfg(windows)]
        {
            // Windows: 先检查带扩展名的
            let extensions = get_executable_extensions();
            
            // 首先检查是否有扩展名的文件
            if candidate.is_file() {
                let candidate_str = candidate.to_string_lossy().to_uppercase();
                if extensions.iter().any(|ext| candidate_str.ends_with(ext)) {
                    if let Ok(canonical) = candidate.canonicalize() {
                        result.push(canonical);
                        if !check_option(options, vec!["all", "-a"], true) {
                            return Ok(result);
                        }
                    }
                }
            }
            
            // 然后尝试所有常见的扩展名
            for ext in &extensions {
                let candidate_ext = dir_path.join(format!("{}{}", cmd, ext));
                if candidate_ext.is_file() {
                    if let Ok(canonical) = candidate_ext.canonicalize() {
                        // 避免重复添加
                        if !result.iter().any(|p| p == &canonical) {
                            result.push(canonical);
                            if !check_option(options, vec!["all", "-a"], true) {
                                return Ok(result);
                            }
                        }
                    }
                }
            }
        }
    }

    if !result.is_empty() {
        return Ok(result);
    }
    Err(anyhow!("{} not found", cmd))
}

/// 查找第一个匹配的命令路径（默认行为）
/// 
/// # 参数
/// * `cmd` - 要查找的命令名称
/// * `options` - 选项映射表（目前未使用，保留用于未来扩展）
/// 
/// # 返回值
/// 成功时返回找到的第一个路径，失败时返回错误信息
#[allow(dead_code)]
pub fn which(cmd: &str, options: &HashMap<String, bool>) -> Result<PathBuf, Error> {
    let paths = which_all(cmd, options)?;
    paths.into_iter().next()
        .ok_or_else(|| anyhow!("{} not found", cmd))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_which_all_existing_command() {
        // 使用 ls 作为测试命令（Linux/macOS）或 where is（Windows）
        let test_cmd = if cfg!(windows) { "cmd" } else { "ls" };
        let options = HashMap::new();
        let result = which_all(test_cmd, &options);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_which_all_with_all_option() {
        let test_cmd = if cfg!(windows) { "cmd" } else { "ls" };
        let mut options = HashMap::new();
        options.insert("all".to_string(), true);
        let result = which_all(test_cmd, &options);
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
        if result.is_err() {
            // 路径不存在是可能的
            assert!(result.unwrap_err().to_string().contains("not found"));
        }
    }

    #[test]
    fn test_which_single() {
        let test_cmd = if cfg!(windows) { "cmd" } else { "ls" };
        let options = HashMap::new();
        let result = which(test_cmd, &options);
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

    #[test]
    #[cfg(unix)]
    fn test_is_executable() {
        let test_cmd = "/bin/ls";
        let path = PathBuf::from(test_cmd);
        if path.exists() {
            assert!(is_executable(&path));
        }
    }
}
