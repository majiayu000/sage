#!/bin/zsh
# Correct way to pass instance IDs as separate arguments (zsh version)

# Read all missing instances into an array
INSTANCES=("${(@f)$(cat missing_instances.txt)}")

echo "Total missing instances: ${#INSTANCES[@]}"
echo "First few: ${INSTANCES[@]:0:5}"

# Run with array expansion
nohup uv run run_agent.py \
    --instances "${INSTANCES[@]}" \
    --output predictions_missing.json \
    --timeout 900 \
    --max-retries 1 \
    > swebench_missing.log 2>&1 &

echo "Started with PID: $!"
echo "Monitor: tail -f swebench_missing.log"
