#!/bin/bash
# Manual test for terminal history preservation
#
# This script tests that Sage:
# 1. Allows scrolling to see content before the app started
# 2. Preserves output in terminal history after exiting
#
# Usage: ./tests/terminal_history_manual_test.sh

set -e

echo "================================"
echo "Terminal History Test for Sage"
echo "================================"
echo ""
echo "This test will:"
echo "1. Print 30 lines of content"
echo "2. Launch Sage (it will immediately exit)"
echo "3. Print confirmation message"
echo ""
echo "Expected behavior:"
echo "- ✅ You should be able to scroll up to see all 30 lines"
echo "- ✅ Sage's output should remain visible in terminal"
echo "- ✅ No content should disappear"
echo ""
read -p "Press Enter to continue..."
echo ""

# Print 30 lines of test content
echo "=== BEFORE SAGE ==="
for i in {1..30}; do
    echo "Line $i: This content was printed BEFORE Sage started"
done
echo ""

# Launch Sage with a simple task (will run and exit)
echo "=== LAUNCHING SAGE ===" echo ""
# Create a test directory
TEST_DIR="/tmp/sage_history_test_$(date +%s)"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

# Run Sage with a simple task that exits immediately
echo "Task: Print 'Hello from Sage' and exit"
timeout 10s ../../../target/release/sage -p "Print a simple greeting message" || true

echo ""
echo "=== AFTER SAGE ==="
for i in {1..10}; do
    echo "Line $i: This content was printed AFTER Sage exited"
done
echo ""

echo "================================"
echo "Test Complete!"
echo "================================"
echo ""
echo "Manual Verification Steps:"
echo "1. Scroll up in your terminal (using mouse wheel or Shift+PageUp)"
echo "2. Verify you can see the 30 lines printed BEFORE Sage"
echo "3. Verify Sage's output is still visible"
echo "4. Verify all content remains in terminal history"
echo ""
echo "If all content is visible and scrollable, the test PASSED ✅"
echo "If content disappeared or alternate screen was used, the test FAILED ❌"
echo ""

# Cleanup
cd - > /dev/null
rm -rf "$TEST_DIR"
