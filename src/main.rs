mod core;

use anyhow::Result;
use clap::{Parser, CommandFactory};
use core::core::which_all;
use std::collections::HashMap;
use std::time::Duration;
use serde::Serialize;

/// Format XML string with proper indentation
fn format_xml(xml: &str, base_indent: usize) -> String {
    let mut result = String::new();
    let mut indent_level: usize = 0;
    let mut current_tag = String::new();
    let mut chars = xml.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '<' => {
                // Start of a tag
                current_tag.push(c);
                let next_char = chars.peek();
                if let Some(&'/') = next_char {
                    // Closing tag, decrease indent before processing
                    indent_level = indent_level.saturating_sub(1);
                }
            }
            '>' => {
                // End of a tag
                current_tag.push(c);

                // Check if it's a self-closing tag
                let is_self_closing = current_tag.contains("/>") || current_tag.trim_start_matches('<').starts_with("?");
                let is_closing_tag = current_tag.contains("</");

                // Add indentation before the tag
                if !result.is_empty() && !result.ends_with('\n') {
                    result.push('\n');
                }
                result.push_str(&" ".repeat(indent_level * 2 + base_indent));

                // Add the tag
                result.push_str(&current_tag);

                // Reset current tag
                current_tag.clear();

                // If it's an opening tag (not closing or self-closing), increase indent
                if !is_closing_tag && !is_self_closing {
                    indent_level += 1;
                }

                // Check if next character is not '<', meaning there's content
                if let Some(&next_c) = chars.peek() && next_c != '<' {
                    // Collect content
                    let mut content = String::new();
                    while let Some(&next_c) = chars.peek() {
                        if next_c == '<' {
                            break;
                        }
                        content.push(chars.next().unwrap());
                    }

                    // Add content with proper indentation
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        result.push('\n');
                        result.push_str(&" ".repeat((indent_level) * 2 + base_indent));
                        result.push_str(trimmed);
                        indent_level = indent_level.saturating_sub(1);
                    }
                }
            }
            _ => {
                current_tag.push(c);
            }
        }
    }

    result.trim().to_string()
}

/// Format duration into human-readable string
fn format_duration(duration: Duration) -> String {
    let millis = duration.as_millis();
    let seconds = duration.as_secs();
    let minutes = seconds / 60;
    let hours = minutes / 60;

    if hours > 0 {
        let remaining_minutes = minutes % 60;
        if remaining_minutes > 0 {
            format!("{}h {}min", hours, remaining_minutes)
        } else {
            format!("{}h", hours)
        }
    } else if minutes > 0 {
        let remaining_seconds = seconds % 60;
        if remaining_seconds > 0 {
            format!("{}min {}s", minutes, remaining_seconds)
        } else {
            format!("{}min", minutes)
        }
    } else if seconds > 0 {
        let remaining_millis = (millis % 1000) as u64;
        if remaining_millis > 0 {
            format!("{}s {}ms", seconds, remaining_millis)
        } else {
            format!("{}s", seconds)
        }
    } else if millis > 0 {
        format!("{}ms", millis)
    } else {
        format!("{}μs", duration.as_micros())
    }
}

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
    elapsed_time: Option<String>,
}

impl WhichResult {
    fn new_with_time(command: &str, paths: Vec<std::path::PathBuf>, elapsed_time: Option<String>) -> Self {
        let found = !paths.is_empty();
        let path_strings: Vec<String> = paths.iter()
            .filter_map(|p| {
                // On Windows, canonicalize() returns paths with \\?\ prefix
                // Remove it for cleaner output
                let path_str = p.to_str()?;
                Some(path_str.replace(r"\\?\", ""))
            })
            .collect();

        Self {
            command: command.to_string(),
            paths: path_strings,
            found,
            elapsed_time,
        }
    }

}

/// A Rust implementation of the 'which' command-line utility
#[derive(Parser, Debug)]
#[command(name = "which")]
#[command(author = env!("CARGO_PKG_AUTHORS"))]
#[command(version = env!("CARGO_PKG_VERSION"), disable_version_flag = true)]
#[command(about = "Locate a command", long_about = None)]
struct Args {
    /// Show all matches in PATH, not just the first
    #[arg(short = 'a', long = "all")]
    all: bool,

    /// Output format: text (default), json, or xml
    #[arg(short = 'f', long = "format", default_value = "text")]
    format: String,

    /// Show version information
    #[arg(long = "version",short='V')]
    version: bool,

    /// Command to locate
    command: Vec<String>,

    /// Show time elapsed for the search
    #[arg(short = 't', long = "time")]
    time: bool,
}

impl Args {
    pub(crate) fn output(&self, all_results: &[WhichResult]) -> Result<()> {
        match self.format.as_str() {
            "json" => {
                if all_results.len() == 1 {
                    println!("{}", serde_json::to_string_pretty(&all_results[0])?);
                } else {
                    println!("{}", serde_json::to_string_pretty(&all_results)?);
                }
            }
            "xml" => {
                if all_results.len() == 1 {
                    let xml = quick_xml::se::to_string(&all_results[0])?;
                    let formatted = format_xml(&xml, 0);
                    println!("{}", formatted);
                } else {
                    println!("<results>");
                    for result in all_results {
                        let xml = quick_xml::se::to_string(result)?;
                        let formatted = format_xml(&xml, 2);
                        println!("{}", formatted);
                    }
                    println!("</results>");
                }
            }
            "text" => {
                for result in all_results {
                    if result.found {
                        for path in &result.paths {
                            println!("{}", path);
                        }

                        // Show elapsed time if -t flag is enabled
                        if self.time && let Some(time_str) = &result.elapsed_time {
                            println!("Time: {}", time_str);
                        }
                    } else if self.time && let Some(time_str) = &result.elapsed_time {
                        // Show elapsed time for not found commands
                        eprintln!("{} not found (Time: {})", result.command, time_str);
                    }
                }
            }
            _ => {
                anyhow::bail!("unsupported format: {}. Use 'text', 'json', or 'xml'", self.format);
            }
        }
        Ok(())
    }
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
        // 调用 help
        Args::command().print_help().ok();
        std::process::exit(0);
    }

    // Prepare options
    let mut options = HashMap::new();
    options.insert("all".to_string(), args.all);

    // Process each command
    let mut all_results = Vec::new();
    for cmd in &args.command {
        let start = std::time::Instant::now();
        let result = which_all(cmd, &options);
        let elapsed = start.elapsed();

        let elapsed_time_str = if args.time {
            Some(format_duration(elapsed))
        } else {
            None
        };

        match result {
            Ok(paths) => {
                all_results.push(WhichResult::new_with_time(cmd, paths, elapsed_time_str));
            }
            Err(e) => {
                // For format other than text, we still want to report not found
                if args.format != "text" {
                    all_results.push(WhichResult::new_with_time(cmd, vec![], elapsed_time_str));
                } else if !args.time {
                    // Text format without time: print error to stderr
                    eprintln!("{}: {}", cmd, e);
                }
                // With time option, the error message is already printed in output()
            }
        }
    }

    // Output results in the requested format
    args.output(&all_results)?;

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
        let result = WhichResult::new_with_time("test", vec![
            std::path::PathBuf::from("/usr/bin/test"),
            std::path::PathBuf::from("/bin/test")
        ], None);
        assert_eq!(result.command, "test");
        assert_eq!(result.paths.len(), 2);
        assert!(result.found);
    }

    #[test]
    fn test_which_result_not_found() {
        let result = WhichResult::new_with_time("nonexistent", vec![], None);
        assert_eq!(result.command, "nonexistent");
        assert_eq!(result.paths.len(), 0);
        assert!(!result.found);
    }

    #[test]
    fn test_format_duration_microseconds() {
        let duration = Duration::from_micros(500);
        let formatted = format_duration(duration);
        assert_eq!(formatted, "500μs");
    }

    #[test]
    fn test_format_duration_milliseconds() {
        let duration = Duration::from_millis(500);
        let formatted = format_duration(duration);
        assert_eq!(formatted, "500ms");
    }

    #[test]
    fn test_format_duration_seconds() {
        let duration = Duration::from_secs(5);
        let formatted = format_duration(duration);
        assert_eq!(formatted, "5s");
    }

    #[test]
    fn test_format_duration_seconds_with_ms() {
        let duration = Duration::from_millis(5500);
        let formatted = format_duration(duration);
        assert_eq!(formatted, "5s 500ms");
    }

    #[test]
    fn test_format_duration_minutes() {
        let duration = Duration::from_secs(300);
        let formatted = format_duration(duration);
        assert_eq!(formatted, "5min");
    }

    #[test]
    fn test_format_duration_minutes_with_seconds() {
        let duration = Duration::from_secs(330);
        let formatted = format_duration(duration);
        assert_eq!(formatted, "5min 30s");
    }

    #[test]
    fn test_format_duration_hours() {
        let duration = Duration::from_secs(7200);
        let formatted = format_duration(duration);
        assert_eq!(formatted, "2h");
    }

    #[test]
    fn test_format_duration_hours_with_minutes() {
        let duration = Duration::from_secs(7500);
        let formatted = format_duration(duration);
        assert_eq!(formatted, "2h 5min");
    }

    #[test]
    fn test_which_result_with_time() {
        let result = WhichResult::new_with_time("test", vec![
            std::path::PathBuf::from("/usr/bin/test")
        ], Some("10ms".to_string()));
        assert_eq!(result.command, "test");
        assert_eq!(result.paths.len(), 1);
        assert!(result.found);
        assert_eq!(result.elapsed_time, Some("10ms".to_string()));
    }

    #[test]
    fn test_format_xml_simple() {
        let xml = "<WhichResult><command>test</command><found>true</found></WhichResult>";
        let formatted = format_xml(xml, 0);
        assert!(formatted.contains("<command>"));
        assert!(formatted.contains("test"));
        assert!(formatted.contains("</command>"));
    }
}
