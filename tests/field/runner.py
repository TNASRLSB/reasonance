#!/usr/bin/env python3
"""Reasonance Field Test Runner v2.

Usage:
    python runner.py --all                    Run all 67 tests
    python runner.py --suite smoke            Run one suite
    python runner.py --test smoke_01          Run one test
    python runner.py --no-launch              App already running
    python runner.py --verbose                Screenshot every step
    python runner.py --retry 2               Retry failures
    python runner.py --calibrate             Print window geometry
    python runner.py --list                   List with source labels
"""

import argparse
import os
import sys
import time
from pathlib import Path

import yaml

from lib.app import ReasonanceApp
from lib.context import TestContext
from lib.telemetry import Telemetry
from lib.executor import YamlExecutor
from lib.actions import ACTION_REGISTRY
from lib.screen import get_screenshot_dir
from lib.window import focus, minimize_others, maximize, get_geometry
from lib.report import generate_report, generate_bug_report
from suites import discover_python_tests

FIELD_DIR = Path(__file__).parent
SCENARIOS_DIR = FIELD_DIR / "scenarios"
REPORTS_DIR = FIELD_DIR / "reports"
BUGS_DIR = FIELD_DIR / "bugs"

SUITES = ["smoke", "e2e", "stress", "edge", "cross", "security", "visual", "integrity"]


def load_scenarios(suite: str = None, test_id: str = None) -> list[dict]:
    """Load test scenarios from YAML files."""
    scenarios = []
    files = (
        [SCENARIOS_DIR / f"{s}.yaml" for s in SUITES]
        if suite is None
        else [SCENARIOS_DIR / f"{suite}.yaml"]
    )

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


def list_scenarios(python_tests: dict):
    """Print all available test scenarios with [Python] or [YAML] source label."""
    for suite in SUITES:
        scenarios = load_scenarios(suite=suite)
        if scenarios:
            print(f"\n--- {suite.upper()} ({len(scenarios)} tests) ---")
            for s in scenarios:
                llm = " [LLM]" if s.get("requires_llm") else ""
                source = "[py]" if s["id"] in python_tests else "[yaml]"
                print(f"  {s['id']}: {s['name']}{llm}  {source}")


def run_python_test(test_fn, scenario: dict, ctx: "TestContext") -> dict:
    """Run a Python test function and return a result dict."""
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
        "steps_executed": 0,
        "steps_total": 0,
    }

    try:
        test_fn(ctx)
        result["status"] = "pass"
    except AssertionError as e:
        result["status"] = "fail"
        result["errors"].append(f"AssertionError: {e}")
    except Exception as e:
        result["status"] = "fail"
        result["errors"].append(f"{type(e).__name__}: {e}")

    result["duration_ms"] = int((time.time() - start) * 1000)

    # Collect screenshots: scan screenshot_dir for files matching test_id
    screenshot_dir = ctx.screenshot_dir
    test_id = scenario["id"]
    try:
        for fname in sorted(os.listdir(screenshot_dir)):
            if test_id in fname and fname.endswith(".png"):
                result["screenshots"].append(os.path.join(screenshot_dir, fname))
    except OSError:
        pass

    return result


def calibrate():
    """Print window geometry for reasonance."""
    print("Querying window geometry for 'reasonance'...")
    try:
        rect = get_geometry("reasonance")
        print(f"  x={rect['x']}, y={rect['y']}, width={rect['width']}, height={rect['height']}")
    except Exception as e:
        print(f"  Failed: {e}")
        print("  Make sure Reasonance is running and focused.")


def main():
    parser = argparse.ArgumentParser(description="Reasonance Field Test Runner v2")
    parser.add_argument("--all", action="store_true", help="Run all suites")
    parser.add_argument("--suite", type=str, help="Run specific suite")
    parser.add_argument("--test", type=str, help="Run single test by ID")
    parser.add_argument("--no-launch", action="store_true", help="Don't launch app (assume already running)")
    parser.add_argument("--verbose", action="store_true", help="Screenshot every step")
    parser.add_argument("--retry", type=int, default=0, help="Number of retry attempts for failures")
    parser.add_argument("--calibrate", action="store_true", help="Print window geometry")
    parser.add_argument("--list", action="store_true", help="List all scenarios with source labels")
    args = parser.parse_args()

    # Discover Python tests upfront (needed for --list and for running)
    python_tests = discover_python_tests()

    if args.calibrate:
        calibrate()
        return

    if args.list:
        list_scenarios(python_tests)
        return

    if args.test:
        scenarios = load_scenarios(test_id=args.test)
    elif args.suite:
        scenarios = load_scenarios(suite=args.suite)
    elif args.all:
        scenarios = load_scenarios()
    else:
        parser.print_help()
        return

    if not scenarios:
        print("No scenarios found.")
        return

    print(f"Running {len(scenarios)} test(s)...")

    app = ReasonanceApp()
    screenshot_dir = get_screenshot_dir()
    yaml_executor = YamlExecutor(ACTION_REGISTRY)
    results = []
    failed = 0
    run_start = time.time()

    try:
        if not args.no_launch:
            print("Launching Reasonance...")
            app.launch()
            if not app.wait_ready(timeout=120):
                print("ERROR: App failed to start!")
                print(app.logs()[-500:])
                sys.exit(1)

            print("App ready. Preparing window...")
            time.sleep(2)
            maximize("reasonance")
            focus("reasonance")
            minimize_others("reasonance")
            time.sleep(1)

        # Start telemetry
        telemetry = Telemetry(app)
        telemetry.start()

        # Get window geometry (fallback to 1280x800)
        try:
            window_rect = get_geometry("reasonance")
        except Exception as e:
            print(f"Warning: could not get window geometry ({e}), using 1280x800 fallback")
            window_rect = {"x": 0, "y": 0, "width": 1280, "height": 800}

        for scenario in scenarios:
            test_id = scenario["id"]
            print(f"  [{test_id}] {scenario['name']}...", end=" ", flush=True)

            # Skip LLM tests if FIELD_TEST_LLM not set
            if scenario.get("requires_llm") and not os.environ.get("FIELD_TEST_LLM"):
                source = "py" if test_id in python_tests else "yaml"
                print(f"SKIP (requires LLM)")
                results.append({
                    "id": test_id,
                    "name": scenario["name"],
                    "suite": scenario.get("suite", "unknown"),
                    "status": "skip",
                    "duration_ms": 0,
                    "screenshots": [],
                    "errors": [],
                    "notes": "Skipped: requires_llm=true, set FIELD_TEST_LLM=1 to enable",
                    "steps_executed": 0,
                    "steps_total": 0,
                })
                continue

            # Build TestContext for this test
            ctx = TestContext(
                app=app,
                telemetry=telemetry,
                screenshot_dir=screenshot_dir,
                test_id=test_id,
                test_name=scenario["name"],
                verbose=args.verbose,
                window_rect=window_rect,
            )
            ctx.mark_error_checkpoint()

            # Determine source label
            is_python = test_id in python_tests
            source_label = "py" if is_python else "yaml"

            # Attempt with retry support
            max_attempts = 1 + args.retry
            result = None
            for attempt in range(max_attempts):
                telemetry.mark("test_start", test_id=test_id, attempt=attempt)

                if is_python:
                    result = run_python_test(python_tests[test_id], scenario, ctx)
                else:
                    result = yaml_executor.execute(scenario, ctx)

                telemetry.mark("test_end", test_id=test_id, status=result["status"])

                if result["status"] == "pass":
                    break
                if attempt < max_attempts - 1:
                    print(f"RETRY ({attempt + 1}/{args.retry})...", end=" ", flush=True)
                    time.sleep(1)

            results.append(result)
            print(f"{result['status'].upper()} [{source_label}] ({result['duration_ms']}ms)")

        telemetry_summary = telemetry.stop()
        timeline = telemetry.timeline()

        os.makedirs(str(REPORTS_DIR), exist_ok=True)
        os.makedirs(str(BUGS_DIR), exist_ok=True)
        generate_report(results, str(REPORTS_DIR), telemetry_summary=telemetry_summary, timeline=timeline)

        for r in results:
            if r["status"] == "fail":
                # Build a timeline excerpt for this test
                tl_excerpt = [
                    ev for ev in timeline
                    if ev.get("data", {}).get("test_id") == r["id"]
                    or ev.get("test_id") == r["id"]
                ]
                generate_bug_report(r, str(BUGS_DIR), timeline_excerpt=tl_excerpt or None)

        passed = sum(1 for r in results if r["status"] == "pass")
        failed = sum(1 for r in results if r["status"] == "fail")
        skipped = sum(1 for r in results if r["status"] == "skip")
        total_s = time.time() - run_start

        rss_peak = telemetry_summary.get("rss_peak_mb")
        event_count = telemetry_summary.get("event_count", 0)

        print(f"\nResults: {passed} passed, {failed} failed, {skipped} skipped / {len(results)} total ({total_s:.1f}s)")
        if rss_peak is not None:
            print(f"Telemetry: RSS peak {rss_peak:.1f} MB, {event_count} events")
        else:
            print(f"Telemetry: {event_count} events")
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
