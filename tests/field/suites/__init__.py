"""Python test suite discovery.

Convention: suites/{suite_name}.py with test_{test_id}(ctx) functions.
If a test ID exists in both YAML and Python, Python wins.
"""

from __future__ import annotations

import importlib
import inspect
from pathlib import Path
from typing import Callable

SUITES_DIR = Path(__file__).parent


def discover_python_tests() -> dict[str, Callable]:
    """Return {test_id: test_function} for all Python test suites.

    Scans suites/*.py for functions named test_{id}(ctx).
    E.g. test_smoke_07 in suites/smoke.py → key "smoke_07"
    """
    tests: dict[str, Callable] = {}
    for py_file in sorted(SUITES_DIR.glob("*.py")):
        if py_file.name.startswith("_"):
            continue
        module_name = f"suites.{py_file.stem}"
        try:
            mod = importlib.import_module(module_name)
        except ImportError as e:
            print(f"Warning: could not import {module_name}: {e}")
            continue

        for name, fn in inspect.getmembers(mod, inspect.isfunction):
            if name.startswith("test_"):
                test_id = name[5:]  # strip "test_" prefix
                tests[test_id] = fn
    return tests
