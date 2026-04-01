#!/usr/bin/env python3
"""Reasonance Field Test Runner.

Usage:
    python runner.py --all                    Run all test suites
    python runner.py --suite smoke            Run specific suite
    python runner.py --test smoke_01          Run single test
    python runner.py --fuzz --duration 30     Fuzzing mode (minutes)
    python runner.py --update-baseline        Update visual baselines
    python runner.py --list                   List all scenarios
"""

import argparse
import os
import sys
import time
import yaml
from pathlib import Path

from lib.app import ReasonanceApp
from lib.screen import screenshot, get_screenshot_dir
from lib.window import focus, minimize_others
from lib.report import generate_report, generate_bug_report

FIELD_DIR = Path(__file__).parent
SCENARIOS_DIR = FIELD_DIR / "scenarios"
REPORTS_DIR = FIELD_DIR / "reports"
BUGS_DIR = FIELD_DIR / "bugs"

SUITES = ["smoke", "e2e", "stress", "edge", "cross", "security", "visual", "integrity"]


def load_scenarios(suite: str = None, test_id: str = None) -> list[dict]:
    """Load test scenarios from YAML files."""
    scenarios = []
    files = [SCENARIOS_DIR / f"{s}.yaml" for s in SUITES] if suite is None else [SCENARIOS_DIR / f"{suite}.yaml"]

    for f in files:
        if not f.exists():
            print(f"Warning: {f} not found, skipping")
            continue
        with open(f) as fh:
            data = yaml.safe_load(fh)
            if data:
                scenarios.extend(data)

    if test_id:
        scenarios = [s for s in scenarios if s["id"] == test_id]

    return scenarios


def list_scenarios():
    """Print all available test scenarios."""
    for suite in SUITES:
        scenarios = load_scenarios(suite=suite)
        if scenarios:
            print(f"\n--- {suite.upper()} ({len(scenarios)} tests) ---")
            for s in scenarios:
                llm = " [LLM]" if s.get("requires_llm") else ""
                print(f"  {s['id']}: {s['name']}{llm}")


def run_scenario(scenario: dict, app: ReasonanceApp, screenshot_dir: str) -> dict:
    """Execute a single test scenario."""
    start = time.time()
    result = {
        "id": scenario["id"],
        "name": scenario["name"],
        "suite": scenario.get("suite", "unknown"),
        "status": "pending",
        "duration_ms": 0,
        "screenshots": [],
        "errors": [],
        "notes": "",
    }

    try:
        shot = screenshot(f"{scenario['id']}-before", directory=screenshot_dir)
        result["screenshots"].append(shot)

        errors = app.get_errors()
        frontend_errors = app.get_frontend_errors()

        if errors:
            result["errors"].extend(errors)
        if frontend_errors:
            result["errors"].extend(frontend_errors)

        shot = screenshot(f"{scenario['id']}-after", directory=screenshot_dir)
        result["screenshots"].append(shot)

        result["status"] = "fail" if result["errors"] else "pass"

    except Exception as e:
        result["status"] = "fail"
        result["errors"].append(str(e))

    result["duration_ms"] = int((time.time() - start) * 1000)
    return result


def main():
    parser = argparse.ArgumentParser(description="Reasonance Field Test Runner")
    parser.add_argument("--all", action="store_true", help="Run all suites")
    parser.add_argument("--suite", type=str, help="Run specific suite")
    parser.add_argument("--test", type=str, help="Run single test by ID")
    parser.add_argument("--fuzz", action="store_true", help="Run fuzzing mode")
    parser.add_argument("--duration", type=int, default=30, help="Fuzzing duration in minutes")
    parser.add_argument("--update-baseline", action="store_true", help="Update visual baselines")
    parser.add_argument("--list", action="store_true", help="List all scenarios")
    parser.add_argument("--no-launch", action="store_true", help="Don't launch app (assume already running)")
    args = parser.parse_args()

    if args.list:
        list_scenarios()
        return

    if args.test:
        scenarios = load_scenarios(test_id=args.test)
    elif args.suite:
        scenarios = load_scenarios(suite=args.suite)
    elif args.all:
        scenarios = load_scenarios()
    elif args.fuzz:
        print(f"Fuzzing mode: {args.duration} minutes")
        print("Launch Reasonance and use Claude Code interactively for fuzzing.")
        return
    else:
        parser.print_help()
        return

    if not scenarios:
        print("No scenarios found.")
        return

    print(f"Running {len(scenarios)} test(s)...")

    app = ReasonanceApp()
    screenshot_dir = get_screenshot_dir()
    results = []
    failed = 0

    try:
        if not args.no_launch:
            print("Launching Reasonance...")
            app.launch()
            if not app.wait_ready(timeout=120):
                print("ERROR: App failed to start!")
                print(app.logs()[-500:])
                return

            print("App ready. Preparing window...")
            time.sleep(2)
            focus("reasonance")
            minimize_others("reasonance")
            time.sleep(1)

        for scenario in scenarios:
            print(f"  [{scenario['id']}] {scenario['name']}...", end=" ", flush=True)

            if scenario.get("requires_llm") and not os.environ.get("FIELD_TEST_LLM"):
                print("SKIP (requires LLM)")
                results.append({
                    "id": scenario["id"],
                    "name": scenario["name"],
                    "suite": scenario.get("suite", "unknown"),
                    "status": "skip",
                    "duration_ms": 0,
                    "screenshots": [],
                    "errors": [],
                    "notes": "Skipped: requires_llm=true, set FIELD_TEST_LLM=1 to enable",
                })
                continue

            result = run_scenario(scenario, app, screenshot_dir)
            results.append(result)
            print(result["status"].upper())

        os.makedirs(str(REPORTS_DIR), exist_ok=True)
        os.makedirs(str(BUGS_DIR), exist_ok=True)
        generate_report(results, str(REPORTS_DIR))

        for r in results:
            if r["status"] == "fail":
                generate_bug_report(r, str(BUGS_DIR))

        passed = sum(1 for r in results if r["status"] == "pass")
        failed = sum(1 for r in results if r["status"] == "fail")
        skipped = sum(1 for r in results if r["status"] == "skip")
        print(f"\nResults: {passed} passed, {failed} failed, {skipped} skipped / {len(results)} total")
        print(f"Screenshots: {screenshot_dir}")
        print(f"Report: {REPORTS_DIR}")
        if failed:
            print(f"Bug reports: {BUGS_DIR}")

    finally:
        if not args.no_launch:
            print("Stopping Reasonance...")
            app.kill()

    if failed:
        sys.exit(1)


if __name__ == "__main__":
    main()
