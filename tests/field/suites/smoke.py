"""Smoke test suite — Python overrides.

Functions named test_{id}(ctx) override YAML scenarios of the same ID.
"""

from __future__ import annotations

from lib.actions import (
    assert_no_new_errors,
    focus_window,
    press_key,
    screenshot,
    search_palette,
    snapshot_performance,
    type_text,
    wait_ms,
)
from lib.context import TestContext


def test_smoke_07(ctx: TestContext) -> None:
    """State persistence: open files, kill, relaunch, verify."""
    search_palette(ctx)
    wait_ms(ctx, 500)
    type_text(ctx, "tauri.ts")
    wait_ms(ctx, 500)
    press_key(ctx, "Return")
    wait_ms(ctx, 1500)
    screenshot(ctx, "smoke-07-state-before")

    ctx.app.kill()
    wait_ms(ctx, 2000)

    ctx.app.launch()
    assert ctx.app.wait_ready(60), "App did not become ready after relaunch"

    focus_window(ctx)
    wait_ms(ctx, 3000)
    screenshot(ctx, "smoke-07-state-after")
    snapshot_performance(ctx)
