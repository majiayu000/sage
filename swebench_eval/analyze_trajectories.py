#!/usr/bin/env python3
"""Analyze trajectory files for errors and unreasonable patterns."""

import json
import os
from pathlib import Path
from collections import defaultdict

def analyze_trajectory(filepath):
    """Analyze a single trajectory file."""
    try:
        with open(filepath) as f:
            data = json.load(f)
    except Exception as e:
        return {"parse_error": str(e)}

    errors = []
    unreasonable = []
    tool_usage = defaultdict(int)

    agent_steps = data.get("agent_steps", [])

    for step in agent_steps:
        step_num = step.get("step_number", "?")

        # Analyze tool calls
        tool_calls = step.get("tool_calls") or []
        for tc in tool_calls:
            if not tc:
                continue
            tool_name = tc.get("name", "unknown")
            tool_usage[tool_name] += 1
            args = tc.get("arguments") or {}

            # Check for unreasonable patterns
            if tool_name == "Write":
                file_path = str(args.get("file_path", ""))
                if "/tmp" in file_path:
                    unreasonable.append({
                        "step": step_num,
                        "issue": f"Write to /tmp: {file_path[:50]}",
                        "severity": "high"
                    })

            if tool_name == "bash":
                cmd = str(args.get("command", ""))
                if "cat " in cmd and ".py" in cmd:
                    unreasonable.append({
                        "step": step_num,
                        "issue": "Using 'cat' instead of Read tool",
                        "severity": "low"
                    })
                if "echo " in cmd and ">" in cmd:
                    unreasonable.append({
                        "step": step_num,
                        "issue": "Using echo redirect instead of Write tool",
                        "severity": "medium"
                    })

        # Analyze tool results for errors
        tool_results = step.get("tool_results") or []
        for tr in tool_results:
            if not tr:
                continue
            if tr.get("error") and tr["error"] != "null" and tr["error"] is not None:
                error_msg = str(tr["error"])
                if "Binary file" not in error_msg:  # Skip binary file warnings
                    errors.append({
                        "step": step_num,
                        "error": error_msg[:200]
                    })

    return {
        "total_steps": len(agent_steps),
        "errors": errors,
        "unreasonable": unreasonable,
        "tool_usage": dict(tool_usage),
        "task": data.get("task", "")[:100] if data.get("task") else ""
    }

def main():
    base_dir = Path("/Users/lifcc/Desktop/code/AI/agent/sage/swebench_eval/swebench_runs")

    all_results = {}
    error_summary = defaultdict(int)
    unreasonable_summary = defaultdict(int)

    trajectory_files = list(base_dir.glob("*/trajectories/*.json"))
    print(f"Found {len(trajectory_files)} trajectory files\n")

    for traj_file in sorted(trajectory_files):
        instance_id = traj_file.parent.parent.name
        result = analyze_trajectory(traj_file)
        all_results[instance_id] = result

        if result.get("parse_error"):
            print(f"[PARSE ERROR] {instance_id}: {result['parse_error']}")
            continue

        # Summarize
        for err in result.get("errors", []):
            err_type = err["error"].split(":")[0][:50]
            error_summary[err_type] += 1

        for issue in result.get("unreasonable", []):
            unreasonable_summary[issue["issue"].split(":")[0]] += 1

        # Report instances with issues
        if result["errors"] or result["unreasonable"]:
            print(f"=== {instance_id} ===")
            print(f"  Steps: {result['total_steps']}")
            if result["errors"]:
                print(f"  Errors ({len(result['errors'])}):")
                for e in result["errors"][:3]:
                    print(f"    Step {e['step']}: {e['error'][:80]}")
            if result["unreasonable"]:
                print(f"  Unreasonable ({len(result['unreasonable'])}):")
                for u in result["unreasonable"][:3]:
                    print(f"    Step {u['step']}: {u['issue']}")
            print()

    # Print summary
    print("\n" + "="*60)
    print("SUMMARY")
    print("="*60)
    print(f"\nTotal instances analyzed: {len(all_results)}")

    # Error type summary
    if error_summary:
        print("\nError Types:")
        for err_type, count in sorted(error_summary.items(), key=lambda x: -x[1])[:10]:
            print(f"  {count:3d}x {err_type}")

    # Unreasonable pattern summary
    if unreasonable_summary:
        print("\nUnreasonable Patterns:")
        for issue, count in sorted(unreasonable_summary.items(), key=lambda x: -x[1]):
            print(f"  {count:3d}x {issue}")

    # Tool usage across all
    total_tool_usage = defaultdict(int)
    for result in all_results.values():
        if isinstance(result, dict) and "tool_usage" in result:
            for tool, count in result["tool_usage"].items():
                total_tool_usage[tool] += count

    print("\nTool Usage (top 15):")
    for tool, count in sorted(total_tool_usage.items(), key=lambda x: -x[1])[:15]:
        print(f"  {count:5d}x {tool}")

if __name__ == "__main__":
    main()
