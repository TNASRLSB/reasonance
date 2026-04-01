"""E2E test suite — Python overrides.

Functions named test_{id}(ctx) override YAML scenarios of the same ID.
"""

from __future__ import annotations

from lib.actions import (
    assert_no_new_errors,
    checkpoint,
    close_file,
    focus_window,
    press_key,
    screenshot,
    search_palette,
    snapshot_performance,
    type_text,
    undo,
    wait_ms,
)
from lib.context import TestContext


def test_e2e_09(ctx: TestContext) -> None:
    """Editor full workflow: open, edit, undo, multi-tab, close."""
    # Open lib.rs
    search_palette(ctx)
    wait_ms(ctx, 500)
    type_text(ctx, "lib.rs")
    wait_ms(ctx, 500)
    press_key(ctx, "Return")
    wait_ms(ctx, 1500)

    # Make an edit then undo
    type_text(ctx, "// field test edit")
    wait_ms(ctx, 300)
    undo(ctx)
    wait_ms(ctx, 300)

    # Open a second file in a new tab
    search_palette(ctx)
    wait_ms(ctx, 500)
    type_text(ctx, "Cargo.toml")
    wait_ms(ctx, 500)
    press_key(ctx, "Return")
    wait_ms(ctx, 1500)

    # Close the current tab
    close_file(ctx)
    wait_ms(ctx, 500)

    assert_no_new_errors(ctx)


def test_e2e_11(ctx: TestContext) -> None:
    """Terminal PTY: open, type command, verify."""
    press_key(ctx, "ctrl+grave")
    wait_ms(ctx, 2000)
    type_text(ctx, "echo FIELD_TEST_OK")
    wait_ms(ctx, 300)
    press_key(ctx, "Return")
    wait_ms(ctx, 1000)
    screenshot(ctx, "e2e-11-terminal")
    assert_no_new_errors(ctx)


def test_e2e_22(ctx: TestContext) -> None:
    """PTY resilience: verify terminal works."""
    press_key(ctx, "ctrl+grave")
    wait_ms(ctx, 2000)
    type_text(ctx, "echo $$")
    wait_ms(ctx, 300)
    press_key(ctx, "Return")
    wait_ms(ctx, 1000)
    checkpoint(ctx, "e2e-22-pty-resilience")
    assert_no_new_errors(ctx)


def test_e2e_25(ctx: TestContext) -> None:
    """Full state persistence: open 3 files, kill, verify."""
    files = ["lib.rs", "Cargo.toml", "tauri.conf.json"]
    for filename in files:
        search_palette(ctx)
        wait_ms(ctx, 500)
        type_text(ctx, filename)
        wait_ms(ctx, 500)
        press_key(ctx, "Return")
        wait_ms(ctx, 1500)

    screenshot(ctx, "e2e-25-state-before-kill")

    ctx.app.kill()
    wait_ms(ctx, 2000)

    ctx.app.launch()
    assert ctx.app.wait_ready(60), "App did not become ready after relaunch"

    focus_window(ctx)
    wait_ms(ctx, 3000)
    screenshot(ctx, "e2e-25-state-after-relaunch")
    snapshot_performance(ctx)
