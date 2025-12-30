#!/bin/zsh
cd /Users/lifcc/Desktop/code/AI/agent/sage/swebench_eval

LOG_FILE="auto_retry_$(date +%Y%m%d_%H%M%S).log"

log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" | tee -a "$LOG_FILE"
}

log "=== Starting auto retry for 100% patch rate ==="

# Function to get instances without valid patches
get_missing() {
    python3 << 'EOF'
import json
from pathlib import Path

all_preds = {}

# Load all existing predictions
for f in Path(".").glob("predictions*.json"):
    if f.name.startswith("predictions"):
        try:
            with open(f) as fp:
                for item in json.load(fp):
                    iid = item["instance_id"]
                    patch = item.get("model_patch", "")
                    if iid not in all_preds or (patch and not all_preds[iid].get("model_patch")):
                        all_preds[iid] = item
        except: pass

for f in Path("swebench_runs").glob("predictions*.json"):
    try:
        with open(f) as fp:
            for item in json.load(fp):
                iid = item["instance_id"]
                patch = item.get("model_patch", "")
                if iid not in all_preds or (patch and not all_preds[iid].get("model_patch")):
                    all_preds[iid] = item
    except: pass

# Find missing
missing = [iid for iid, p in all_preds.items() if not p.get("model_patch")]
print("\n".join(missing))
EOF
}

ROUND=1
while true; do
    log "Round $ROUND: Checking for missing patches..."
    
    MISSING=$(get_missing)
    COUNT=$(echo "$MISSING" | grep -c . || echo 0)
    
    if [ "$COUNT" -eq 0 ] || [ -z "$MISSING" ]; then
        log "✅ All patches complete! 100% coverage achieved!"
        break
    fi
    
    log "Found $COUNT instances without patches. Starting evaluation..."
    
    # Save missing to file
    echo "$MISSING" > current_retry.txt
    
    # Run evaluation
    INSTANCES=("${(@f)$(cat current_retry.txt)}")
    
    uv run run_agent.py \
        --instances "${INSTANCES[@]}" \
        --output "predictions_retry_r${ROUND}.json" \
        --timeout 900 \
        --max-retries 2 \
        >> "$LOG_FILE" 2>&1
    
    log "Round $ROUND complete. Checking results..."
    
    ROUND=$((ROUND + 1))
    
    # Safety limit
    if [ $ROUND -gt 10 ]; then
        log "⚠️ Reached 10 rounds limit. Stopping."
        break
    fi
    
    sleep 10
done

# Final merge
log "Merging all predictions..."
python3 << 'EOF'
import json
from pathlib import Path

all_preds = {}
for f in list(Path(".").glob("predictions*.json")) + list(Path("swebench_runs").glob("predictions*.json")):
    try:
        with open(f) as fp:
            for item in json.load(fp):
                iid = item["instance_id"]
                patch = item.get("model_patch", "")
                if iid not in all_preds or (patch and not all_preds[iid].get("model_patch")):
                    all_preds[iid] = item
    except: pass

result = list(all_preds.values())
with open("predictions_final_100.json", "w") as f:
    json.dump(result, f, indent=2)

valid = sum(1 for p in result if p.get("model_patch"))
print(f"Final: {len(result)} instances, {valid} valid ({100*valid//len(result)}%)")
EOF

log "=== Auto retry complete ==="
