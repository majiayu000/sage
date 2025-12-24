#!/bin/bash

# ============================================================
# Sage Agent - Startup Time Benchmark
# Compares startup performance with other code agents
#
# Usage:
#   ./benchmarks/startup.sh
#   ./benchmarks/startup.sh --iterations 20
#   ./benchmarks/startup.sh --json
#
# ============================================================

set -e

# Configuration
ITERATIONS="${1:-10}"
OUTPUT_FORMAT="${2:-text}"  # text or json

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --iterations|-i)
            ITERATIONS="$2"
            shift 2
            ;;
        --json|-j)
            OUTPUT_FORMAT="json"
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -i, --iterations N   Number of iterations (default: 10)"
            echo "  -j, --json           Output in JSON format"
            echo "  -h, --help           Show this help"
            exit 0
            ;;
        *)
            shift
            ;;
    esac
done

# Get high-resolution timestamp (milliseconds)
get_time_ms() {
    if command -v gdate &> /dev/null; then
        # macOS with coreutils
        echo $(($(gdate +%s%N) / 1000000))
    elif command -v date &> /dev/null && date +%N &> /dev/null 2>&1; then
        # Linux
        echo $(($(date +%s%N) / 1000000))
    else
        # Fallback: seconds only (less precise)
        echo $(($(date +%s) * 1000))
    fi
}

# Benchmark a single command
benchmark_command() {
    local name=$1
    local cmd=$2
    local iterations=$3

    local total=0
    local min=999999
    local max=0
    local times=()

    for ((i=1; i<=iterations; i++)); do
        local start=$(get_time_ms)
        eval "$cmd" > /dev/null 2>&1 || true
        local end=$(get_time_ms)
        local elapsed=$((end - start))

        times+=($elapsed)
        total=$((total + elapsed))

        if [ $elapsed -lt $min ]; then min=$elapsed; fi
        if [ $elapsed -gt $max ]; then max=$elapsed; fi
    done

    local avg=$((total / iterations))

    echo "$name|$avg|$min|$max"
}

# Print header
print_header() {
    echo ""
    echo -e "${CYAN}${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}${BOLD}                 Code Agent Startup Benchmark                   ${NC}"
    echo -e "${CYAN}${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "  Iterations: ${BOLD}$ITERATIONS${NC}"
    echo -e "  Platform:   ${BOLD}$(uname -s) $(uname -m)${NC}"
    echo ""
}

# Print results table
print_results() {
    local results=("$@")

    echo -e "${BOLD}Results (lower is better):${NC}"
    echo ""
    printf "  %-20s %10s %10s %10s\n" "Agent" "Avg (ms)" "Min (ms)" "Max (ms)"
    echo "  ────────────────────────────────────────────────────────"

    local sage_avg=0

    for result in "${results[@]}"; do
        IFS='|' read -r name avg min max <<< "$result"

        if [ "$name" = "sage" ]; then
            sage_avg=$avg
            printf "  ${GREEN}%-20s %10s %10s %10s${NC}\n" "$name" "$avg" "$min" "$max"
        else
            printf "  %-20s %10s %10s %10s\n" "$name" "$avg" "$min" "$max"
        fi
    done

    echo ""

    # Print comparison
    if [ $sage_avg -gt 0 ]; then
        echo -e "${BOLD}Comparison:${NC}"
        echo ""
        for result in "${results[@]}"; do
            IFS='|' read -r name avg min max <<< "$result"
            if [ "$name" != "sage" ] && [ $avg -gt 0 ]; then
                local ratio=$(echo "scale=1; $avg / $sage_avg" | bc 2>/dev/null || echo "?")
                echo -e "  Sage is ${GREEN}${ratio}x faster${NC} than $name"
            fi
        done
        echo ""
    fi
}

# Print ASCII bar chart
print_chart() {
    local results=("$@")
    local max_avg=0

    # Find max for scaling
    for result in "${results[@]}"; do
        IFS='|' read -r name avg min max <<< "$result"
        if [ $avg -gt $max_avg ]; then max_avg=$avg; fi
    done

    echo -e "${BOLD}Visual Comparison:${NC}"
    echo ""

    for result in "${results[@]}"; do
        IFS='|' read -r name avg min max <<< "$result"

        # Calculate bar length (max 40 chars)
        local bar_len=$((avg * 40 / max_avg))
        local bar=""
        for ((i=0; i<bar_len; i++)); do
            bar="${bar}█"
        done

        if [ "$name" = "sage" ]; then
            printf "  ${GREEN}%-12s ${bar} %dms${NC}\n" "$name" "$avg"
        else
            printf "  %-12s ${BLUE}${bar}${NC} %dms\n" "$name" "$avg"
        fi
    done
    echo ""
}

# Print JSON output
print_json() {
    local results=("$@")

    echo "{"
    echo "  \"benchmark\": \"startup_time\","
    echo "  \"iterations\": $ITERATIONS,"
    echo "  \"platform\": \"$(uname -s) $(uname -m)\","
    echo "  \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "  \"results\": ["

    local first=true
    for result in "${results[@]}"; do
        IFS='|' read -r name avg min max <<< "$result"
        if [ "$first" = true ]; then
            first=false
        else
            echo ","
        fi
        echo -n "    {\"name\": \"$name\", \"avg_ms\": $avg, \"min_ms\": $min, \"max_ms\": $max}"
    done

    echo ""
    echo "  ]"
    echo "}"
}

# Main
main() {
    local results=()

    if [ "$OUTPUT_FORMAT" = "text" ]; then
        print_header
    fi

    # Warm up
    if [ "$OUTPUT_FORMAT" = "text" ]; then
        echo -e "${BLUE}Warming up...${NC}"
    fi

    # Check which tools are available and benchmark them
    local tools_found=0

    # Sage (required)
    if command -v sage &> /dev/null; then
        if [ "$OUTPUT_FORMAT" = "text" ]; then
            echo -e "  Benchmarking ${GREEN}sage${NC}..."
        fi
        sage --version > /dev/null 2>&1 || true  # Warm up
        result=$(benchmark_command "sage" "sage --version" "$ITERATIONS")
        results+=("$result")
        tools_found=$((tools_found + 1))
    else
        # Try local build
        if [ -f "./target/release/sage" ]; then
            if [ "$OUTPUT_FORMAT" = "text" ]; then
                echo -e "  Benchmarking ${GREEN}sage (local)${NC}..."
            fi
            ./target/release/sage --version > /dev/null 2>&1 || true
            result=$(benchmark_command "sage" "./target/release/sage --version" "$ITERATIONS")
            results+=("$result")
            tools_found=$((tools_found + 1))
        else
            echo -e "${RED}Error: sage not found. Build it first with 'cargo build --release'${NC}"
            exit 1
        fi
    fi

    # Claude Code
    if command -v claude &> /dev/null; then
        if [ "$OUTPUT_FORMAT" = "text" ]; then
            echo -e "  Benchmarking claude..."
        fi
        claude --version > /dev/null 2>&1 || true
        result=$(benchmark_command "claude" "claude --version" "$ITERATIONS")
        results+=("$result")
        tools_found=$((tools_found + 1))
    fi

    # Aider
    if command -v aider &> /dev/null; then
        if [ "$OUTPUT_FORMAT" = "text" ]; then
            echo -e "  Benchmarking aider..."
        fi
        aider --version > /dev/null 2>&1 || true
        result=$(benchmark_command "aider" "aider --version" "$ITERATIONS")
        results+=("$result")
        tools_found=$((tools_found + 1))
    fi

    # Codex CLI
    if command -v codex &> /dev/null; then
        if [ "$OUTPUT_FORMAT" = "text" ]; then
            echo -e "  Benchmarking codex..."
        fi
        codex --version > /dev/null 2>&1 || true
        result=$(benchmark_command "codex" "codex --version" "$ITERATIONS")
        results+=("$result")
        tools_found=$((tools_found + 1))
    fi

    # Continue
    if command -v continue &> /dev/null; then
        if [ "$OUTPUT_FORMAT" = "text" ]; then
            echo -e "  Benchmarking continue..."
        fi
        continue --version > /dev/null 2>&1 || true
        result=$(benchmark_command "continue" "continue --version" "$ITERATIONS")
        results+=("$result")
        tools_found=$((tools_found + 1))
    fi

    # Node.js (reference)
    if command -v node &> /dev/null; then
        if [ "$OUTPUT_FORMAT" = "text" ]; then
            echo -e "  Benchmarking node (reference)..."
        fi
        node --version > /dev/null 2>&1 || true
        result=$(benchmark_command "node" "node --version" "$ITERATIONS")
        results+=("$result")
    fi

    # Python (reference)
    if command -v python3 &> /dev/null; then
        if [ "$OUTPUT_FORMAT" = "text" ]; then
            echo -e "  Benchmarking python3 (reference)..."
        fi
        python3 --version > /dev/null 2>&1 || true
        result=$(benchmark_command "python3" "python3 --version" "$ITERATIONS")
        results+=("$result")
    fi

    echo ""

    # Output results
    if [ "$OUTPUT_FORMAT" = "json" ]; then
        print_json "${results[@]}"
    else
        print_results "${results[@]}"
        print_chart "${results[@]}"

        if [ $tools_found -lt 2 ]; then
            echo -e "${YELLOW}Note: Install more code agents (claude, aider, codex) for comparison.${NC}"
            echo ""
        fi
    fi
}

main
