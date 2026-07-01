#!/usr/bin/env python3
"""Fail when generated files, tracked changes, or untracked files dirty a repo."""

from __future__ import annotations

import argparse
import subprocess
import sys
import tempfile
from pathlib import Path


def git_status(repo: Path) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), "status", "--porcelain", "--untracked-files=all"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or "git status failed")
    return result.stdout


def check_clean(repo: Path) -> None:
    status = git_status(repo)
    if status.strip():
        details = "\n".join(f"  {line}" for line in status.rstrip().splitlines())
        raise RuntimeError(f"worktree is dirty:\n{details}")


def command_check(args: argparse.Namespace) -> None:
    check_clean(Path(args.repo).resolve())
    print("worktree clean")


def command_self_test(_args: argparse.Namespace) -> None:
    with tempfile.TemporaryDirectory(prefix="sage-clean-worktree-test-") as temp:
        repo = Path(temp)
        subprocess.run(["git", "init"], cwd=repo, stdout=subprocess.DEVNULL, check=True)
        check_clean(repo)
        (repo / "generated.txt").write_text("dirty\n", encoding="utf-8")
        try:
            check_clean(repo)
        except RuntimeError as error:
            if "generated.txt" not in str(error):
                raise
        else:
            raise RuntimeError("dirty-file fixture did not fail")
    print("clean worktree self-test passed")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=".", help="Repository root to check")
    parser.add_argument("--self-test", action="store_true", help="Run the dirty-file fixture")
    args = parser.parse_args()

    try:
        if args.self_test:
            command_self_test(args)
        else:
            command_check(args)
        return 0
    except RuntimeError as error:
        print(f"clean worktree check failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
