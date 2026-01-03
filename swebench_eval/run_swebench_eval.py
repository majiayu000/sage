#!/usr/bin/env python3
"""Run SWE-bench evaluation on predictions."""

import argparse
from swebench import run_evaluation

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--predictions", required=True, help="Path to predictions JSON")
    parser.add_argument("--max-workers", type=int, default=4, help="Number of parallel workers")
    parser.add_argument("--run-id", default="sage-eval", help="Run ID for this evaluation")
    parser.add_argument("--timeout", type=int, default=1800, help="Timeout per instance")
    args = parser.parse_args()
    
    print(f"Running SWE-bench evaluation on {args.predictions}")
    print(f"Max workers: {args.max_workers}")
    print(f"Timeout: {args.timeout}s")
    
    run_evaluation(
        dataset_name="princeton-nlp/SWE-bench_Lite",
        split="test",
        instance_ids=None,  # Evaluate all instances in predictions
        predictions_path=args.predictions,
        max_workers=args.max_workers,
        force_rebuild=False,
        cache_level="env",
        clean=False,
        open_file_limit=4096,
        run_id=args.run_id,
        timeout=args.timeout,
        namespace=None,
        rewrite_reports=False,
        modal=False,
    )

if __name__ == "__main__":
    main()
