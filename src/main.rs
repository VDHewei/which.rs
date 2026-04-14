mod core;

use anyhow::Result;
use clap::Parser;
use core::core::{which_all};
use std::collections::HashMap;
use serde::Serialize;
use std::env;

/// Which version information
#[derive(Debug, Serialize)]
struct VersionInfo {
    name: String,
    version: String,
    git_commit: Option<String>,
    git_branch: Option<String>,
    build_date: Option<String>,
}

impl VersionInfo {
    fn new() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_commit: option_env!("VERGEN_GIT_SHA").map(|s| s.to_string()),
            git_branch: option_env!("VERGEN_GIT_BRANCH").map(|s| s.to_string()),
            build_date: option_env!("VERGEN_BUILD_DATE").map(|s| s.to_string()),
        }
    }

    fn display(&self) -> String {
        let mut info = format!("{} {}", self.name, self.version);
        if let Some(commit) = &self.git_commit {
            info.push_str(&format!(" (commit: {})", commit));
        }
        if let Some(branch) = &self.git_branch {
            info.push_str(&format!(" (branch: {})", branch));
        }
        if let Some(date) = &self.build_date {
            info.push_str(&format!(" (built: {})", date));
        }
        info
    }
}

/// Which result for JSON/XML serialization
#[derive(Debug, Serialize)]
struct WhichResult {
    command: String,
    paths: Vec<String>,
    found: bool,
}

impl WhichResult {
    fn new(command: &str, paths: Vec<std::path::PathBuf>) -> Self {
        let found = !paths.is_empty();
        let path_strings: Vec<String> = paths.iter()
            .filter_map(|p| p.to_str().map(|s| s.to_string()))
            .collect();
        
        Self {
            command: command.to_string(),
            paths: path_strings,
            found,
        }
    }
}

/// A Rust implementation of the 'which' command-line utility
#[derive(Parser, Debug)]
#[command(name = "which")]
#[command(author = env!("CARGO_PKG_AUTHORS"))]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Locate a command", long_about = None)]
struct Args {
    /// Show all matches in PATH, not just the first
    #[arg(short = 'a', long = "all")]
    all: bool,

    /// Output format: text (default), json, or xml
    #[arg(short = 'f', long = "format", default_value = "text")]
    format: String,

    /// Show version information
    #[arg(long = "version")]
    version: bool,

    /// Command to locate
    command: Vec<String>,
}

/// Run the which command with given arguments
fn run(args: Args) -> Result<()> {
    // Show version and exit if requested
    if args.version {
        let version_info = VersionInfo::new();
        println!("{}", version_info.display());
        return Ok(());
    }

    // Require at least one command
    if args.command.is_empty() {
        anyhow::bail!("missing operand: please provide at least one command to locate");
    }

    // Prepare options
    let mut options = HashMap::new();
    options.insert("all".to_string(), args.all);
    options.insert("-a".to_string(), args.all);

    // Process each command
    let mut all_results = Vec::new();
    for cmd in &args.command {
        let result = which_all(cmd, &options);
        
        match result {
            Ok(paths) => {
                all_results.push(WhichResult::new(cmd, paths));
            }
            Err(e) => {
                // For format other than text, we still want to report not found
                if args.format != "text" {
                    all_results.push(WhichResult::new(cmd, vec![]));
                } else {
                    // Text format: print error to stderr
                    eprintln!("{}: {}", cmd, e);
                }
            }
        }
    }

    // Output results in the requested format
    match args.format.as_str() {
        "json" => {
            if all_results.len() == 1 {
                println!("{}", serde_json::to_string_pretty(&all_results[0])?);
            } else {
                println!("{}", serde_json::to_string_pretty(&all_results)?);
            }
        }
        "xml" => {
            if all_results.len() == 1 {
                println!("{}", quick_xml::se::to_string(&all_results[0])?);
            } else {
                println!("<results>");
                for result in &all_results {
                    println!("{}", quick_xml::se::to_string(result)?);
                }
                println!("</results>");
            }
        }
        "text" => {
            for result in &all_results {
                if result.found {
                    for path in &result.paths {
                        println!("{}", path);
                    }
                }
            }
        }
        _ => {
            anyhow::bail!("unsupported format: {}. Use 'text', 'json', or 'xml'", args.format);
        }
    }

    Ok(())
}
#[warn(dead_code)]
fn main() {
    let args = Args::parse();

    if let Err(e) = run(args) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info_display() {
        let version = VersionInfo::new();
        let display = version.display();
        assert!(display.contains("which"));
    }

    #[test]
    fn test_which_result_new() {
        let result = WhichResult::new("test", vec![
            std::path::PathBuf::from("/usr/bin/test"),
            std::path::PathBuf::from("/bin/test")
        ]);
        assert_eq!(result.command, "test");
        assert_eq!(result.paths.len(), 2);
        assert!(result.found);
    }

    #[test]
    fn test_which_result_not_found() {
        let result = WhichResult::new("nonexistent", vec![]);
        assert_eq!(result.command, "nonexistent");
        assert_eq!(result.paths.len(), 0);
        assert!(!result.found);
    }
}
