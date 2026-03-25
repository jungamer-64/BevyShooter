// build.rs
// Build script to conditionally enable bevy_log only in debug builds
//
// This works by:
// 1. Detecting the build profile (debug vs release)
// 2. Setting a cfg flag that main.rs can use
// 3. Cargo.toml uses optional bevy_log feature controlled by "dev-logging" feature

fn main() {
    // Tell Cargo to rerun this script if the profile changes
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-changed=assets");
    
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    
    // Set cfg flag based on profile
    if profile == "debug" {
        println!("cargo:rustc-cfg=enable_logging");
    }
    
    // Print build info
    println!("cargo:warning=Building with profile: {}", profile);
}
