# Which

一个跨平台的 Rust 实现 `which` 命令行工具，兼容 Windows、Linux 和 macOS。

## 功能特性

- 🚀 跨平台支持（Windows、Linux、macOS）
- 🔍 在 PATH 环境变量中定位可执行文件
- 📊 多种输出格式：文本（默认）、JSON、XML
- 🔧 兼容 GNU which 命令行选项
- 📦 使用 Rust 构建，安全高效
- 🎯 Git 集成支持版本追踪

## 安装

### 从 Crates.io 安装

```bash
cargo install rust-which
```

### 从源码安装

```bash
git clone https://github.com/yourusername/rust-which.git
cd rust-which
cargo build --release
```

二进制文件将位于 `target/release/rust-which`（Windows 上为 `rust-which.exe`）。

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
git clone https://github.com/yourusername/rust-which.git
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

## 致谢

- 灵感来源于 GNU `which` 命令
- 使用 [Rust](https://www.rust-lang.org/) 构建
- 使用 [clap](https://github.com/clap-rs/clap) 进行命令行解析
- 使用 [serde](https://serde.rs/) 进行 JSON/XML 序列化

## 依赖说明
```
vergen 9.1.0 不再支持 git 和 gitcl 特性，只支持 si 特性
vergen 8.3.2 完全支持 git 和 gitcl 特性，符合 build.rs 的需求
```