# Which

一个跨平台的 Rust 实现 `which` 命令行工具，兼容 Windows、Linux 和 macOS。

## 功能特性

- 🚀 跨平台支持（Windows、Linux、macOS）
- 🔍 在 PATH 环境变量中定位可执行文件
- 📊 多种输出格式：文本（默认）、JSON、XML
- 🔧 兼容 GNU which 命令行选项
- 📦 使用 Rust 构建，安全高效
- 🎯 Git 集成支持版本追踪
- 🌐 **WebAssembly 支持**：可编译为 WebAssembly 以在浏览器环境中使用
- 💾 **虚拟文件系统支持**：在内存中的虚拟文件系统进行搜索
- ⚡ 并发搜索：使用并行处理实现更快的搜索

## 安装

### 从 Crates.io 安装

```bash
cargo install rust-which
```

### 从源码安装

```bash
git clone https://github.com/VDHewei/which.rs
cd rust-which
cargo build --release
```

二进制文件将位于 `target/release/which`（Windows 上为 `which.exe`）。

## 使用方法

### 基本用法

```bash
which python
```

这将输出 PATH 中第一个匹配 `python` 的可执行文件的完整路径。

### 显示所有匹配项

```bash
which -a python
# 或
which --all python
```

这将输出 PATH 中所有匹配的可执行文件，而不仅仅是第一个。

### 查找多个命令

```bash
which python node git
```

这将同时定位多个命令。

### 输出格式

#### 文本格式（默认）

```bash
which python
# 输出: /usr/bin/python
```

#### JSON 格式

```bash
which -f json python
# 或
which --format json python
```

输出：
```json
{
  "command": "python",
  "paths": [
    "/usr/bin/python"
  ],
  "found": true
}
```

#### XML 格式

```bash
which -f xml python
# 或
which --format xml python
```

输出：
```xml
<WhichResult>
  <command>python</command>
  <paths>/usr/bin/python</paths>
  <found>true</found>
</WhichResult>
```

### 版本信息

```bash
which --version
```

这将显示版本信息，包括：
- 包名和版本
- Git 提交哈希（如果从 git 仓库构建）
- Git 分支名（如果从 git 仓库构建）

## 命令行选项

| 选项 | 简写 | 描述 |
|------|------|------|
| `--all` | `-a` | 显示 PATH 中所有匹配项 |
| `--format` | `-f` | 输出格式：text、json 或 xml |
| `--version` | | 显示版本信息 |
| `--help` | `-h` | 显示帮助信息 |

## 示例

### 查找 Python 可执行文件

```bash
$ which python
/usr/bin/python
```

### 查找所有 Python 可执行文件

```bash
$ which -a python
/usr/bin/python
/usr/local/bin/python3
```

### 多命令 JSON 输出

```bash
$ which -f json python node
[
  {
    "command": "python",
    "paths": ["/usr/bin/python"],
    "found": true
  },
  {
    "command": "node",
    "paths": ["/usr/local/bin/node"],
    "found": true
  }
]
```

### 检查命令是否存在

```bash
$ which python3 && echo "Python 3 已安装"
/usr/bin/python3
Python 3 已安装

$ which nonexistent || echo "命令未找到"
命令未找到
```

## 构建

### 系统要求

- Rust 1.70 或更高版本
- Git（用于获取版本信息）

### 构建步骤

```bash
# 克隆仓库
git clone https://github.com/VDHewei/which.rs
cd rust-which

# 构建项目
cargo build --release

# 运行测试
cargo test

# 本地安装
cargo install --path .
```

### Git 信息

如果项目是 Git 仓库，构建脚本会自动收集 Git 信息（提交哈希、分支）。无需特殊构建标志。

## 平台特定行为

### Windows

- 搜索具有 PATHEXT 定义的可执行文件（.exe、.bat、.cmd 等）
- 使用分号 (;) 作为 PATH 分隔符
- 文件扩展名不区分大小写匹配

### Linux/macOS

- 检查文件的可执行权限
- 使用冒号 (:) 作为 PATH 分隔符
- 区分大小写匹配

## 与 GNU which 的对比

此实现旨在与 GNU which 命令兼容。主要区别如下：

- 添加了 JSON 和 XML 输出格式
- Rust 实现以获得更好的安全性和性能
- 扩展了带有 Git 集成的版本信息
- 支持一次查询多个命令

## 贡献

欢迎贡献！请随时提交 Pull Request。

## 测试

运行测试套件：

```bash
cargo test
```

运行测试并显示输出：

```bash
cargo test -- --nocapture
```

## 开源协议

MIT License - 详见 LICENSE 文件

## 高级功能

### 虚拟文件系统支持

本库支持在虚拟（内存中）文件系统中搜索，这对于测试和需要模拟文件系统操作的场景非常有用。

```rust
use std::collections::HashMap;
use which::core::core::which_fs;
use which::core::filesystem::VirtualFileSystem;

fn main() -> anyhow::Result<()> {
    // 创建虚拟文件系统
    let vfs = VirtualFileSystem::new();
    
    // 添加虚拟文件
    vfs.add_files(vec![
        ("/usr/bin/myapp", true),
        ("/bin/myapp", true),
    ]);
    
    // 在虚拟文件系统中搜索
    let options = HashMap::new();
    let path_var = "/usr/bin:/bin";
    let path = which_fs("myapp", &options, &vfs, path_var)?;
    
    println!("找到于: {}", path.display());
    Ok(())
}
```

运行虚拟文件系统示例：

```bash
cargo run --example virtual_fs
```

### WebAssembly 支持

本库可以编译为 WebAssembly，以便在浏览器环境中使用。

编译为 WebAssembly：

```bash
# 如果尚未安装 wasm-pack，请先安装
cargo install wasm-pack

# 构建 wasm 包
wasm-pack build --dev

# 或构建 wasm 示例
cargo build --target wasm32-unknown-unknown --example wasm --features wasm
```

在 JavaScript 中使用：

```javascript
import init, { find_command, find_all_commands } from './pkg/which.js';

async function main() {
    await init();
    
    // 查找单个命令
    const result = find_command("node", "/usr/local/bin:/usr/bin", false);
    console.log(result.found, result.paths);
    
    // 查找所有匹配项
    const allResults = find_all_commands("python", "/usr/bin:/usr/local/bin", true);
    console.log(allResults.found, allResults.paths);
}

main();
```

### 库 API

你也可以将此库作为 Rust 依赖使用：

```toml
[dependencies]
rust-which = "0.1"
```

```rust
use which::{which_all, which_fs};
use which::core::filesystem::VirtualFileSystem;
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    // 在本地文件系统中搜索
    let options = HashMap::new();
    let paths = which_all("rustc", &options)?;
    for path in paths {
        println!("{}", path.display());
    }
    
    // 或在虚拟文件系统中搜索
    let vfs = VirtualFileSystem::new();
    vfs.add_file("/usr/bin/myapp", true);
    let path = which_fs("myapp", &options, &vfs, "/usr/bin")?;
    println!("{}", path.display());
    
    Ok(())
}
```

## 致谢

- 灵感来源于 GNU `which` 命令
- 使用 [Rust](https://www.rust-lang.org/) 构建
- 使用 [clap](https://github.com/clap-rs/clap) 进行命令行解析
- 使用 [serde](https://serde.rs/) 进行 JSON/XML 序列化
- 使用 [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen) 实现 WebAssembly 支持

## 依赖说明
```
vergen 9.1.0 不再支持 git 和 gitcl 特性，只支持 si 特性
vergen 8.3.2 完全支持 git 和 gitcl 特性，符合 build.rs 的需求
```