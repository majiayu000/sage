#!/bin/zsh
cd /Users/lifcc/Desktop/code/AI/agent/sage/swebench_eval

echo "$(date): Starting retry for 143 instances without valid patches..."

# Read retry instances into array
INSTANCES=("${(@f)$(cat retry_instances.txt)}")

echo "Retrying ${#INSTANCES[@]} instances..."

nohup uv run run_agent.py \
    --instances "${INSTANCES[@]}" \
    --output predictions_retry.json \
    --timeout 900 \
    --max-retries 2 \
    > swebench_retry.log 2>&1 &

echo "Started with PID: $!"
echo "Monitor: tail -f swebench_retry.log"
