#!/usr/bin/env python3
"""
Pre-clone repositories to cache for faster SWE-bench evaluation.

Usage:
    python warmup_cache.py              # Clone all SWE-bench Lite repos
    python warmup_cache.py --repos django/django sympy/sympy  # Clone specific repos
"""

import argparse
import subprocess
from pathlib import Path
from datasets import load_dataset


def get_swebench_repos():
    """Get unique repositories from SWE-bench Lite dataset."""
    ds = load_dataset("princeton-nlp/SWE-bench_Lite", split="test")
    repos = set(item["repo"] for item in ds)
    return sorted(repos)


def warmup_cache(repos: list[str], cache_dir: Path):
    """Clone repositories to cache."""
    cache_dir.mkdir(parents=True, exist_ok=True)

    print(f"Cache directory: {cache_dir}")
    print(f"Repositories to clone: {len(repos)}")
    print()

    for i, repo in enumerate(repos, 1):
        repo_safe_name = repo.replace("/", "__")
        cached_repo_dir = cache_dir / repo_safe_name

        if cached_repo_dir.exists():
            print(f"[{i}/{len(repos)}] {repo} - Already cached ✓")
            continue

        print(f"[{i}/{len(repos)}] {repo} - Cloning...")
        repo_url = f"https://github.com/{repo}.git"

        try:
            result = subprocess.run(
                ["git", "clone", repo_url, str(cached_repo_dir)],
                capture_output=True,
                text=True,
                timeout=600,  # 10 minute timeout
            )
            if result.returncode == 0:
                print(f"    ✓ Cloned successfully")
            else:
                print(f"    ✗ Failed: {result.stderr[:200]}")
        except subprocess.TimeoutExpired:
            print(f"    ✗ Timeout after 10 minutes")
        except Exception as e:
            print(f"    ✗ Error: {e}")

    print()
    print("Cache warmup complete!")

    # Show cache stats
    cached = list(cache_dir.iterdir())
    total_size = sum(
        sum(f.stat().st_size for f in repo.rglob("*") if f.is_file())
        for repo in cached
        if repo.is_dir()
    )
    print(f"Cached repositories: {len(cached)}")
    print(f"Total cache size: {total_size / 1024 / 1024 / 1024:.2f} GB")


def main():
    parser = argparse.ArgumentParser(description="Pre-clone repositories for SWE-bench")
    parser.add_argument(
        "--repos",
        nargs="+",
        help="Specific repositories to clone (e.g., django/django sympy/sympy)",
    )
    parser.add_argument(
        "--cache-dir",
        type=Path,
        default=Path(__file__).parent / "swebench_runs" / ".repo_cache",
        help="Cache directory (default: swebench_runs/.repo_cache)",
    )

    args = parser.parse_args()

    if args.repos:
        repos = args.repos
    else:
        print("Loading SWE-bench Lite dataset to get repository list...")
        repos = get_swebench_repos()
        print(f"Found {len(repos)} unique repositories:")
        for repo in repos:
            print(f"  - {repo}")
        print()

    warmup_cache(repos, args.cache_dir)


if __name__ == "__main__":
    main()
