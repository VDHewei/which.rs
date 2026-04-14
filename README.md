# Which-rs

A cross-platform Rust implementation of the `which` command-line utility, compatible with Windows, Linux, and macOS.

## Features

- 🚀 Cross-platform support (Windows, Linux, macOS)
- 🔍 Locate executable files in your PATH
- 📊 Multiple output formats: text (default), JSON, XML
- 🔧 Compatible with GNU which command-line options
- 📦 Built with Rust for safety and performance
- 🎯 Git integration for version tracking

## Installation

### From Crates.io

```bash
cargo install which-rs
```

### From Source

```bash
git clone https://github.com/yourusername/which-rs.git
cd which-rs
cargo build --release
```

The binary will be available at `target/release/which` (or `which.exe` on Windows).

## Usage

### Basic Usage

```bash
which python
```

This will print the full path to the first executable matching `python` in your PATH.

### Show All Matches

```bash
which -a python
# or
which --all python
```

This will print all matching executables in your PATH, not just the first one.

### Multiple Commands

```bash
which python node git
```

This will locate multiple commands at once.

### Output Formats

#### Text (default)

```bash
which python
# Output: /usr/bin/python
```

#### JSON

```bash
which -f json python
# or
which --format json python
```

Output:
```json
{
  "command": "python",
  "paths": [
    "/usr/bin/python"
  ],
  "found": true
}
```

#### XML

```bash
which -f xml python
# or
which --format xml python
```

Output:
```xml
<WhichResult>
  <command>python</command>
  <paths>/usr/bin/python</paths>
  <found>true</found>
</WhichResult>
```

### Version Information

```bash
which --version
```

This will display version information including:
- Package name and version
- Git commit hash (if built from git repository)
- Git branch name (if built from git repository)
- Build date

## Command-Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--all` | `-a` | Show all matches in PATH |
| `--format` | `-f` | Output format: text, json, or xml |
| `--version` | | Show version information |
| `--help` | `-h` | Display help message |

## Examples

### Find Python executable

```bash
$ which python
/usr/bin/python
```

### Find all Python executables

```bash
$ which -a python
/usr/bin/python
/usr/local/bin/python3
```

### Multiple commands with JSON output

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

### Check if a command exists

```bash
$ which python3 && echo "Python 3 is installed"
/usr/bin/python3
Python 3 is installed

$ which nonexistent || echo "Command not found"
Command not found
```

## Building

### Requirements

- Rust 1.70 or later
- Git (for version information)

### Build Steps

```bash
# Clone the repository
git clone https://github.com/yourusername/which-rs.git
cd which-rs

# Build the project
cargo build --release

# Run tests
cargo test

# Install locally
cargo install --path .
```

### Build with Git Information

The build script automatically collects Git information (commit hash, branch) if the project is a Git repository:

```bash
cargo build --features git
```

## Platform-Specific Behavior

### Windows

- Searches for executables with extensions defined in PATHEXT (.exe, .bat, .cmd, etc.)
- Uses semicolon (;) as PATH separator
- Case-insensitive matching for file extensions

### Linux/macOS

- Checks for executable permissions on files
- Uses colon (:) as PATH separator
- Case-sensitive matching

## Comparison with GNU which

This implementation aims to be compatible with the GNU which command. The main differences are:

- Added JSON and XML output formats
- Rust implementation for better safety and performance
- Extended version information with Git integration
- Support for querying multiple commands at once

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Testing

Run the test suite:

```bash
cargo test
```

Run tests with output:

```bash
cargo test -- --nocapture
```

## License

MIT License - see LICENSE file for details

## Acknowledgments

- Inspired by the GNU `which` command
- Built with [Rust](https://www.rust-lang.org/)
- Uses [clap](https://github.com/clap-rs/clap) for command-line parsing
- Uses [serde](https://serde.rs/) for JSON/XML serialization
