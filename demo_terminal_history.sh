#!/bin/bash
# Demo: Terminal History Preservation in Sage
#
# This script demonstrates that Sage now preserves terminal history
# just like Claude Code, allowing scrolling to see content from before
# the app started.

set -e

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║  Terminal History Preservation Demo - Sage (like Claude Code) ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""
echo "This demo shows that Sage uses inline mode (not alternate screen)"
echo "which means terminal history is preserved."
echo ""

# Print some content before Sage starts
echo "────────────────────────────────────────────────────────────────"
echo "CONTENT BEFORE SAGE STARTS:"
echo "────────────────────────────────────────────────────────────────"
echo ""

for i in {1..20}; do
    echo "$(printf '%02d' $i). Pre-Sage line - This was here BEFORE Sage started"
done

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "Now launching Sage with a simple task..."
echo "────────────────────────────────────────────────────────────────"
echo ""

# Run Sage in print mode with a simple task
echo "Task: List files in current directory"
timeout 10s ./target/release/sage -p "List the files in the current directory" || true

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "CONTENT AFTER SAGE EXITS:"
echo "────────────────────────────────────────────────────────────────"
echo ""

for i in {1..10}; do
    echo "$(printf '%02d' $i). Post-Sage line - This was printed AFTER Sage exited"
done

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║                    ✅ DEMO COMPLETE                            ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""
echo "Manual Verification Checklist:"
echo ""
echo "  ☐ Scroll up in your terminal (mouse wheel or Shift+PageUp)"
echo "  ☐ You should see all 20 \"Pre-Sage\" lines above"
echo "  ☐ Sage's output should be visible in the middle"
echo "  ☐ All \"Post-Sage\" lines should be at the bottom"
echo "  ☐ NO content should have disappeared"
echo ""
echo "If you can see ALL content by scrolling, then:"
echo ""
echo "  ✅ Terminal history preservation is WORKING!"
echo "  ✅ Sage is using inline mode correctly!"
echo "  ✅ No alternate screen buffer was used!"
echo ""
echo "Compare this with traditional full-screen apps (vim, less, htop)"
echo "which use alternate screen and lose the 'Pre-Sage' content."
echo ""
