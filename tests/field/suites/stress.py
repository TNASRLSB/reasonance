"""Stress test suite — Python overrides.

Functions named test_{id}(ctx) override YAML scenarios of the same ID.
"""

from __future__ import annotations

import os
import time

from lib.actions import (
    assert_no_new_errors,
    close_file,
    focus_window,
    press_key,
    screenshot,
    search_palette,
    snapshot_performance,
    toolbar_settings,
    type_text,
    wait_ms,
)
from lib.context import TestContext

# File names cycled when we need many distinct filenames
_CYCLE_FILES = [
    "lib.rs",
    "Cargo.toml",
    "tauri.conf.json",
    "App.svelte",
    "main.ts",
    "package.json",
    "README.md",
    "tsconfig.json",
    "vite.config.ts",
    "svelte.config.js",
]


def test_stress_26(ctx: TestContext) -> None:
    """50+ files open."""
    snapshot_performance(ctx)

    # Open 50 files by cycling through available filenames
    for i in range(50):
        filename = _CYCLE_FILES[i % len(_CYCLE_FILES)]
        search_palette(ctx)
        wait_ms(ctx, 300)
        type_text(ctx, filename)
        wait_ms(ctx, 300)
        press_key(ctx, "Return")
        wait_ms(ctx, 500)

    screenshot(ctx, "stress-26-50-files-open")
    snapshot_performance(ctx)
    assert_no_new_errors(ctx)


def test_stress_28(ctx: TestContext) -> None:
    """Large file in editor."""
    large_file = "/tmp/reasonance-large-test.txt"

    # Generate a ~10 MB file
    line = "A" * 99 + "\n"  # 100 bytes per line
    total_lines = 10 * 1024 * 1024 // 100  # ~100 000 lines
    try:
        with open(large_file, "w") as fh:
            for _ in range(total_lines):
                fh.write(line)

        # Open via search palette (absolute path)
        search_palette(ctx)
        wait_ms(ctx, 500)
        type_text(ctx, large_file)
        wait_ms(ctx, 500)
        press_key(ctx, "Return")
        wait_ms(ctx, 5000)

        screenshot(ctx, "stress-28-large-file")
        snapshot_performance(ctx)
    finally:
        if os.path.exists(large_file):
            os.unlink(large_file)


def test_stress_31(ctx: TestContext) -> None:
    """Memory leak check: 100 open/edit/close cycles."""
    snapshot_performance(ctx)

    for i in range(100):
        search_palette(ctx)
        wait_ms(ctx, 300)
        type_text(ctx, "lib.rs")
        wait_ms(ctx, 300)
        press_key(ctx, "Return")
        wait_ms(ctx, 500)
        type_text(ctx, "x")
        wait_ms(ctx, 100)
        close_file(ctx)
        wait_ms(ctx, 200)

        # Snapshot every 25 iterations
        if (i + 1) % 25 == 0:
            snapshot_performance(ctx)

    snapshot_performance(ctx)
    assert_no_new_errors(ctx)


def test_stress_32(ctx: TestContext) -> None:
    """Rapid UI interaction."""
    # Open 5 files
    open_files = ["lib.rs", "Cargo.toml", "tauri.conf.json", "App.svelte", "main.ts"]
    for filename in open_files:
        search_palette(ctx)
        wait_ms(ctx, 300)
        type_text(ctx, filename)
        wait_ms(ctx, 300)
        press_key(ctx, "Return")
        wait_ms(ctx, 500)

    # Rapid tab switches: cycle ctrl+1 through ctrl+5, 50 times total
    tab_keys = ["ctrl+1", "ctrl+2", "ctrl+3", "ctrl+4", "ctrl+5"]
    for i in range(50):
        press_key(ctx, tab_keys[i % len(tab_keys)])
        wait_ms(ctx, 100)

    # Rapid settings toggle: open and close 10 times
    for _ in range(10):
        toolbar_settings(ctx)
        wait_ms(ctx, 100)
        press_key(ctx, "Escape")
        wait_ms(ctx, 100)

    screenshot(ctx, "stress-32-rapid-ui")
    assert_no_new_errors(ctx)


def test_stress_33(ctx: TestContext) -> None:
    """Startup benchmark: 5 launches, measure times."""
    startup_times: list[float] = []

    for i in range(5):
        ctx.app.kill()
        wait_ms(ctx, 2000)

        t_start = time.time()
        ctx.app.launch()
        ready = ctx.app.wait_ready(30)
        elapsed = time.time() - t_start

        if ready:
            focus_window(ctx)
            wait_ms(ctx, 500)

        startup_times.append(elapsed)
        snapshot_performance(ctx)

    avg = sum(startup_times) / len(startup_times)
    max_time = max(startup_times)

    assert avg < 10, f"Average startup time {avg:.1f}s exceeds 10s limit"
    assert max_time < 15, f"Max startup time {max_time:.1f}s exceeds 15s limit"
