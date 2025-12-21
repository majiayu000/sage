#!/bin/bash
# Docker-based SWE-bench evaluation

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_NAME="sage-swebench-eval"

# Build Docker image
echo "Building Docker image..."
docker build -t "$IMAGE_NAME" "$SCRIPT_DIR"

# Run evaluation
echo "Running evaluation..."
docker run --rm \
    -v "$SCRIPT_DIR/predictions.json:/eval/predictions.json:ro" \
    -v "$SCRIPT_DIR/results:/eval/results" \
    "$IMAGE_NAME" \
    evaluate predictions.json "$@"

echo "Results saved to: $SCRIPT_DIR/results/"
