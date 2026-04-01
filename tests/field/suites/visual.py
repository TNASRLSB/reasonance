"""Visual tests — Python overrides for screenshot-based checks."""

import os
import shutil

from lib.actions import (
    assert_no_new_errors,
    close_dialog,
    press_key,
    screenshot,
    toolbar_analytics,
    toolbar_git,
    toolbar_hive,
    toolbar_memory,
    toolbar_settings,
    wait_ms,
)
from lib.context import TestContext
from lib.window import _run_kwin_script


def test_visual_58(ctx: TestContext):
    """Baseline screenshots: capture every major view."""
    baseline_dir = os.path.join(ctx.screenshot_dir, "..", "baselines")
    baseline_dir = os.path.normpath(baseline_dir)
    os.makedirs(baseline_dir, exist_ok=True)

    # Editor — current view (no navigation needed)
    wait_ms(ctx, 800)
    path = screenshot(ctx, "editor")
    shutil.copy2(path, os.path.join(baseline_dir, "editor.png"))

    # Settings (toolbar_settings opens via Ctrl+,; close with Escape)
    toolbar_settings(ctx)
    wait_ms(ctx, 800)
    path = screenshot(ctx, "settings")
    shutil.copy2(path, os.path.join(baseline_dir, "settings.png"))
    close_dialog(ctx)

    # Analytics — toggle open, screenshot, toggle closed
    toolbar_analytics(ctx)
    wait_ms(ctx, 800)
    path = screenshot(ctx, "analytics")
    shutil.copy2(path, os.path.join(baseline_dir, "analytics.png"))
    toolbar_analytics(ctx)

    # Memory — toggle open, screenshot, toggle closed
    toolbar_memory(ctx)
    wait_ms(ctx, 800)
    path = screenshot(ctx, "memory")
    shutil.copy2(path, os.path.join(baseline_dir, "memory.png"))
    toolbar_memory(ctx)

    # Hive — toggle open, screenshot, toggle closed
    toolbar_hive(ctx)
    wait_ms(ctx, 800)
    path = screenshot(ctx, "hive")
    shutil.copy2(path, os.path.join(baseline_dir, "hive.png"))
    toolbar_hive(ctx)

    # Git — toggle open, screenshot, toggle closed
    toolbar_git(ctx)
    wait_ms(ctx, 800)
    path = screenshot(ctx, "git")
    shutil.copy2(path, os.path.join(baseline_dir, "git.png"))
    toolbar_git(ctx)

    assert_no_new_errors(ctx)


def test_visual_61(ctx: TestContext):
    """Responsive layout: resize to multiple sizes."""
    sizes = [
        ("small", 1024, 600),
        ("large", 1920, 1080),
        ("tiny", 800, 500),
        ("default", 1280, 800),
    ]

    for label, w, h in sizes:
        kwin_script = f"""
var clients = workspace.windowList();
for (var i = 0; i < clients.length; i++) {{
    if (clients[i].resourceClass === "reasonance") {{
        clients[i].setMaximize(false, false);
        clients[i].frameGeometry = Qt.rect(
            clients[i].frameGeometry.x,
            clients[i].frameGeometry.y,
            {w}, {h}
        );
        break;
    }}
}}
"""
        _run_kwin_script(kwin_script)
        wait_ms(ctx, 1000)
        screenshot(ctx, f"visual-61-{label}-{w}x{h}")

    assert_no_new_errors(ctx)
