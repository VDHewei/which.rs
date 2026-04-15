//! 虚拟文件系统示例
//!
//! 这个示例展示了如何使用虚拟文件系统来查找命令

use std::collections::HashMap;
use which::core::core::which_fs;
use which::core::core::which_all_fs;
use which::core::filesystem::VirtualFileSystem;

fn main() -> anyhow::Result<()> {
    println!("=== 虚拟文件系统示例 ===\n");

    // 创建虚拟文件系统
    let vfs = VirtualFileSystem::new();
    
    // 添加一些虚拟文件
    vfs.add_files(vec![
        ("/usr/bin/ls", true),
        ("/usr/bin/cat", true),
        ("/usr/bin/grep", true),
        ("/usr/bin/sed", true),
        ("/usr/bin/awk", true),
        ("/bin/ls", true),
        ("/bin/cat", true),
        ("/home/user/script.sh", true),
        ("/home/user/config.toml", false),
    ]);

    println!("已添加虚拟文件:");
    println!("  - /usr/bin/ls (executable)");
    println!("  - /usr/bin/cat (executable)");
    println!("  - /usr/bin/grep (executable)");
    println!("  - /bin/ls (executable)");
    println!("  - /bin/cat (executable)");
    println!("  - /home/user/script.sh (executable)");
    println!("  - /home/user/config.toml (not executable)\n");

    // 示例 1: 查找单个命令
    println!("示例 1: 查找 'ls' 命令");
    let options = HashMap::new();
    // Use correct separator based on platform
    let path_var = if cfg!(windows) { "/usr/bin;/bin" } else { "/usr/bin:/bin" };
    
    match which_fs("ls", &options, &vfs, path_var) {
        Ok(path) => println!("找到: {}\n", path.display()),
        Err(e) => println!("未找到: {}\n", e),
    }

    // 示例 2: 查找所有匹配的命令
    println!("示例 2: 查找所有 'ls' 命令");
    let mut options_all = HashMap::new();
    options_all.insert("all".to_string(), true);
    let path_var = if cfg!(windows) { "/usr/bin;/bin" } else { "/usr/bin:/bin" };
    
    match which_all_fs("ls", &options_all, &vfs, path_var) {
        Ok(paths) => {
            println!("找到 {} 个路径:", paths.len());
            for path in paths {
                println!("  - {}", path.display());
            }
            println!();
        }
        Err(e) => println!("未找到: {}\n", e),
    }

    // 示例 3: 查找不存在的命令
    println!("示例 3: 查找不存在的命令");
    match which_fs("nonexistent", &options, &vfs, path_var) {
        Ok(path) => println!("找到: {}\n", path.display()),
        Err(e) => println!("未找到: {}\n", e),
    }

    // 示例 4: 查找不可执行的文件
    println!("示例 4: 查找不可执行的文件");
    let path_var_config = "/home/user";
    match which_fs("config.toml", &options, &vfs, path_var_config) {
        Ok(path) => println!("找到: {}\n", path.display()),
        Err(e) => println!("未找到或不可执行: {}\n", e),
    }

    // 示例 5: 使用相对路径
    println!("示例 5: 设置当前目录并查找相对路径");
    let mut vfs_relative = VirtualFileSystem::new();
    vfs_relative.set_current_dir("/home/user");
    vfs_relative.add_file("/home/user/myscript", true);
    
    let path_var_relative = ".";
    match which_fs("myscript", &options, &vfs_relative, path_var_relative) {
        Ok(path) => println!("找到: {}\n", path.display()),
        Err(e) => println!("未找到: {}\n", e),
    }

    // 示例 6: 带路径的命令
    println!("示例 6: 查找带路径的命令");
    match which_fs("/usr/bin/grep", &options, &vfs, path_var) {
        Ok(path) => println!("找到: {}\n", path.display()),
        Err(e) => println!("未找到: {}\n", e),
    }

    Ok(())
}
