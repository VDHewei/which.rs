fn main() {
    // Always emit basic information
    println!("cargo:rerun-if-changed=build.rs");
    
    // Generate version information
    let mut args = vergen::EmitBuilder::builder();
    
    // Try to emit git information if available
    let _ = args.all_git().emit();
}
