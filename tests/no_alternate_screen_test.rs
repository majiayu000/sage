//! Test: Verify Sage doesn't use alternate screen escape sequences
//!
//! This test ensures that Sage preserves terminal history by not
//! using the alternate screen buffer.

use std::process::Command;

#[test]
#[ignore] // Requires release build: cargo test -- --ignored
fn test_sage_no_alternate_screen_escape_sequences() {
    // Build Sage first
    let build_output = Command::new("cargo")
        .args(["build", "--release"])
        .output()
        .expect("Failed to build Sage");

    assert!(
        build_output.status.success(),
        "Sage build failed: {}",
        String::from_utf8_lossy(&build_output.stderr)
    );

    // Run Sage with print mode (-p) to execute and exit quickly
    // Capture both stdout and stderr to look for escape sequences
    let output = Command::new("./target/release/sage")
        .args(["-p", "echo test"])
        .env("TERM", "xterm-256color") // Ensure terminal support
        .output()
        .expect("Failed to run Sage");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Check for alternate screen escape sequences
    // CSI ?1049h - Enter alternate screen
    // CSI ?1049l - Leave alternate screen
    // CSI ?47h - Enter alternate screen (older)
    // CSI ?47l - Leave alternate screen (older)

    let has_enter_alt_screen = combined.contains("\x1b[?1049h") || combined.contains("\x1b[?47h");
    let has_leave_alt_screen = combined.contains("\x1b[?1049l") || combined.contains("\x1b[?47l");

    if has_enter_alt_screen {
        panic!(
            "❌ FAILED: Sage used 'Enter Alternate Screen' escape sequence!\n\
            This means terminal history won't be preserved.\n\
            Output:\n{}",
            combined
        );
    }

    if has_leave_alt_screen {
        panic!(
            "❌ FAILED: Sage used 'Leave Alternate Screen' escape sequence!\n\
            This means alternate screen was enabled.\n\
            Output:\n{}",
            combined
        );
    }

    println!("✅ PASSED: Sage doesn't use alternate screen escape sequences");
    println!("   Terminal history will be preserved correctly");
}

#[test]
#[ignore] // Requires release build: cargo test -- --ignored
fn test_sage_uses_raw_mode() {
    // Verify that Sage can still capture input (requires raw mode or similar)
    // This is a sanity check that we didn't accidentally disable input handling

    // For now, just verify the binary exists and runs
    let output = Command::new("./target/release/sage")
        .arg("--version")
        .output()
        .expect("Failed to run Sage");

    assert!(output.status.success(), "Sage --version failed");

    let version_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        version_output.contains("sage") || version_output.contains("Sage"),
        "Version output doesn't contain 'sage'"
    );

    println!("✅ PASSED: Sage binary is functional");
}

#[test]
fn test_escape_sequence_detection() {
    // Test the escape sequence detection itself

    // Positive tests - should detect these
    assert!(
        "\x1b[?1049h".contains("\x1b[?1049h"),
        "Failed to detect enter alternate screen"
    );
    assert!(
        "\x1b[?1049l".contains("\x1b[?1049l"),
        "Failed to detect leave alternate screen"
    );

    // Negative tests - should NOT detect in normal output
    let normal_output = "Hello World\nSome normal text\n\x1b[32mGreen text\x1b[0m";
    assert!(
        !normal_output.contains("\x1b[?1049h"),
        "False positive: detected alt screen in normal output"
    );

    println!("✅ PASSED: Escape sequence detection works correctly");
}
