#!/usr/bin/env python3

import argparse
import fnmatch
import json
import subprocess
import sys
from pathlib import Path


def load_json(path: Path):
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def matches(path: str, patterns: list[str]) -> bool:
    return any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns)


def scenario_for_path(path: str) -> str | None:
    prefix = "nix/scenarios/"
    if not path.startswith(prefix) or not path.endswith(".nix"):
        return None
    stem = path.removeprefix(prefix).removesuffix(".nix")
    return f"scenario-{stem}"


def classify(path: str, contracts: dict, inventory: set[str]) -> tuple[str, set[str]]:
    scenario = scenario_for_path(path)
    if scenario is not None:
        if scenario in inventory:
            return f"scenario {scenario}", {scenario}
        return "unmapped scenario definition (conservative all)", set(inventory)
    if matches(path, contracts["global"]):
        return "cross-cutting input", set(inventory)
    if matches(path, contracts["nonProduct"]):
        return "outside VM product contracts", set()
    for rule in contracts["rules"]:
        if matches(path, rule["paths"]):
            surface = rule["surface"]
            selected = {
                name
                for name, surfaces in contracts["scenarios"].items()
                if name in inventory and surface in surfaces
            }
            return f"surface {surface}", selected
    return "unclassified input (conservative all)", set(inventory)


def validate_contracts(contracts: dict, repo: Path, inventory: set[str]) -> None:
    if contracts.get("schemaVersion") != 1:
        raise ValueError("scenario contract schemaVersion must be 1")
    declared = set(contracts["scenarios"])
    if declared != inventory:
        missing = sorted(inventory - declared)
        stale = sorted(declared - inventory)
        raise ValueError(f"scenario inventory mismatch: missing={missing}, stale={stale}")
    surfaces = {rule["surface"] for rule in contracts["rules"]}
    referenced = {surface for values in contracts["scenarios"].values() for surface in values}
    unknown_surfaces = sorted(referenced - surfaces)
    if unknown_surfaces:
        raise ValueError(f"scenario contracts reference unknown surfaces: {unknown_surfaces}")

    tracked = subprocess.run(
        ["git", "ls-files"],
        cwd=repo,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout.splitlines()
    product_inputs = [
        path
        for path in tracked
        if path in {"Cargo.lock", "Cargo.toml", "flake.nix"}
        or (path.startswith("crates/") and (path.endswith(".rs") or path.endswith("Cargo.toml")))
        or path.startswith("nix/scenarios/")
    ]
    unclassified = [
        path
        for path in product_inputs
        if classify(path, contracts, inventory)[0].startswith("unclassified")
    ]
    if unclassified:
        raise ValueError(f"tracked product inputs lack a contract classification: {unclassified}")


def inventory_from_checks(path: Path) -> set[str]:
    checks = load_json(path)
    return {name for name in checks if name.startswith("scenario-")}


def command_validate(args) -> int:
    contracts = load_json(args.contracts)
    inventory = inventory_from_checks(args.checks)
    validate_contracts(contracts, args.repo, inventory)
    print(
        f"scenario contracts valid: {len(inventory)} scenarios, "
        f"{len(contracts['rules'])} ordered surface rules"
    )
    return 0


def command_select(args) -> int:
    contracts = load_json(args.contracts)
    inventory = inventory_from_checks(args.checks)
    validate_contracts(contracts, args.repo, inventory)
    changes = [line for line in args.changes.read_text(encoding="utf-8").splitlines() if line]
    reasons: dict[str, dict[str, list[str]]] = {name: {} for name in inventory}
    classified = []
    for path in sorted(set(changes)):
        label, selected = classify(path, contracts, inventory)
        classified.append({"path": path, "classification": label})
        for name in selected:
            reasons[name].setdefault(label, []).append(path)
    selections = []
    for name in sorted(inventory):
        scenario_reasons = []
        for label, paths in reasons[name].items():
            if len(paths) == 1:
                scenario_reasons.append(f"{paths[0]}: {label}")
            else:
                scenario_reasons.append(f"{label} ({len(paths)} changed paths)")
        if scenario_reasons:
            selections.append(
                {
                    "status": "selected",
                    "name": name,
                    "reason": "; ".join(scenario_reasons),
                }
            )
        else:
            selections.append(
                {
                    "status": "skipped",
                    "name": name,
                    "reason": "no changed path intersects declared contracts",
                }
            )
    json.dump({"changes": classified, "selections": selections}, sys.stdout, indent=2)
    print()
    return 0


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser()
    result.add_argument("--contracts", type=Path, required=True)
    result.add_argument("--repo", type=Path, required=True)
    result.add_argument("--checks", type=Path, required=True)
    subcommands = result.add_subparsers(dest="command", required=True)
    subcommands.add_parser("validate")
    select = subcommands.add_parser("select")
    select.add_argument("--changes", type=Path, required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        if args.command == "validate":
            return command_validate(args)
        return command_select(args)
    except (OSError, subprocess.CalledProcessError, ValueError) as error:
        print(f"scenario contract error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
