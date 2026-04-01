"""Integrity tests — kill/recovery and concurrent-write scenarios."""

import os
import signal
import time

from lib.actions import (
    assert_no_new_errors,
    press_key,
    save_file,
    screenshot,
    search_palette,
    snapshot_performance,
    type_text,
    wait_ms,
)
from lib.context import TestContext


def _sigkill_app(ctx: TestContext) -> None:
    """Send SIGKILL to the app process group."""
    pid = ctx.app.pid
    if pid is None:
        return
    try:
        os.killpg(os.getpgid(pid), signal.SIGKILL)
    except ProcessLookupError:
        pass


def test_integrity_63(ctx: TestContext):
    """Kill during save."""
    # Write a test file to disk for the editor to open
    test_file = "/tmp/field-integrity-63.rs"
    with open(test_file, "w") as f:
        f.write("// integrity-63 test file\nfn main() {}\n")

    # Open the test file via the search palette
    search_palette(ctx)
    wait_ms(ctx, 300)
    type_text(ctx, test_file)
    wait_ms(ctx, 300)
    press_key(ctx, "Return")
    wait_ms(ctx, 800)

    # Make an edit
    type_text(ctx, "// edited by test_integrity_63\n")

    # Save and immediately kill
    save_file(ctx)
    _sigkill_app(ctx)

    # Wait briefly for the process to die
    time.sleep(1)

    # Verify the file is non-empty (save may or may not have flushed)
    stat = os.stat(test_file)
    assert stat.st_size > 0, f"File {test_file} is empty after kill-during-save"

    # Relaunch and take a screenshot
    ctx.app.launch()
    ctx.app.wait_ready()
    screenshot(ctx, "integrity-63-after-relaunch")


def test_integrity_64(ctx: TestContext):
    """Transaction semantics: kill mid-operation."""
    # Open lib.rs via search palette
    search_palette(ctx)
    wait_ms(ctx, 300)
    type_text(ctx, "lib.rs")
    wait_ms(ctx, 300)
    press_key(ctx, "Return")
    wait_ms(ctx, 500)

    # Kill immediately after opening
    _sigkill_app(ctx)
    time.sleep(1)

    # Relaunch
    ctx.app.launch()
    ctx.app.wait_ready()

    # Verify no corruption markers in logs
    logs = ctx.app.logs()
    assert "corrupt" not in logs.lower(), "Found 'corrupt' in logs after mid-operation kill"

    screenshot(ctx, "integrity-64-after-relaunch")


def test_integrity_65(ctx: TestContext):
    """Full state restore: 5 files, kill, verify."""
    files = ["lib.rs", "main.rs", "Cargo.toml", "tauri.conf.json", "package.json"]

    for filename in files:
        search_palette(ctx)
        wait_ms(ctx, 300)
        type_text(ctx, filename)
        wait_ms(ctx, 300)
        press_key(ctx, "Return")
        wait_ms(ctx, 500)

    screenshot(ctx, "integrity-65-before-kill")

    # Kill the app
    _sigkill_app(ctx)
    time.sleep(1)

    # Relaunch
    ctx.app.launch()
    ctx.app.wait_ready()

    screenshot(ctx, "integrity-65-after-relaunch")
    snapshot_performance(ctx)


def test_integrity_67(ctx: TestContext):
    """Concurrent writes: editor + external edit."""
    test_file = "/tmp/field-integrity-67.rs"
    with open(test_file, "w") as f:
        f.write("// integrity-67 test file\nfn main() {}\n")

    # Open the file in the editor
    search_palette(ctx)
    wait_ms(ctx, 300)
    type_text(ctx, test_file)
    wait_ms(ctx, 300)
    press_key(ctx, "Return")
    wait_ms(ctx, 800)

    # Make an edit in the editor
    type_text(ctx, "// editor edit\n")

    # External write to the same file
    with open(test_file, "a") as f:
        f.write("// external edit by test_integrity_67\n")

    # Wait for the app to detect the external change
    wait_ms(ctx, 5000)

    screenshot(ctx, "integrity-67-concurrent-write")
    assert_no_new_errors(ctx)

    # Cleanup
    try:
        os.remove(test_file)
    except FileNotFoundError:
        pass
