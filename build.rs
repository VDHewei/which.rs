fn main() {
    // Always emit basic information
    println!("cargo:rerun-if-changed=build.rs");
    
    // Generate version information
    let mut args = vergen::EmitBuilder::builder();
    
    // Try to emit git information if available
    // Use unwrap_or_else to suppress warnings when git is not configured
    let _ = args.all_git().emit().ok();
}
