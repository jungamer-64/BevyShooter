// build.rs
// Build script for lightweight build diagnostics and asset change tracking.
//
// This works by:
// 1. Detecting the active Cargo profile
// 2. Re-running when assets change
// 3. Emitting a short build note for easier troubleshooting

fn main() {
    // Tell Cargo to rerun this script if the profile changes
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-changed=assets");

    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    // Print build info
    println!("cargo:warning=Building with profile: {}", profile);
}
