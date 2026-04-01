"""Action registry — UI automation primitives for field tests.

Each action takes a TestContext as first arg and logs to telemetry.
Import aliases allow test mocking of underlying I/O modules.
"""

from __future__ import annotations

import os
import time as _time_module

from lib import input as _input
from lib import screen as _screen
from lib import window as _window
from lib.context import TestContext

# Module-level alias so tests can @patch("lib.actions.time")
time = _time_module

# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------

ACTION_REGISTRY: dict[str, callable] = {}


def _register(fn):
    """Decorator: add *fn* to ACTION_REGISTRY under its __name__."""
    ACTION_REGISTRY[fn.__name__] = fn
    return fn


# ---------------------------------------------------------------------------
# Toolbar layout constants  (% of window width/height, measured on 1280x800)
# ---------------------------------------------------------------------------

TOOLBAR_Y_PCT = 0.025
TOOLBAR_BUTTONS = {
    "git": 0.760,
    "analytics": 0.805,
    "memory": 0.845,
    "hive": 0.885,
    "settings": 0.920,
}


# ===========================================================================
# Input actions
# ===========================================================================


@_register
def press_key(ctx: TestContext, combo: str) -> None:
    """Press a key or key combination."""
    ctx.telemetry.mark("action", data={"name": "press_key", "combo": combo})
    _input.key(combo)


@_register
def type_text(ctx: TestContext, text: str, delay_ms: int = 50) -> None:
    """Type a string of text."""
    ctx.telemetry.mark("action", data={"name": "type_text", "text": text, "delay_ms": delay_ms})
    _input.type_text(text, delay_ms)


@_register
def click_relative(ctx: TestContext, x_pct: float, y_pct: float) -> None:
    """Click at window-relative percentage coordinates."""
    ctx.telemetry.mark("action", data={"name": "click_relative", "x_pct": x_pct, "y_pct": y_pct})
    ax, ay = ctx.to_absolute(x_pct, y_pct)
    _input.click(ax, ay)


@_register
def click_at(ctx: TestContext, x: int, y: int) -> None:
    """Click at absolute screen pixel coordinates."""
    ctx.telemetry.mark("action", data={"name": "click_at", "x": x, "y": y})
    _input.click(x, y)


@_register
def right_click_relative(ctx: TestContext, x_pct: float, y_pct: float) -> None:
    """Right-click at window-relative percentage coordinates."""
    ctx.telemetry.mark("action", data={"name": "right_click_relative", "x_pct": x_pct, "y_pct": y_pct})
    ax, ay = ctx.to_absolute(x_pct, y_pct)
    _input.right_click(ax, ay)


@_register
def double_click_relative(ctx: TestContext, x_pct: float, y_pct: float) -> None:
    """Double-click at window-relative percentage coordinates."""
    ctx.telemetry.mark("action", data={"name": "double_click_relative", "x_pct": x_pct, "y_pct": y_pct})
    ax, ay = ctx.to_absolute(x_pct, y_pct)
    _input.double_click(ax, ay)


@_register
def scroll_down(ctx: TestContext, amount: int = 3) -> None:
    """Scroll down."""
    ctx.telemetry.mark("action", data={"name": "scroll_down", "amount": amount})
    _input.scroll("down", amount)


@_register
def scroll_up(ctx: TestContext, amount: int = 3) -> None:
    """Scroll up."""
    ctx.telemetry.mark("action", data={"name": "scroll_up", "amount": amount})
    _input.scroll("up", amount)


# ===========================================================================
# Wait actions
# ===========================================================================


@_register
def wait_ms(ctx: TestContext, ms: int) -> None:
    """Pause for *ms* milliseconds."""
    ctx.telemetry.mark("action", data={"name": "wait_ms", "ms": ms})
    time.sleep(ms / 1000.0)


@_register
def wait_ready(ctx: TestContext, timeout: int = 120) -> None:
    """Block until the app reports ready."""
    ctx.telemetry.mark("action", data={"name": "wait_ready", "timeout": timeout})
    ctx.app.wait_ready(timeout)


@_register
def wait_for_log(ctx: TestContext, pattern: str, timeout: int = 10) -> None:
    """Block until *pattern* appears in app logs."""
    ctx.telemetry.mark("action", data={"name": "wait_for_log", "pattern": pattern, "timeout": timeout})
    ctx.app.wait_for_log(pattern, timeout)


@_register
def wait_stable(ctx: TestContext, ms: int = 500) -> None:
    """Wait for UI to settle (simple pause)."""
    ctx.telemetry.mark("action", data={"name": "wait_stable", "ms": ms})
    time.sleep(ms / 1000.0)


# ===========================================================================
# Window actions
# ===========================================================================


@_register
def focus_window(ctx: TestContext, resource_class: str = "reasonance") -> None:
    """Bring the app window to focus."""
    ctx.telemetry.mark("action", data={"name": "focus_window", "resource_class": resource_class})
    _window.focus(resource_class)


@_register
def maximize_window(ctx: TestContext, resource_class: str = "reasonance") -> None:
    """Maximize the app window."""
    ctx.telemetry.mark("action", data={"name": "maximize_window", "resource_class": resource_class})
    _window.maximize(resource_class)


@_register
def get_window_rect(ctx: TestContext, resource_class: str = "reasonance") -> dict:
    """Return the window geometry dict and update ctx.window_rect."""
    ctx.telemetry.mark("action", data={"name": "get_window_rect", "resource_class": resource_class})
    rect = _window.get_geometry(resource_class)
    ctx.window_rect = rect
    return rect


# ===========================================================================
# Screenshot actions
# ===========================================================================


@_register
def screenshot(ctx: TestContext, name: str = "screenshot") -> str:
    """Capture a full-screen screenshot."""
    ctx.telemetry.mark("action", data={"name": "screenshot", "label": name})
    return _screen.screenshot(name, ctx.screenshot_dir)


@_register
def screenshot_active(ctx: TestContext, name: str = "active") -> str:
    """Capture the active window only."""
    ctx.telemetry.mark("action", data={"name": "screenshot_active", "label": name})
    return _screen.screenshot_active(name, ctx.screenshot_dir)


@_register
def checkpoint(ctx: TestContext, label: str = "checkpoint") -> str:
    """Named screenshot checkpoint — full screen."""
    ctx.telemetry.mark("action", data={"name": "checkpoint", "label": label})
    return _screen.screenshot(label, ctx.screenshot_dir)


# ===========================================================================
# Performance action
# ===========================================================================


@_register
def snapshot_performance(ctx: TestContext) -> dict:
    """Record current RSS and return a snapshot dict."""
    ctx.telemetry.mark("action", data={"name": "snapshot_performance"})
    rss = ctx.telemetry.rss_mb()
    return {"rss_mb": rss}


# ===========================================================================
# Navigation actions
# ===========================================================================


@_register
def open_folder(ctx: TestContext, path: str) -> None:
    """Open a folder via the File menu (click File → first item, type path, Enter)."""
    ctx.telemetry.mark("action", data={"name": "open_folder", "path": path})
    # File menu lives at ~5% x, TOOLBAR_Y_PCT y
    ax, ay = ctx.to_absolute(0.05, TOOLBAR_Y_PCT)
    _input.click(ax, ay)
    time.sleep(0.3)
    # First menu item (Open Folder…)
    ax2, ay2 = ctx.to_absolute(0.05, 0.08)
    _input.click(ax2, ay2)
    time.sleep(0.3)
    _input.type_text(path)
    _input.key("Return")


@_register
def search_palette(ctx: TestContext) -> None:
    """Open command/search palette (Ctrl+P)."""
    ctx.telemetry.mark("action", data={"name": "search_palette"})
    _input.key("ctrl+p")


@_register
def save_file(ctx: TestContext) -> None:
    """Save current file (Ctrl+S)."""
    ctx.telemetry.mark("action", data={"name": "save_file"})
    _input.key("ctrl+s")


@_register
def close_file(ctx: TestContext) -> None:
    """Close current file (Ctrl+W)."""
    ctx.telemetry.mark("action", data={"name": "close_file"})
    _input.key("ctrl+w")


@_register
def undo(ctx: TestContext) -> None:
    """Undo last action (Ctrl+Z)."""
    ctx.telemetry.mark("action", data={"name": "undo"})
    _input.key("ctrl+z")


@_register
def redo(ctx: TestContext) -> None:
    """Redo last undone action (Ctrl+Shift+Z)."""
    ctx.telemetry.mark("action", data={"name": "redo"})
    _input.key("ctrl+shift+z")


@_register
def find_in_file(ctx: TestContext) -> None:
    """Open in-file search (Ctrl+F)."""
    ctx.telemetry.mark("action", data={"name": "find_in_file"})
    _input.key("ctrl+f")


@_register
def find_in_files(ctx: TestContext) -> None:
    """Open project-wide search (Ctrl+Shift+F)."""
    ctx.telemetry.mark("action", data={"name": "find_in_files"})
    _input.key("ctrl+shift+f")


@_register
def toggle_sidebar(ctx: TestContext) -> None:
    """Toggle sidebar visibility (Ctrl+B)."""
    ctx.telemetry.mark("action", data={"name": "toggle_sidebar"})
    _input.key("ctrl+b")


@_register
def close_dialog(ctx: TestContext) -> None:
    """Dismiss an open dialog (Escape)."""
    ctx.telemetry.mark("action", data={"name": "close_dialog"})
    _input.key("Escape")


# ===========================================================================
# Toolbar actions
# ===========================================================================


@_register
def toolbar_settings(ctx: TestContext) -> None:
    """Open settings (Ctrl+,)."""
    ctx.telemetry.mark("action", data={"name": "toolbar_settings"})
    _input.key("ctrl+comma")


@_register
def toolbar_analytics(ctx: TestContext) -> None:
    """Open analytics panel (Ctrl+Shift+A)."""
    ctx.telemetry.mark("action", data={"name": "toolbar_analytics"})
    _input.key("ctrl+shift+a")


@_register
def toolbar_memory(ctx: TestContext) -> None:
    """Click the memory toolbar button."""
    ctx.telemetry.mark("action", data={"name": "toolbar_memory"})
    ax, ay = ctx.to_absolute(TOOLBAR_BUTTONS["memory"], TOOLBAR_Y_PCT)
    _input.click(ax, ay)


@_register
def toolbar_hive(ctx: TestContext) -> None:
    """Click the hive toolbar button."""
    ctx.telemetry.mark("action", data={"name": "toolbar_hive"})
    ax, ay = ctx.to_absolute(TOOLBAR_BUTTONS["hive"], TOOLBAR_Y_PCT)
    _input.click(ax, ay)


@_register
def toolbar_git(ctx: TestContext) -> None:
    """Click the git toolbar button."""
    ctx.telemetry.mark("action", data={"name": "toolbar_git"})
    ax, ay = ctx.to_absolute(TOOLBAR_BUTTONS["git"], TOOLBAR_Y_PCT)
    _input.click(ax, ay)


# ===========================================================================
# Assertion actions
# ===========================================================================


@_register
def assert_no_errors(ctx: TestContext) -> None:
    """Assert there are no errors in the app log."""
    ctx.telemetry.mark("action", data={"name": "assert_no_errors"})
    errors = ctx.app.get_errors()
    if errors:
        raise AssertionError(f"Expected no errors, found {len(errors)}: {errors[:3]}")


@_register
def assert_no_new_errors(ctx: TestContext) -> None:
    """Assert no errors have appeared since errors_at_start."""
    ctx.telemetry.mark("action", data={"name": "assert_no_new_errors"})
    new_errors = ctx.new_errors()
    if new_errors:
        raise AssertionError(
            f"Expected no new errors since checkpoint, found {len(new_errors)}: {new_errors[:3]}"
        )


@_register
def assert_screenshot_matches(ctx: TestContext, name: str, baseline_dir: str, threshold: float = 0.95) -> None:
    """Compare a fresh screenshot against a baseline image."""
    ctx.telemetry.mark("action", data={"name": "assert_screenshot_matches", "baseline": name})
    current_path = _screen.screenshot(name, ctx.screenshot_dir)
    baseline_path = os.path.join(baseline_dir, f"{name}.png")
    if not os.path.exists(baseline_path):
        raise AssertionError(f"Baseline image not found: {baseline_path}")
    similarity = _screen.compare_images(current_path, baseline_path)
    if similarity < threshold:
        raise AssertionError(
            f"Screenshot '{name}' similarity {similarity:.3f} < threshold {threshold:.3f}"
        )


@_register
def assert_log_contains(ctx: TestContext, pattern: str) -> None:
    """Assert *pattern* appears in the app log."""
    ctx.telemetry.mark("action", data={"name": "assert_log_contains", "pattern": pattern})
    logs = ctx.app.logs()
    if pattern not in logs:
        raise AssertionError(f"Expected pattern {pattern!r} not found in app log")


@_register
def assert_log_not_contains(ctx: TestContext, pattern: str) -> None:
    """Assert *pattern* does not appear in the app log."""
    ctx.telemetry.mark("action", data={"name": "assert_log_not_contains", "pattern": pattern})
    logs = ctx.app.logs()
    if pattern in logs:
        raise AssertionError(f"Unexpected pattern {pattern!r} found in app log")
