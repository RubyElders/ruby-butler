use std::env;
use std::process::Command;

fn main() {
    // Capture git information for version embedding
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/");

    // Get git commit hash
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Get git tag (if on a tagged commit)
    let git_tag = Command::new("git")
        .args(["describe", "--tags", "--exact-match", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string());

    // Check if working directory is dirty
    let git_dirty = Command::new("git")
        .args(["diff", "--quiet", "HEAD"])
        .output()
        .map(|output| !output.status.success())
        .unwrap_or(false);

    // Set environment variables for the binary
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    
    if let Some(tag) = git_tag {
        println!("cargo:rustc-env=GIT_TAG={}", tag);
    }

    if git_dirty {
        println!("cargo:rustc-env=GIT_DIRTY=true");
    }

    // Build profile information
    let profile = env::var("PROFILE").unwrap_or_default();
    println!("cargo:rustc-env=BUILD_PROFILE={}", profile);
    
    if profile == "release" {
        println!("cargo:warning=Build script executed for release build");
    }
}
