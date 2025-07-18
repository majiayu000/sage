#!/bin/bash

# Test script for interrupt functionality
echo "Testing Sage Agent interrupt functionality..."

# Create a test configuration file
cat > test_config.json << 'EOF'
{
  "default_provider": "openai",
  "providers": {
    "openai": {
      "api_key": "test-key",
      "model": "gpt-3.5-turbo",
      "base_url": "https://api.openai.com/v1"
    }
  },
  "max_steps": 10,
  "max_tokens": 1000
}
EOF

echo "Created test configuration file: test_config.json"

# Test 1: Interactive mode (manual test)
echo ""
echo "=== Test 1: Interactive Mode ==="
echo "This will start sage in interactive mode."
echo "To test interrupt:"
echo "1. Enter a task like 'write a long story'"
echo "2. Press Ctrl+C during execution"
echo "3. Verify that the task stops but sage continues running"
echo "4. Type 'exit' to quit"
echo ""
echo "Press Enter to start interactive mode test..."
read -r

./target/debug/sage --config-file test_config.json

echo ""
echo "=== Test 2: Run Mode ==="
echo "This will start a task in run mode."
echo "Press Ctrl+C during execution to test interrupt."
echo ""
echo "Press Enter to start run mode test..."
read -r

./target/debug/sage run "Write a very long detailed story about a robot learning to paint. Include many characters and plot twists. Make it at least 2000 words long." --config-file test_config.json

echo ""
echo "=== Test Complete ==="
echo "Cleaning up test files..."
rm -f test_config.json

echo "Test completed!"
