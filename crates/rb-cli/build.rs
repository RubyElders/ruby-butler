use std::env;

fn main() {
    // Only run this in release builds to avoid copying during development
    let profile = env::var("PROFILE").unwrap_or_default();
    if profile != "release" {
        return;
    }

    println!("cargo:rerun-if-changed=src/");

    // This will run after the binary is built
    println!("cargo:warning=Build script executed for release build");
}
