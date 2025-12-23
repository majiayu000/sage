#!/usr/bin/env python3
"""
SWE-bench Evaluation Runner for Sage Agent

This script:
1. Loads SWE-bench instances
2. Runs Sage agent on each instance
3. Extracts patches from git diff
4. Generates predictions.json for official evaluation
"""

import os
import sys
import json
import subprocess
import tempfile
import shutil
from pathlib import Path
from datetime import datetime
from typing import Optional, List, Dict, Any

# Try to import datasets, provide helpful error if not available
try:
    from datasets import load_dataset
except ImportError:
    print("Error: 'datasets' package not installed. Run: pip install datasets")
    sys.exit(1)


# SWE-bench specific prompt to ensure agent modifies source code
SWEBENCH_PROMPT_PREFIX = """
## CRITICAL INSTRUCTIONS FOR BUG FIX TASK

You are tasked with FIXING A BUG in the codebase. This is NOT an analysis task - you MUST implement the fix.

### REQUIREMENTS:
1. You MUST modify existing source code files to fix the bug
2. You MUST use the Edit tool to make changes (NOT Write tool for new files)
3. Your changes will be evaluated via `git diff` - ONLY modifications to tracked files count
4. Do NOT just analyze or explain the problem - you must IMPLEMENT the fix
5. Do NOT create workaround scripts, example files, or patches in /tmp
6. Do NOT suggest solutions - APPLY them directly to the source code

### WHAT COUNTS AS SUCCESS:
- Modifying the actual source code files (e.g., django/db/models/xxx.py)
- Changes that appear in `git diff` output
- Code changes that fix the described bug

### WHAT DOES NOT COUNT:
- Creating new files (unless the bug fix requires a new file in the source tree)
- Writing to /tmp or other temporary locations
- Just explaining what needs to be changed
- Suggesting the user make the change themselves

### WORKFLOW:
1. Read the problem statement carefully
2. Search the codebase to find the relevant source files
3. Understand the bug and identify the fix
4. Use the Edit tool to modify the source code
5. Verify your changes with `git diff`

---

## Bug Report:

"""

SWEBENCH_RETRY_PROMPT = """

## IMPORTANT: Previous Attempt Failed

Your previous attempt did not produce any changes to the source code (git diff is empty).

You MUST:
1. Use the Edit tool to modify existing source files
2. Make actual code changes, not just analysis
3. Do NOT create new files - modify existing source code

Please try again and make the necessary code changes to fix the bug.
"""


class SageEvaluator:
    """Runs Sage agent on SWE-bench instances and collects predictions."""

    def __init__(
        self,
        sage_binary: str = None,
        work_dir: str = None,
        max_steps: int = 25,
        timeout: int = 600,
        max_retries: int = 1,
    ):
        self.sage_binary = sage_binary or self._find_sage_binary()
        if work_dir:
            self.work_dir = Path(work_dir)
        else:
            env_dir = os.environ.get("SAGE_SWEBENCH_RUN_DIR")
            self.work_dir = Path(env_dir) if env_dir else Path.cwd() / "swebench_runs"
        self.max_steps = max_steps
        self.timeout = timeout
        self.max_retries = max_retries  # Number of retry attempts if no patch generated
        self.work_dir.mkdir(parents=True, exist_ok=True)

    def _find_sage_binary(self) -> str:
        """Find the Sage binary."""
        # Check common locations
        locations = [
            Path(__file__).parent.parent / "target" / "release" / "sage",
            Path.home() / ".cargo" / "bin" / "sage",
            shutil.which("sage"),
        ]
        for loc in locations:
            if loc and Path(loc).exists():
                return str(loc)
        raise FileNotFoundError("Could not find sage binary. Please specify path.")

    def load_instances(
        self,
        dataset_name: str = "princeton-nlp/SWE-bench_Lite",
        split: str = "test",
        instance_ids: List[str] = None,
        repo_filter: str = None,
        limit: int = None,
    ) -> List[Dict[str, Any]]:
        """Load SWE-bench instances from HuggingFace."""
        print(f"Loading dataset: {dataset_name} ({split})...")
        dataset = load_dataset(dataset_name, split=split)

        instances = list(dataset)

        # Filter by instance IDs
        if instance_ids:
            instances = [i for i in instances if i["instance_id"] in instance_ids]

        # Filter by repo
        if repo_filter:
            instances = [i for i in instances if repo_filter in i["repo"]]

        # Limit number of instances
        if limit:
            instances = instances[:limit]

        print(f"Loaded {len(instances)} instances")
        return instances

    def setup_instance(self, instance: Dict[str, Any]) -> Path:
        """Set up a test instance (clone repo, checkout commit)."""
        instance_id = instance["instance_id"]
        repo = instance["repo"]
        base_commit = instance["base_commit"]

        instance_dir = self.work_dir / instance_id

        # Clean up if exists
        if instance_dir.exists():
            shutil.rmtree(instance_dir)

        instance_dir.mkdir(parents=True)

        print(f"  Setting up {instance_id}...")

        # Clone repo
        repo_url = f"https://github.com/{repo}.git"
        subprocess.run(
            ["git", "clone", "--depth", "1", repo_url, str(instance_dir)],
            capture_output=True,
            check=True,
        )

        # Fetch the specific commit
        subprocess.run(
            ["git", "-C", str(instance_dir), "fetch", "--depth", "100", "origin", base_commit],
            capture_output=True,
            check=True,
        )

        # Checkout base commit
        subprocess.run(
            ["git", "-C", str(instance_dir), "checkout", base_commit],
            capture_output=True,
            check=True,
        )

        # Write problem statement
        problem_file = instance_dir / "PROBLEM_STATEMENT.md"
        problem_file.write_text(f"# {instance_id}\n\n{instance['problem_statement']}")

        # Copy sage config if exists
        config_src = Path(__file__).parent.parent / "sage_config.json"
        if config_src.exists():
            shutil.copy(config_src, instance_dir / "sage_config.json")

        return instance_dir

    def run_agent(self, instance_dir: Path, problem_statement: str, instance_id: str, is_retry: bool = False) -> bool:
        """Run Sage agent on the instance.

        Args:
            instance_dir: Directory containing the instance
            problem_statement: The original problem statement
            instance_id: Instance ID for trajectory file naming
            is_retry: Whether this is a retry attempt (adds retry prompt)
        """
        print(f"  Running agent{'  (retry)' if is_retry else ''}...")

        # Create trajectory file path for this instance
        trajectory_file = instance_dir / f"trajectory_{instance_id}.json"

        # Build the full prompt with SWE-bench specific instructions
        full_prompt = SWEBENCH_PROMPT_PREFIX + problem_statement
        if is_retry:
            full_prompt += SWEBENCH_RETRY_PROMPT

        try:
            # Build command - note: max_steps is now unlimited by default
            cmd = [
                self.sage_binary,
                "unified",
                full_prompt,
                "--trajectory-file", str(trajectory_file),
                "--non-interactive",  # Auto-respond to questions
            ]

            # Only add max-steps if explicitly set
            if self.max_steps and self.max_steps > 0:
                cmd.extend(["--max-steps", str(self.max_steps)])

            result = subprocess.run(
                cmd,
                cwd=str(instance_dir),
                capture_output=True,
                text=True,
                timeout=self.timeout,
            )

            # Check if trajectory was created
            if trajectory_file.exists():
                print(f"  📝 Trajectory saved: {trajectory_file.name}")
            else:
                print(f"  ⚠️ No trajectory file created")

            return result.returncode == 0
        except subprocess.TimeoutExpired:
            print(f"  Agent timed out after {self.timeout}s")
            return False
        except Exception as e:
            print(f"  Agent error: {e}")
            return False

    def extract_patch(self, instance_dir: Path) -> Optional[str]:
        """Extract git diff as patch."""
        try:
            result = subprocess.run(
                ["git", "diff"],
                cwd=str(instance_dir),
                capture_output=True,
                text=True,
                check=True,
            )
            patch = result.stdout.strip()
            return patch if patch else None
        except Exception as e:
            print(f"  Failed to extract patch: {e}")
            return None

    def evaluate_instance(self, instance: Dict[str, Any]) -> Dict[str, Any]:
        """Evaluate a single instance with retry logic."""
        instance_id = instance["instance_id"]
        print(f"\n{'='*60}")
        print(f"Evaluating: {instance_id}")
        print(f"{'='*60}")

        result = {
            "instance_id": instance_id,
            "model_patch": "",
            "model_name_or_path": "sage-agent",
        }

        try:
            # Setup instance
            instance_dir = self.setup_instance(instance)

            # Run agent with retry logic
            patch = None
            for attempt in range(self.max_retries + 1):
                is_retry = attempt > 0

                if is_retry:
                    print(f"\n  🔄 Retry attempt {attempt}/{self.max_retries}")
                    # Reset git state before retry
                    subprocess.run(
                        ["git", "checkout", "."],
                        cwd=str(instance_dir),
                        capture_output=True,
                    )

                # Run agent
                success = self.run_agent(
                    instance_dir,
                    instance["problem_statement"],
                    instance_id,
                    is_retry=is_retry
                )

                # Extract patch
                patch = self.extract_patch(instance_dir)

                if patch:
                    result["model_patch"] = patch
                    print(f"  ✅ Generated patch ({len(patch)} chars)")
                    break  # Success, no need to retry
                else:
                    if attempt < self.max_retries:
                        print(f"  ⚠️ No patch generated, will retry...")
                    else:
                        print(f"  ❌ No patch generated after {self.max_retries + 1} attempts")

        except Exception as e:
            print(f"  ❌ Error: {e}")

        return result

    def run_evaluation(
        self,
        instances: List[Dict[str, Any]],
        output_file: str = "predictions.json",
    ) -> List[Dict[str, Any]]:
        """Run evaluation on all instances."""
        predictions = []

        start_time = datetime.now()

        for i, instance in enumerate(instances, 1):
            print(f"\n[{i}/{len(instances)}]")
            prediction = self.evaluate_instance(instance)
            predictions.append(prediction)

            # Save intermediate results
            self._save_predictions(predictions, output_file)

        elapsed = datetime.now() - start_time

        # Print summary
        print(f"\n{'='*60}")
        print("EVALUATION SUMMARY")
        print(f"{'='*60}")
        print(f"Total instances: {len(instances)}")
        print(f"Patches generated: {sum(1 for p in predictions if p['model_patch'])}")
        print(f"Time elapsed: {elapsed}")
        print(f"Output saved to: {output_file}")

        return predictions

    def _save_predictions(self, predictions: List[Dict[str, Any]], output_file: str):
        """Save predictions to JSON file."""
        output_path = self.work_dir / output_file
        with open(output_path, "w") as f:
            json.dump(predictions, f, indent=2)


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Run Sage agent on SWE-bench instances")
    parser.add_argument(
        "--dataset",
        default="princeton-nlp/SWE-bench_Lite",
        help="Dataset name (default: princeton-nlp/SWE-bench_Lite)",
    )
    parser.add_argument(
        "--split",
        default="test",
        help="Dataset split (default: test)",
    )
    parser.add_argument(
        "--instances",
        nargs="+",
        help="Specific instance IDs to evaluate",
    )
    parser.add_argument(
        "--repo",
        help="Filter by repository (e.g., 'django/django')",
    )
    parser.add_argument(
        "--limit",
        type=int,
        help="Maximum number of instances to evaluate",
    )
    parser.add_argument(
        "--max-steps",
        type=int,
        default=25,
        help="Maximum steps for agent (default: 25)",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=600,
        help="Timeout per instance in seconds (default: 600)",
    )
    parser.add_argument(
        "--max-retries",
        type=int,
        default=1,
        help="Max retry attempts if no patch generated (default: 1)",
    )
    parser.add_argument(
        "--output",
        default="predictions.json",
        help="Output file name (default: predictions.json)",
    )
    parser.add_argument(
        "--work-dir",
        help="Working directory for test instances",
    )
    parser.add_argument(
        "--sage-binary",
        help="Path to sage binary",
    )

    args = parser.parse_args()

    # Initialize evaluator
    evaluator = SageEvaluator(
        sage_binary=args.sage_binary,
        work_dir=args.work_dir,
        max_steps=args.max_steps,
        timeout=args.timeout,
        max_retries=args.max_retries,
    )

    # Load instances
    instances = evaluator.load_instances(
        dataset_name=args.dataset,
        split=args.split,
        instance_ids=args.instances,
        repo_filter=args.repo,
        limit=args.limit,
    )

    if not instances:
        print("No instances to evaluate!")
        sys.exit(1)

    # Run evaluation
    evaluator.run_evaluation(instances, args.output)


if __name__ == "__main__":
    main()
