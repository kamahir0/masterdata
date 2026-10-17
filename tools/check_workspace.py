#!/usr/bin/env python3
"""現在の文書リンクと再開pointerを検証する。製品の適合性は検証しない。"""

from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parent.parent
STAGES = {
    "designing", "decision-required", "implementation-ready",
    "verification-ready", "correction-ready", "objective-complete",
}


def git(*args: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, check=False,
    )


def headings(path: Path) -> set[str]:
    anchors: set[str] = set()
    counts: dict[str, int] = {}
    in_code = False
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("```"):
            in_code = not in_code
        if in_code or not re.match(r"^#{1,6} ", line):
            continue
        title = re.sub(r"^#+\s+", "", line).strip().lower()
        slug = re.sub(r"[^\w\- ]", "", title).replace(" ", "-")
        count = counts.get(slug, 0)
        counts[slug] = count + 1
        anchors.add(f"{slug}-{count}" if count else slug)
    return anchors


def check() -> list[str]:
    errors: list[str] = []
    required = [
        "AGENTS.md", "README.md", "LICENSE", "docs/product/direction.md",
        "docs/current-objective.md", "docs/execution-state.md",
        "docs/execution-workflow.md", "docs/contributing/specification-workflow.md",
        "docs/contributing/documentation-policy.md", "docs/specs/README.md",
        ".github/workflows/workspace.yml",
    ]
    required += [
        f".agents/skills/{name}/SKILL.md"
        for name in ("refine-spec", "review-spec", "implement-spec", "review-code")
    ]
    for name in required:
        if not (ROOT / name).is_file():
            errors.append(f"missing: {name}")

    listing = git("ls-files", "--cached", "--others", "--exclude-standard", "-z")
    if listing.returncode:
        errors.append("Git working treeを確認できません")
        return errors

    for name in sorted(set(listing.stdout.split("\0")) - {""}):
        path = ROOT / name
        if path.suffix != ".md" or not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        for target in re.findall(r"\]\(([^\s)]+)\)", text):
            url = urlsplit(target.strip("<>"))
            if url.scheme or url.netloc:
                continue
            destination = (path.parent / unquote(url.path)).resolve() if url.path else path
            if not destination.is_relative_to(ROOT):
                errors.append(f"{name}: repository外のlocal link: {target}")
            elif not destination.exists():
                errors.append(f"{name}: broken link: {target}")
            elif url.fragment and destination.suffix == ".md":
                if unquote(url.fragment) not in headings(destination):
                    errors.append(f"{name}: missing heading: {target}")

    state_path = ROOT / "docs/execution-state.md"
    if not state_path.is_file():
        return errors
    state = state_path.read_text(encoding="utf-8")
    values = {}
    for key in ("Stage", "Candidate", "Work base"):
        matches = re.findall(rf"^{key}: (.+)$", state, re.MULTILINE)
        if len(matches) != 1:
            errors.append(f"state: {key}は一つ必要です")
        else:
            values[key] = matches[0]
    stage = values.get("Stage")
    if stage not in STAGES:
        errors.append("state: unknown Stage")
    if stage in {"verification-ready", "correction-ready", "objective-complete"}:
        if values.get("Candidate") == "none":
            errors.append(f"state: {stage}にはCandidate SHAが必要です")
    for key in ("Candidate", "Work base"):
        value = values.get(key)
        if value is None or value == "none":
            continue
        if not re.fullmatch(r"[0-9a-f]{40}", value):
            errors.append(f"state: invalid {key} SHA")
        elif git("cat-file", "-e", f"{value}^{{commit}}").returncode:
            errors.append(f"state: {key} commitがありません")
        elif git("merge-base", "--is-ancestor", value, "HEAD").returncode:
            errors.append(f"state: {key}はHEADのancestorではありません")
    for heading in ("Active work", "Blocking findings"):
        if f"## {heading}\n" not in state:
            errors.append(f"state: missing {heading}")
    if stage == "decision-required" and "## Human decision needed\n" not in state:
        errors.append("state: Human decision neededが必要です")
    if stage == "objective-complete":
        if not re.search(r"## Active work\s+None\.\s*(?:##|\Z)", state):
            errors.append("state: 完了時のActive workはNone.にしてください")
    return errors


if __name__ == "__main__":
    findings = check()
    for finding in findings:
        print(f"ERROR: {finding}", file=sys.stderr)
    if findings:
        raise SystemExit(1)
    print("Workspace integrity: OK (links / state / required entry points)")
