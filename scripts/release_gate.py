#!/usr/bin/env python3
"""Release, CI, and supply-chain gates for Sage."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shlex
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover
    import tomli as tomllib  # type: ignore


class GateError(RuntimeError):
    """Release gate validation failed."""


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def read_json(path: Path) -> dict:
    return json.loads(read_text(path))


def read_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def release_config(repo: Path) -> dict:
    return read_json(repo / ".github" / "release-support.json")


def fail(message: str) -> None:
    raise GateError(message)


def semver_from_tag(tag: str) -> str:
    if not re.fullmatch(r"v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", tag):
        fail(f"release tag must be vMAJOR.MINOR.PATCH[-PRERELEASE], got {tag!r}")
    return tag[1:]


def workspace_versions(repo: Path) -> tuple[str, list[str]]:
    root = read_toml(repo / "Cargo.toml")
    workspace_version = root.get("workspace", {}).get("package", {}).get("version")
    if not workspace_version:
        fail("Cargo.toml is missing [workspace.package].version")

    mismatches: list[str] = []
    root_version = root.get("package", {}).get("version")
    if root_version != workspace_version:
        mismatches.append(f"root package version {root_version!r} != workspace {workspace_version!r}")

    members = root.get("workspace", {}).get("members", [])
    for member in members:
        manifest = repo / member / "Cargo.toml"
        package = read_toml(manifest).get("package", {})
        version = package.get("version")
        if isinstance(version, dict) and version.get("workspace") is True:
            continue
        if version != workspace_version:
            mismatches.append(f"{manifest}: package version {version!r} != workspace {workspace_version!r}")

    return workspace_version, mismatches


def check_internal_path_versions(repo: Path, version: str) -> list[str]:
    mismatches: list[str] = []
    for manifest in [repo / "Cargo.toml", *repo.glob("crates/*/Cargo.toml")]:
        data = read_toml(manifest)
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            for name, spec in data.get(section, {}).items():
                if isinstance(spec, dict) and "path" in spec and str(spec["path"]).startswith(("crates/", "../")):
                    dep_version = spec.get("version")
                    if dep_version and dep_version != version:
                        mismatches.append(f"{manifest}: {section}.{name} version {dep_version!r} != {version!r}")
    return mismatches


def command_preflight(args: argparse.Namespace) -> None:
    repo = Path(args.repo).resolve()
    tag_version = semver_from_tag(args.tag)
    workspace_version, mismatches = workspace_versions(repo)
    mismatches.extend(check_internal_path_versions(repo, workspace_version))
    if workspace_version != tag_version:
        mismatches.append(f"tag version {tag_version!r} != workspace version {workspace_version!r}")
    if mismatches:
        fail("; ".join(mismatches))

    changelog = read_text(repo / "CHANGELOG.md")
    pattern = rf"^## \[{re.escape(tag_version)}\](?:\s+-|\])"
    if not re.search(pattern, changelog, re.MULTILINE):
        fail(f"CHANGELOG.md is missing release entry [{tag_version}]")
    print(f"release preflight passed for {args.tag}")


def command_support_matrix(args: argparse.Namespace) -> None:
    config = release_config(Path(args.repo).resolve())
    platforms = {item.get("os"): item for item in config.get("platforms", [])}
    missing = {"linux", "macos", "windows"} - set(platforms)
    if missing:
        fail(f"support matrix missing platform entries: {sorted(missing)}")
    for os_name, item in platforms.items():
        status = item.get("status")
        targets = item.get("targets", [])
        if status == "supported" and not targets:
            fail(f"{os_name} is supported but has no targets")
        if status == "unsupported" and not item.get("reason"):
            fail(f"{os_name} is unsupported but has no reason")
        if status not in {"supported", "unsupported"}:
            fail(f"{os_name} has invalid status {status!r}")
    target_set = {target for item in platforms.values() for target in item.get("targets", [])}
    release_target_configs = config.get("release_targets", [])
    release_targets = {item.get("target") for item in release_target_configs}
    if not release_targets <= target_set:
        fail(f"release targets not declared in support matrix: {sorted(release_targets - target_set)}")
    unsupported_targets = {
        target
        for item in platforms.values()
        if item.get("status") == "unsupported"
        for target in item.get("targets", [])
    }
    if release_targets & unsupported_targets:
        fail(f"release targets include unsupported platforms: {sorted(release_targets & unsupported_targets)}")
    for target in release_target_configs:
        if target.get("smoke") not in {"build-runner", "qemu-aarch64"}:
            fail(f"{target.get('target')} is missing an executable archive smoke policy")
    print("support matrix passed")


def workflow_uses(workflow: Path) -> list[str]:
    uses: list[str] = []
    for line in read_text(workflow).splitlines():
        match = re.search(r"\buses:\s*([^#\s]+)", line)
        if match:
            uses.append(match.group(1).strip("'\""))
    return uses


def validate_action_policy(repo: Path, config: dict) -> list[str]:
    allowed = {item["uses"] for item in config.get("actions_policy", {}).get("allowed_unpinned", [])}
    errors: list[str] = []
    workflow_dir = repo / ".github" / "workflows"
    workflows = sorted([*workflow_dir.glob("*.yml"), *workflow_dir.glob("*.yaml")])
    for workflow in workflows:
        for uses in workflow_uses(workflow):
            if uses.startswith("./"):
                continue
            if uses.startswith("docker://"):
                if "@sha256:" not in uses:
                    errors.append(f"{workflow}: Docker action {uses!r} must be digest-pinned")
                continue
            ref = uses.rsplit("@", 1)[-1] if "@" in uses else ""
            if re.fullmatch(r"[0-9a-fA-F]{40}", ref):
                continue
            if uses not in allowed:
                errors.append(f"{workflow}: action {uses!r} is neither SHA-pinned nor policy-allowed")
    return errors


def command_validate_workflows(args: argparse.Namespace) -> None:
    repo = Path(args.repo).resolve()
    config = release_config(repo)
    errors = validate_action_policy(repo, config)

    workflow_checks = {
        ".github/workflows/ci.yml": [
            "name: Check",
            "name: Format",
            "name: Clippy",
            "name: Test",
            "name: Build",
            "name: Documentation consistency",
            "name: Release gate policy",
            "name: Clean worktree",
            "name: Required gates",
            "scripts/check_clean_worktree.py --self-test",
            "needs: [fmt, shell-scripts, clippy, test, build, documentation-consistency, release-gate-policy, clean-worktree]",
        ],
        ".github/workflows/security.yml": [
            "name: Security Audit",
            "name: License & Dependency Check",
            "name: Check Outdated Dependencies",
            "name: Minimum Supported Rust Version",
            "name: Workflow policy",
            "name: Required security gates",
            "cargo outdated --workspace --root-deps-only --exit-code 1",
            "needs: [audit, deny, outdated, msrv, workflow-policy]",
        ],
        ".github/workflows/release.yml": [
            "release-preflight:",
            "quality-gates:",
            "security-gates:",
            "verify-release-artifacts:",
            "publish-release:",
            "needs: [create-release, verify-release-artifacts]",
            "needs: [create-release, verify-release-artifacts, publish-sage-cli]",
            "needs.publish-sage-cli.result",
            "actions/attest-build-provenance@v2",
            "cargo outdated --workspace --root-deps-only --exit-code 1",
            "smoke-archive",
            "qemu-aarch64",
            "macos-15-intel",
            "cargo-install-smoke",
            "cargo install sage-cli",
        ],
    }
    for rel_path, required in workflow_checks.items():
        content = read_text(repo / rel_path)
        for needle in required:
            if needle not in content:
                errors.append(f"{rel_path} missing required workflow marker {needle!r}")

    if errors:
        fail("; ".join(errors))
    print("workflow policy passed")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_sha_file(asset: Path) -> str:
    sha_path = asset.with_name(asset.name + ".sha256")
    if not sha_path.exists():
        fail(f"missing checksum for {asset.name}")
    expected = sha_path.read_text(encoding="utf-8").split()[0]
    actual = sha256_file(asset)
    if expected != actual:
        fail(f"checksum mismatch for {asset.name}: {expected} != {actual}")
    return actual


def archive_members(path: Path) -> list[str]:
    if path.suffix == ".zip":
        with zipfile.ZipFile(path) as archive:
            return archive.namelist()
    with tarfile.open(path, "r:*") as archive:
        return archive.getnames()


def ensure_safe_member(dest: Path, name: str) -> None:
    base = dest.resolve()
    target = (base / name).resolve()
    if target != base and base not in target.parents:
        fail(f"archive member escapes extraction directory: {name}")


def extract_archive(path: Path, dest: Path) -> None:
    if path.suffix == ".zip":
        with zipfile.ZipFile(path) as archive:
            for member in archive.namelist():
                ensure_safe_member(dest, member)
            archive.extractall(dest)
    else:
        with tarfile.open(path, "r:*") as archive:
            for member in archive.getmembers():
                ensure_safe_member(dest, member.name)
            archive.extractall(dest)


def find_asset(root: Path, name: str) -> Path:
    matches = list(root.rglob(name))
    if not matches:
        fail(f"missing release artifact {name}")
    if len(matches) > 1:
        fail(f"multiple release artifacts named {name}: {matches}")
    return matches[0]


def smoke_archive(asset: Path, expected_version: str, runner_command: str | None = None) -> None:
    with tempfile.TemporaryDirectory(prefix="sage-release-smoke-") as temp:
        dest = Path(temp)
        extract_archive(asset, dest)
        candidates = [path for path in dest.rglob("sage") if path.is_file()]
        if not candidates:
            fail(f"{asset.name} does not contain a sage binary")
        binary = candidates[0]
        binary.chmod(binary.stat().st_mode | 0o111)
        command = [str(binary), "--version"]
        if runner_command:
            command = [*shlex.split(runner_command), str(binary), "--version"]
        output = subprocess.run(
            command,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        if output.returncode != 0 or expected_version not in output.stdout:
            fail(f"{asset.name} version smoke failed: {output.stdout.strip()}")


def command_smoke_archive(args: argparse.Namespace) -> None:
    smoke_archive(Path(args.archive).resolve(), args.expected_version.lstrip("v"), args.runner_command)
    print("archive smoke passed")


def command_verify_artifacts(args: argparse.Namespace) -> None:
    repo = Path(args.repo).resolve()
    artifact_dir = Path(args.artifact_dir).resolve()
    version = args.version
    config = release_config(repo)
    manifest_lines: list[str] = []
    manifest_entries: list[dict] = []
    for target in config.get("release_targets", []):
        archive_name = f"sage-{version}-{target['target']}.{target['archive']}"
        asset = find_asset(artifact_dir, archive_name)
        members = archive_members(asset)
        for required in ("sage", "LICENSE", "NOTICE"):
            if not any(Path(member).name == required for member in members):
                fail(f"{archive_name} missing {required}")
        digest = verify_sha_file(asset)
        manifest_lines.append(f"{digest}  {archive_name}")
        manifest_entries.append(
            {
                "name": archive_name,
                "target": target["target"],
                "os": target["os"],
                "archive": target["archive"],
                "sha256": digest,
                "signature": "github-artifact-attestation",
                "smoke": target["smoke"],
            }
        )
        if target.get("smoke") == "build-runner" and target["os"] == "linux" and platform.system() == "Linux":
            smoke_archive(asset, version.lstrip("v"))
    if args.write_manifest:
        (artifact_dir / "SHA256SUMS").write_text("\n".join(manifest_lines) + "\n", encoding="utf-8")
        (artifact_dir / "release-manifest.json").write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "version": version,
                    "signature_policy": "github-artifact-attestations",
                    "artifacts": manifest_entries,
                },
                indent=2,
                sort_keys=True,
            )
            + "\n",
            encoding="utf-8",
        )
    print("artifact verification passed")


def command_cargo_install_smoke(args: argparse.Namespace) -> None:
    repo = Path(args.repo).resolve()
    expected = args.expected_version.lstrip("v")
    with tempfile.TemporaryDirectory(prefix="sage-cargo-install-") as temp:
        root = Path(temp)
        subprocess.run(
            ["cargo", "install", "--path", "crates/sage-cli", "--root", str(root), "--locked", "--debug"],
            cwd=repo,
            check=True,
        )
        binary = root / "bin" / ("sage.exe" if os.name == "nt" else "sage")
        output = subprocess.run(
            [str(binary), "--version"],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        if output.returncode != 0 or expected not in output.stdout:
            fail(f"cargo install smoke failed: {output.stdout.strip()}")
    print("cargo install smoke passed")


def expect_failure(label: str, fn) -> None:
    try:
        fn()
    except GateError:
        return
    fail(f"self-test expected failure did not happen: {label}")


def write_minimal_repo(root: Path, version: str = "1.2.3", changelog: bool = True) -> None:
    (root / ".github" / "workflows").mkdir(parents=True)
    (root / "crates" / "sage-cli").mkdir(parents=True)
    (root / "Cargo.toml").write_text(
        f"""[package]
name = "sage"
version = "{version}"

[workspace]
members = ["crates/sage-cli"]

[workspace.package]
version = "{version}"
""",
        encoding="utf-8",
    )
    (root / "crates" / "sage-cli" / "Cargo.toml").write_text(
        """[package]
name = "sage-cli"
version.workspace = true
""",
        encoding="utf-8",
    )
    (root / "CHANGELOG.md").write_text(
        f"# Changelog\n\n## [{version}] - 2026-01-01\n" if changelog else "# Changelog\n",
        encoding="utf-8",
    )
    shutil.copyfile(Path(__file__).resolve().parents[1] / ".github" / "release-support.json", root / ".github" / "release-support.json")


def command_self_test(_args: argparse.Namespace) -> None:
    with tempfile.TemporaryDirectory(prefix="sage-release-gate-test-") as temp:
        repo = Path(temp)
        write_minimal_repo(repo)
        command_preflight(argparse.Namespace(repo=str(repo), tag="v1.2.3"))
        expect_failure("version mismatch", lambda: command_preflight(argparse.Namespace(repo=str(repo), tag="v1.2.4")))
        (repo / "CHANGELOG.md").write_text("# Changelog\n", encoding="utf-8")
        expect_failure("missing changelog", lambda: command_preflight(argparse.Namespace(repo=str(repo), tag="v1.2.3")))

    with tempfile.TemporaryDirectory(prefix="sage-workflow-policy-test-") as temp:
        repo = Path(temp)
        write_minimal_repo(repo)
        (repo / ".github" / "workflows" / "policy.yaml").write_text(
            "name: Policy\njobs:\n  scan:\n    steps:\n      - uses: vendor/action@v1\n",
            encoding="utf-8",
        )
        errors = validate_action_policy(repo, release_config(repo))
        if not any("policy.yaml" in error for error in errors):
            fail("self-test expected .yaml workflow action policy violation")

    with tempfile.TemporaryDirectory(prefix="sage-docker-policy-test-") as temp:
        repo = Path(temp)
        write_minimal_repo(repo)
        (repo / ".github" / "workflows" / "docker.yml").write_text(
            "name: Docker\njobs:\n  scan:\n    steps:\n      - uses: docker://alpine:latest\n",
            encoding="utf-8",
        )
        errors = validate_action_policy(repo, release_config(repo))
        if not any("Docker action" in error for error in errors):
            fail("self-test expected mutable Docker action policy violation")

    with tempfile.TemporaryDirectory(prefix="sage-release-artifact-test-") as temp:
        root = Path(temp)
        repo = root / "repo"
        artifacts = root / "artifacts"
        repo.mkdir()
        artifacts.mkdir()
        write_minimal_repo(repo)
        name = "sage-v1.2.3-x86_64-unknown-linux-gnu.tar.gz"
        package = root / "package"
        package.mkdir()
        for file_name in ("sage", "LICENSE", "NOTICE"):
            (package / file_name).write_text("#!/bin/sh\necho sage 1.2.3\n", encoding="utf-8")
        with tarfile.open(artifacts / name, "w:gz") as archive:
            for item in package.iterdir():
                archive.add(item, arcname=item.name)
        expect_failure(
            "missing checksum",
            lambda: command_verify_artifacts(
                argparse.Namespace(repo=str(repo), artifact_dir=str(artifacts), version="v1.2.3", write_manifest=False)
            ),
        )
        digest = sha256_file(artifacts / name)
        (artifacts / f"{name}.sha256").write_text(f"{digest}  {name}\n", encoding="utf-8")
        expect_failure(
            "missing matrix artifacts",
            lambda: command_verify_artifacts(
                argparse.Namespace(repo=str(repo), artifact_dir=str(artifacts), version="v1.2.3", write_manifest=False)
            ),
        )

    with tempfile.TemporaryDirectory(prefix="sage-release-artifact-pass-") as temp:
        root = Path(temp)
        repo = root / "repo"
        artifacts = root / "artifacts"
        repo.mkdir()
        artifacts.mkdir()
        write_minimal_repo(repo)
        config = release_config(repo)
        for target in config["release_targets"]:
            name = f"sage-v1.2.3-{target['target']}.{target['archive']}"
            package = root / f"package-{target['target']}"
            package.mkdir()
            for file_name in ("sage", "LICENSE", "NOTICE"):
                (package / file_name).write_text("#!/bin/sh\necho sage 1.2.3\n", encoding="utf-8")
            with tarfile.open(artifacts / name, "w:gz") as archive:
                for item in package.iterdir():
                    archive.add(item, arcname=item.name)
            digest = sha256_file(artifacts / name)
            (artifacts / f"{name}.sha256").write_text(f"{digest}  {name}\n", encoding="utf-8")

        command_support_matrix(argparse.Namespace(repo=str(repo)))
        command_verify_artifacts(
            argparse.Namespace(repo=str(repo), artifact_dir=str(artifacts), version="v1.2.3", write_manifest=True)
        )
        manifest = read_json(artifacts / "release-manifest.json")
        if manifest.get("signature_policy") != "github-artifact-attestations":
            fail("release manifest is missing the signature policy")
    print("release gate self-test passed")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    preflight = sub.add_parser("preflight")
    preflight.add_argument("--repo", default=".")
    preflight.add_argument("--tag", required=True)
    preflight.set_defaults(func=command_preflight)

    matrix = sub.add_parser("support-matrix")
    matrix.add_argument("--repo", default=".")
    matrix.set_defaults(func=command_support_matrix)

    workflows = sub.add_parser("validate-workflows")
    workflows.add_argument("--repo", default=".")
    workflows.set_defaults(func=command_validate_workflows)

    artifacts = sub.add_parser("verify-artifacts")
    artifacts.add_argument("--repo", default=".")
    artifacts.add_argument("--artifact-dir", required=True)
    artifacts.add_argument("--version", required=True)
    artifacts.add_argument("--write-manifest", action="store_true")
    artifacts.set_defaults(func=command_verify_artifacts)

    cargo_smoke = sub.add_parser("cargo-install-smoke")
    cargo_smoke.add_argument("--repo", default=".")
    cargo_smoke.add_argument("--expected-version", required=True)
    cargo_smoke.set_defaults(func=command_cargo_install_smoke)

    archive_smoke = sub.add_parser("smoke-archive")
    archive_smoke.add_argument("--archive", required=True)
    archive_smoke.add_argument("--expected-version", required=True)
    archive_smoke.add_argument("--runner-command")
    archive_smoke.set_defaults(func=command_smoke_archive)

    self_test = sub.add_parser("self-test")
    self_test.set_defaults(func=command_self_test)

    args = parser.parse_args()
    try:
        args.func(args)
        return 0
    except GateError as error:
        print(f"release gate failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
