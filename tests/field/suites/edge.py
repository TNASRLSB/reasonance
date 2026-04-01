"""Edge case test suite — Python overrides.

Functions named test_{id}(ctx) override YAML scenarios of the same ID.
"""

from __future__ import annotations

import os
import shutil
import tempfile

from lib.actions import (
    assert_no_new_errors,
    close_file,
    focus_window,
    open_folder,
    press_key,
    screenshot,
    search_palette,
    type_text,
    wait_ms,
)
from lib.context import TestContext

_LLMS_TOML = os.path.expanduser("~/.config/reasonance/llms.toml")


def test_edge_35(ctx: TestContext) -> None:
    """Unicode and emoji."""
    unicode_file = "/tmp/reasonance-unicode-test.txt"
    try:
        with open(unicode_file, "w", encoding="utf-8") as fh:
            fh.write("# Unicode test\n")
            fh.write("CJK: \u4e2d\u6587\u6f22\u5b57\n")
            fh.write("Emoji: \U0001f600\U0001f4bb\U0001f527\n")
            fh.write("RTL: \u0645\u0631\u062d\u0628\u0627 \u0639\u0627\u0644\u0645\n")
            fh.write("Mixed: caf\u00e9 na\u00efve r\u00e9sum\u00e9\n")

        search_palette(ctx)
        wait_ms(ctx, 500)
        type_text(ctx, unicode_file)
        wait_ms(ctx, 500)
        press_key(ctx, "Return")
        wait_ms(ctx, 1500)
        screenshot(ctx, "edge-35-unicode")
        assert_no_new_errors(ctx)
    finally:
        if os.path.exists(unicode_file):
            os.unlink(unicode_file)


def test_edge_37(ctx: TestContext) -> None:
    """Empty project."""
    empty_dir = tempfile.mkdtemp(prefix="reasonance-empty-")
    try:
        open_folder(ctx, empty_dir)
        wait_ms(ctx, 3000)
        screenshot(ctx, "edge-37-empty-project")

        # Re-open the main REASONANCE project
        open_folder(ctx, "/home/uh1/VIBEPROJECTS/REASONANCE")
        wait_ms(ctx, 3000)
        assert_no_new_errors(ctx)
    finally:
        if os.path.isdir(empty_dir):
            shutil.rmtree(empty_dir, ignore_errors=True)


def test_edge_40(ctx: TestContext) -> None:
    """External file modification."""
    test_file = "/tmp/reasonance-external-mod-test.txt"
    try:
        with open(test_file, "w") as fh:
            fh.write("initial content\n")

        # Open the file in the editor
        search_palette(ctx)
        wait_ms(ctx, 500)
        type_text(ctx, test_file)
        wait_ms(ctx, 500)
        press_key(ctx, "Return")
        wait_ms(ctx, 1500)

        # Modify the file externally while it is open
        with open(test_file, "a") as fh:
            fh.write("EXTERNAL CHANGE\n")

        # Wait for the fs_watcher to detect the change
        wait_ms(ctx, 5000)
        screenshot(ctx, "edge-40-external-mod")
        assert_no_new_errors(ctx)
    finally:
        if os.path.exists(test_file):
            os.unlink(test_file)


def test_edge_41(ctx: TestContext) -> None:
    """Corrupt config."""
    backup_path = _LLMS_TOML + ".bak"
    config_dir = os.path.dirname(_LLMS_TOML)

    # Read existing config if present
    original_content: str | None = None
    if os.path.exists(_LLMS_TOML):
        with open(_LLMS_TOML, "r") as fh:
            original_content = fh.read()
        shutil.copy2(_LLMS_TOML, backup_path)

    try:
        # Write corrupt TOML
        os.makedirs(config_dir, exist_ok=True)
        with open(_LLMS_TOML, "w") as fh:
            fh.write("NOT_VALID_TOML = [[[\n")

        ctx.app.kill()
        wait_ms(ctx, 2000)

        ctx.app.launch()
        started = ctx.app.wait_ready(60)
        assert started, "App failed to start with corrupt llms.toml"

        focus_window(ctx)
        wait_ms(ctx, 2000)
        screenshot(ctx, "edge-41-corrupt-config")
    finally:
        # Restore original config
        if original_content is not None:
            with open(_LLMS_TOML, "w") as fh:
                fh.write(original_content)
            if os.path.exists(backup_path):
                os.unlink(backup_path)
        elif os.path.exists(_LLMS_TOML):
            os.unlink(_LLMS_TOML)
