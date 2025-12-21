#!/usr/bin/env python3
"""
Local SWE-bench Evaluation (Simplified)

This script runs a simplified local evaluation without Docker.
It applies patches and runs the test_patch to verify correctness.

Note: This is a simplified version. For official benchmarking,
use the Docker-based evaluation.
"""

import os
import sys
import json
import subprocess
import tempfile
import shutil
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Optional, Any
import urllib.request


class LocalEvaluator:
    """Simplified local evaluator for SWE-bench."""

    def __init__(self, work_dir: str = None):
        self.work_dir = Path(work_dir) if work_dir else Path.cwd() / "eval_runs"
        self.work_dir.mkdir(parents=True, exist_ok=True)

    def fetch_instance_data(self, instance_id: str) -> Optional[Dict[str, Any]]:
        """Fetch instance data from HuggingFace API."""
        # Search through the dataset API
        base_url = "https://datasets-server.huggingface.co/rows"
        params = "dataset=princeton-nlp%2FSWE-bench_Lite&config=default&split=test"

        # We need to search through pages
        for offset in range(0, 300, 100):
            url = f"{base_url}?{params}&offset={offset}&length=100"
            try:
                with urllib.request.urlopen(url, timeout=30) as response:
                    data = json.loads(response.read().decode())
                    for row in data.get("rows", []):
                        features = row.get("row", {})
                        if features.get("instance_id") == instance_id:
                            return features
            except Exception as e:
                print(f"  Warning: Failed to fetch page {offset}: {e}")
                continue

        return None

    def setup_instance(self, instance_id: str, instance_data: Dict) -> Path:
        """Set up test instance."""
        instance_dir = self.work_dir / instance_id

        if instance_dir.exists():
            shutil.rmtree(instance_dir)

        instance_dir.mkdir(parents=True)

        repo = instance_data["repo"]
        base_commit = instance_data["base_commit"]

        print(f"  Cloning {repo}...")
        subprocess.run(
            ["git", "clone", "--depth", "1", f"https://github.com/{repo}.git", str(instance_dir)],
            capture_output=True,
            check=True,
        )

        print(f"  Fetching commit {base_commit[:8]}...")
        subprocess.run(
            ["git", "-C", str(instance_dir), "fetch", "--depth", "100", "origin", base_commit],
            capture_output=True,
            check=True,
        )

        subprocess.run(
            ["git", "-C", str(instance_dir), "checkout", base_commit],
            capture_output=True,
            check=True,
        )

        return instance_dir

    def apply_patch(self, instance_dir: Path, patch: str) -> bool:
        """Apply a patch to the instance."""
        if not patch.strip():
            return False

        patch_file = instance_dir / "model.patch"
        patch_file.write_text(patch)

        try:
            result = subprocess.run(
                ["git", "apply", "--check", "model.patch"],
                cwd=str(instance_dir),
                capture_output=True,
                text=True,
            )

            if result.returncode != 0:
                print(f"  Patch check failed: {result.stderr}")
                return False

            subprocess.run(
                ["git", "apply", "model.patch"],
                cwd=str(instance_dir),
                capture_output=True,
                check=True,
            )

            return True
        except Exception as e:
            print(f"  Failed to apply patch: {e}")
            return False

    def apply_test_patch(self, instance_dir: Path, test_patch: str) -> bool:
        """Apply the test patch."""
        if not test_patch.strip():
            print("  No test patch available")
            return True

        patch_file = instance_dir / "test.patch"
        patch_file.write_text(test_patch)

        try:
            subprocess.run(
                ["git", "apply", "test.patch"],
                cwd=str(instance_dir),
                capture_output=True,
                check=True,
            )
            return True
        except Exception as e:
            print(f"  Failed to apply test patch: {e}")
            return False

    def run_tests(self, instance_dir: Path, instance_data: Dict) -> Dict[str, Any]:
        """Run tests for the instance."""
        repo = instance_data["repo"]
        result = {"passed": False, "output": "", "error": ""}

        # Determine test command based on repo
        if "django" in repo:
            test_cmd = self._get_django_test_cmd(instance_dir, instance_data)
        elif "sympy" in repo:
            test_cmd = self._get_sympy_test_cmd(instance_dir, instance_data)
        elif "requests" in repo:
            test_cmd = ["python", "-m", "pytest", "-xvs"]
        elif "flask" in repo:
            test_cmd = ["python", "-m", "pytest", "-xvs"]
        else:
            test_cmd = ["python", "-m", "pytest", "-xvs"]

        print(f"  Running: {' '.join(test_cmd[:3])}...")

        try:
            proc = subprocess.run(
                test_cmd,
                cwd=str(instance_dir),
                capture_output=True,
                text=True,
                timeout=300,
                env={**os.environ, "PYTHONPATH": str(instance_dir)},
            )

            result["output"] = proc.stdout + proc.stderr
            result["passed"] = proc.returncode == 0

            if result["passed"]:
                print("  ✅ Tests PASSED")
            else:
                print("  ❌ Tests FAILED")
                # Show last few lines of output
                lines = result["output"].strip().split("\n")
                for line in lines[-10:]:
                    print(f"    {line}")

        except subprocess.TimeoutExpired:
            result["error"] = "Test timeout"
            print("  ⏰ Tests TIMEOUT")
        except Exception as e:
            result["error"] = str(e)
            print(f"  ❌ Test error: {e}")

        return result

    def _get_django_test_cmd(self, instance_dir: Path, instance_data: Dict) -> List[str]:
        """Get Django test command."""
        # Extract test files from test_patch
        test_patch = instance_data.get("test_patch", "")
        test_files = []

        for line in test_patch.split("\n"):
            if line.startswith("diff --git"):
                parts = line.split()
                if len(parts) >= 4:
                    file_path = parts[3].lstrip("b/")
                    if "test" in file_path:
                        # Convert path to test module
                        module = file_path.replace("/", ".").replace(".py", "")
                        test_files.append(module)

        if test_files:
            return ["python", "tests/runtests.py", "--verbosity=2"] + test_files[:3]
        else:
            return ["python", "tests/runtests.py", "--verbosity=2"]

    def _get_sympy_test_cmd(self, instance_dir: Path, instance_data: Dict) -> List[str]:
        """Get SymPy test command."""
        return ["python", "-m", "pytest", "-xvs", "sympy/"]

    def evaluate_prediction(self, prediction: Dict[str, Any]) -> Dict[str, Any]:
        """Evaluate a single prediction."""
        instance_id = prediction["instance_id"]
        model_patch = prediction.get("model_patch", "")

        print(f"\n{'='*60}")
        print(f"Evaluating: {instance_id}")
        print(f"{'='*60}")

        result = {
            "instance_id": instance_id,
            "resolved": False,
            "patch_applied": False,
            "test_output": "",
            "error": "",
        }

        # Fetch instance data
        print("  Fetching instance data...")
        instance_data = self.fetch_instance_data(instance_id)

        if not instance_data:
            result["error"] = "Could not fetch instance data"
            print(f"  ❌ {result['error']}")
            return result

        try:
            # Setup instance
            instance_dir = self.setup_instance(instance_id, instance_data)

            # Apply model patch
            print("  Applying model patch...")
            if not self.apply_patch(instance_dir, model_patch):
                result["error"] = "Failed to apply model patch"
                return result
            result["patch_applied"] = True

            # Apply test patch
            print("  Applying test patch...")
            test_patch = instance_data.get("test_patch", "")
            if not self.apply_test_patch(instance_dir, test_patch):
                result["error"] = "Failed to apply test patch"
                return result

            # Run tests
            test_result = self.run_tests(instance_dir, instance_data)
            result["resolved"] = test_result["passed"]
            result["test_output"] = test_result["output"]
            if test_result["error"]:
                result["error"] = test_result["error"]

        except Exception as e:
            result["error"] = str(e)
            print(f"  ❌ Error: {e}")

        return result

    def evaluate_all(self, predictions: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Evaluate all predictions."""
        results = []

        start_time = datetime.now()

        for i, prediction in enumerate(predictions, 1):
            print(f"\n[{i}/{len(predictions)}]")
            result = self.evaluate_prediction(prediction)
            results.append(result)

        elapsed = datetime.now() - start_time

        # Print summary
        print(f"\n{'='*60}")
        print("EVALUATION SUMMARY")
        print(f"{'='*60}")

        resolved = sum(1 for r in results if r["resolved"])
        patch_applied = sum(1 for r in results if r["patch_applied"])

        print(f"Total instances: {len(results)}")
        print(f"Patches applied: {patch_applied}")
        print(f"Tests passed: {resolved}")
        print(f"Pass rate: {resolved/len(results)*100:.1f}%")
        print(f"Time elapsed: {elapsed}")

        print("\nPer-instance results:")
        for r in results:
            status = "✅" if r["resolved"] else "❌"
            print(f"  {status} {r['instance_id']}")
            if r["error"]:
                print(f"      Error: {r['error']}")

        return results


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Local SWE-bench evaluation")
    parser.add_argument(
        "predictions",
        help="Path to predictions.json",
    )
    parser.add_argument(
        "--instances",
        nargs="+",
        help="Specific instance IDs to evaluate",
    )
    parser.add_argument(
        "--output",
        default="results.json",
        help="Output results file",
    )
    parser.add_argument(
        "--work-dir",
        help="Working directory",
    )

    args = parser.parse_args()

    # Load predictions
    with open(args.predictions) as f:
        predictions = json.load(f)

    # Filter by instance IDs
    if args.instances:
        predictions = [p for p in predictions if p["instance_id"] in args.instances]

    if not predictions:
        print("No predictions to evaluate!")
        sys.exit(1)

    print(f"Evaluating {len(predictions)} predictions...")

    # Run evaluation
    evaluator = LocalEvaluator(work_dir=args.work_dir)
    results = evaluator.evaluate_all(predictions)

    # Save results
    output_path = Path(args.output)
    with open(output_path, "w") as f:
        json.dump(results, f, indent=2)

    print(f"\nResults saved to: {output_path}")


if __name__ == "__main__":
    main()
