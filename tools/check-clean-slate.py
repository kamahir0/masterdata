#!/usr/bin/env python3
"""Clean-slate input integrity only; contains no product execution adapter."""
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib
import unicodedata
from urllib.parse import unquote

import yaml

ROOT = Path(__file__).resolve().parent.parent
os.chdir(ROOT)
errors = []
counts = {}


def require(condition, message):
    if not condition:
        errors.append(message)


def git(*args):
    return subprocess.check_output(["git", *args], text=True).strip()


def read_json(path):
    try:
        return json.loads(path.read_text())
    except (ValueError, OSError) as error:
        errors.append(f"JSON {path}: {error}")
        return {}


manifest = read_json(Path("docs/rewrite-preparation/decommission-manifest.json"))
freeze = manifest["freezeCommit"]
assets = manifest["assets"]
paths = [a["path"] for a in assets]
require(len(paths) == len(set(paths)), "duplicate decommission asset")
require(git("cat-file", "-t", "refs/tags/legacy-final") == "tag", "freeze tag is not annotated")
require(git("rev-parse", "legacy-final^{commit}") == freeze, "freeze tag target changed")
require(subprocess.run(["git", "merge-base", "--is-ancestor", freeze, "HEAD"]).returncode == 0,
        "clean-slate branch does not descend from freeze")
require(set(git("ls-tree", "-r", "--name-only", "legacy-final").splitlines()) == set(paths),
        "decommission manifest does not inventory exact frozen tree")
for ref in ("refs/heads/main", "refs/remotes/origin/main"):
    if subprocess.run(["git", "show-ref", "--quiet", "--verify", ref]).returncode == 0:
        require(git("rev-parse", ref) == freeze, f"{ref} moved from frozen main")

for asset in assets:
    path = Path(asset["path"])
    classification = asset["classification"]
    require(classification in {"KEEP_AUTHORITY_CORPUS", "KEEP_GOVERNANCE_NEUTRAL", "REMOVE", "REVIEW_TRANSITIONAL"},
            f"unknown classification: {path}")
    require(not path.exists() if classification == "REMOVE" else path.exists(),
            f"retention mismatch: {path}")
    if path.exists() and path.parts[0] == "fixtures":
        require(hashlib.sha256(path.read_bytes()).hexdigest() == asset["sha256"],
                f"frozen fixture bytes changed: {path}")
counts["frozen_assets"] = len(assets)
counts["retired_assets"] = sum(a["classification"] == "REMOVE" for a in assets)

for name in ("apps", "crates", "dotnet", "unity", "scripts", ".cargo", "target", "node_modules",
             "Cargo.toml", "Cargo.lock", "package.json", "package-lock.json", "global.json",
             "rust-toolchain.toml", "legacy", "old", "archive", "reference", "previous"):
    require(not Path(name).exists(), f"forbidden legacy path: {name}")
allowed_source = {"tools/check-clean-slate.py", "fixtures/rewrite-oracle/v1/consumer/Consumer.cs",
                  "fixtures/rewrite-oracle/v1/consumer/minimal/Consumer.cs"}
for path in ROOT.rglob("*"):
    if not path.is_file() or ".git" in path.relative_to(ROOT).parts:
        continue
    rel = path.relative_to(ROOT).as_posix()
    if path.suffix in {".rs", ".ts", ".tsx", ".js", ".mjs", ".cjs", ".cs", ".css", ".csproj", ".sln", ".py", ".exe", ".dll"}:
        require(rel in allowed_source, f"unexpected product/adapter source: {rel}")

oracle = Path("fixtures/rewrite-oracle/v1")
index = read_json(oracle / "manifest.json")
ids = []
for group in ("byteScenarios", "saveScenarios", "structuralScenarios", "pasteScenarios"):
    for identifier in index[group]:
        ids.append(identifier)
        directory = oracle / identifier
        scenario = read_json(directory / "scenario.json")
        require(scenario.get("id") == identifier, f"scenario id mismatch: {identifier}")
        require((directory / "input").is_dir() and (directory / "expected").is_dir(),
                f"missing input/expected: {identifier}")
        expected = scenario["expected"]
        if "source" in expected:
            require((directory / expected["source"]).is_file(), f"missing expected bytes: {identifier}")
        for key in ("allDiskFiles", "candidateSources"):
            if key in expected:
                require((directory / expected[key]).is_dir(), f"missing expected directory: {identifier}")
        operation = scenario["operation"]
        for key in ("source", "schemaSource", "selectedRecordSource"):
            if operation.get(key):
                require((directory / "input" / operation[key]).is_file(), f"missing operation source: {identifier}/{key}")
        if group == "pasteScenarios":
            require((directory / "clipboard.tsv").is_file(), f"missing clipboard: {identifier}")
require(len(ids) == len(set(ids)), "duplicate manifest scenario IDs")
scenario_dirs = {p.parent.name for p in oracle.glob("*/scenario.json")}
require(scenario_dirs == set(ids) | {"consumer", "empty-table"}, "unlisted/missing scenario directory")


def input_path(value):
    # The workflow format explicitly defines this one repository-relative asset.
    if value.startswith("fixtures/full"):
        return Path("fixtures/full")
    return oracle / value


for name in ("workflows.json", "faults.json"):
    data = read_json(oracle / name)
    named = [s["id"] for s in data["scenarios"]]
    require(len(named) == len(set(named)), f"duplicate IDs: {name}")
    for scenario in data["scenarios"]:
        inputs = scenario["input"]
        for value in inputs if isinstance(inputs, list) else [inputs]:
            require(input_path(value).exists(), f"missing workflow input: {name}/{value}")
        if name == "faults.json":
            for key in ("candidate", "reviewedCandidate"):
                if key in scenario:
                    require(input_path(scenario[key]).exists(), f"missing fault candidate: {scenario['id']}")
            for key, value in scenario["expected"].items():
                if key.startswith("diskBytes"):
                    require(input_path(value).exists(), f"missing fault disk oracle: {scenario['id']}")
                elif key == "disk":
                    for target in value.values():
                        require(input_path(target).is_file(), f"missing fault file: {target}")
consumer = read_json(oracle / "consumer/scenario.json")
require(Path(consumer["input"]).is_dir(), "missing full consumer fixture")
require((oracle / "consumer" / consumer["oracle"]).is_file(), "missing consumer oracle")
require((oracle / "empty-table/input/sources/schema.yaml").exists(), "missing empty Table input")
navigation = read_json(oracle / "navigation.json")
require((oracle / navigation["input"]["config"]).is_file(), "missing navigation config")
require(navigation["expected"] == {"tables": 3, "physicalSources": 8, "totalRecords": 12000,
                                  "selectedSourceRows": 2000, "selectedSourceColumns": 20},
        "navigation capacity changed")
capacity = read_json(oracle / "capacity.json")
require(capacity["input"]["records"] == 100000 and capacity["expected"]["pasteChangedCells"] == 10000,
        "capacity oracle changed")

json_files = [*Path("fixtures").rglob("*.json"), *Path("docs/evidence").rglob("*.json")]
for path in json_files:
    read_json(path)
counts["json_files"] = len(json_files)
for path in Path("docs/evidence").rglob("*.jsonl"):
    for line in path.read_text().splitlines():
        try:
            json.loads(line)
        except ValueError as error:
            errors.append(f"JSONL {path}: {error}")
for path in Path("docs/evidence").rglob("*.gz"):
    try:
        with gzip.open(path, "rt") as source:
            data = source.read()
        if path.name.endswith(".json.gz"):
            json.loads(data)
        elif path.name.endswith(".jsonl.gz"):
            for line in data.splitlines():
                json.loads(line)
    except (ValueError, OSError) as error:
        errors.append(f"compressed evidence {path}: {error}")

# YAML syntax only; invalid/ fixtures may be syntactically valid but domain-invalid.
yaml_paths = [*Path("fixtures").rglob("*.yaml"), *Path(".github/workflows").glob("*.yml")]
for path in yaml_paths:
    try:
        list(yaml.safe_load_all(path.read_bytes()))
    except yaml.YAMLError as error:
        errors.append(f"unexpected YAML syntax failure {path}: {error}")
for path in Path("fixtures").rglob("*.toml"):
    try:
        tomllib.loads(path.read_text())
    except ValueError as error:
        errors.append(f"TOML {path}: {error}")
counts["yaml_files"] = len(yaml_paths)

markdown = [Path("README.md"), Path("AGENTS.md"), *Path("docs").rglob("*.md"),
            *Path("skills").rglob("*.md"), *Path("fixtures").rglob("*.md")]
definitions = {}


def heading_slug(heading):
    heading = heading.lower().replace("`", "")
    heading = "".join(c for c in heading if c in "-_" or c.isspace()
                      or unicodedata.category(c)[0] in "LN")
    return heading.replace(" ", "-")


for path in markdown:
    text = re.sub(r"```[^\n]*\n.*?```", "", path.read_text(), flags=re.S)
    for target in re.findall(r"\]\(([^)\s]+)\)", text):
        if re.match(r"[a-zA-Z][a-zA-Z0-9+.-]*:", target):
            continue
        filename, _, fragment = unquote(target).partition("#")
        destination = path.parent / filename if filename else path
        require(destination.exists(), f"broken relative link: {path} -> {target}")
        if fragment and destination.is_file() and destination.suffix == ".md":
            headings = re.findall(r"^#{1,6}\s+(.+)$", destination.read_text(), re.M)
            require(fragment in {heading_slug(h) for h in headings},
                    f"broken heading link: {path} -> {target}")
    if str(path).startswith(("docs/specs/", "docs/gui/")):
        for identifier in re.findall(r"^#{2,6}\s+([A-Z]+(?:-[A-Z]+)*-\d{3})\s*$", text, re.M):
            require(identifier not in definitions, f"duplicate Requirement ID: {identifier}")
            definitions[identifier] = str(path)
        forbidden = r"WorkspaceAuthoringSession|NativeApplicationService|open_table_context|select_source|App\.tsx|masterdata-(?:core|app|dotnet|codegen-csharp)"
        require(not re.search(forbidden, text), f"internal implementation name in current contract: {path}")
counts["markdown_files"] = len(markdown)
counts["requirement_definitions"] = len(definitions)

state = Path("docs/execution-state.md").read_text()
stage = re.search(r"^Stage: (.+)$", state, re.M)
require(stage and stage[1] in {"designing", "decision-required", "implementation-ready", "verification-ready",
                             "correction-ready", "objective-complete"}, "invalid Stage")
require(f"Work base: {freeze}" in state, "Work base does not match frozen Ready HEAD")
candidate = re.search(r"^Candidate: (none|[0-9a-f]{40})$", state, re.M)
require(bool(candidate), "invalid Candidate")
if candidate and candidate[1] != "none":
    require(subprocess.run(["git", "cat-file", "-e", candidate[1] + "^{commit}"]).returncode == 0, "missing Candidate commit")
if stage and stage[1] in {"objective-complete", "verification-ready", "correction-ready"}:
    require(candidate and candidate[1] != "none", "Stage requires exact Candidate")
if stage and stage[1] == "objective-complete":
    require("## Active work\n\nNone." in state and "## Blocking findings\n\nNone." in state,
            "complete state has Active/Blocking work")

for path in ("docs/rewrite-preparation/clean-room-handoff.md", "docs/rewrite-preparation/decommission-report.md",
             "docs/rewrite-preparation/constitution.md", "docs/rewrite-preparation/domain-safety.md",
             "docs/rewrite-preparation/performance.md", "docs/rewrite-preparation/acceptance-matrix.md",
             "docs/gui/rewrite-baseline.md", "docs/evidence/rewrite-readiness/report.md"):
    require(Path(path).is_file(), f"missing rewrite input: {path}")

if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(1)
print(json.dumps({"cleanSlateIntegrity": "PASS", "counts": counts, "freeze": freeze}, indent=2))
print("Legacy runtime tests intentionally absent; product conformance not executed.")
