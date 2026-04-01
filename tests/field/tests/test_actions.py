import os
import sys
from unittest.mock import MagicMock, patch, call

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

import lib.actions as _actions_mod
from lib.actions import ACTION_REGISTRY
from lib.context import TestContext


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_ctx(**kwargs) -> TestContext:
    """Build a minimal TestContext with mocked app and telemetry."""
    defaults = dict(
        app=MagicMock(),
        telemetry=MagicMock(),
        screenshot_dir="/tmp/test-screenshots",
        test_id="test_01",
        test_name="Action test",
        verbose=False,
        window_rect={"x": 100, "y": 50, "width": 1000, "height": 500},
    )
    defaults.update(kwargs)
    return TestContext(**defaults)


# ===========================================================================
# Input action tests
# ===========================================================================


class TestPressKey:
    def test_press_key(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.press_key(ctx, "ctrl+z")
            mock_input.key.assert_called_once_with("ctrl+z")

    def test_press_key_marks_telemetry(self):
        ctx = _make_ctx()
        with patch("lib.actions._input"):
            _actions_mod.press_key(ctx, "ctrl+s")
        ctx.telemetry.mark.assert_called_once_with(
            "action", data={"name": "press_key", "combo": "ctrl+s"}
        )


class TestTypeText:
    def test_type_text(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.type_text(ctx, "hello world")
            mock_input.type_text.assert_called_once_with("hello world", 50)

    def test_type_text_custom_delay(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.type_text(ctx, "fast", delay_ms=10)
            mock_input.type_text.assert_called_once_with("fast", 10)

    def test_type_text_marks_telemetry(self):
        ctx = _make_ctx()
        with patch("lib.actions._input"):
            _actions_mod.type_text(ctx, "hi")
        ctx.telemetry.mark.assert_called_once_with(
            "action", data={"name": "type_text", "text": "hi", "delay_ms": 50}
        )


class TestWaitMs:
    def test_wait_ms(self):
        ctx = _make_ctx()
        with patch("lib.actions.time") as mock_time:
            _actions_mod.wait_ms(ctx, 250)
            mock_time.sleep.assert_called_once_with(0.25)

    def test_wait_ms_zero(self):
        ctx = _make_ctx()
        with patch("lib.actions.time") as mock_time:
            _actions_mod.wait_ms(ctx, 0)
            mock_time.sleep.assert_called_once_with(0.0)

    def test_wait_ms_marks_telemetry(self):
        ctx = _make_ctx()
        with patch("lib.actions.time"):
            _actions_mod.wait_ms(ctx, 100)
        ctx.telemetry.mark.assert_called_once_with(
            "action", data={"name": "wait_ms", "ms": 100}
        )


# ===========================================================================
# Click action tests
# ===========================================================================


class TestClickRelative:
    """Window at (100, 50) size 1000x500."""

    def test_click_relative_center(self):
        ctx = _make_ctx()
        # x_pct=0.5 → 100 + 1000*0.5 = 600
        # y_pct=0.5 → 50 + 500*0.5 = 300
        with patch("lib.actions._input") as mock_input:
            _actions_mod.click_relative(ctx, 0.5, 0.5)
            mock_input.click.assert_called_once_with(600, 300)

    def test_click_relative_origin(self):
        ctx = _make_ctx()
        # x_pct=0.0 → 100 + 1000*0.0 = 100
        # y_pct=0.0 → 50 + 500*0.0 = 50
        with patch("lib.actions._input") as mock_input:
            _actions_mod.click_relative(ctx, 0.0, 0.0)
            mock_input.click.assert_called_once_with(100, 50)

    def test_click_relative_bottom_right(self):
        ctx = _make_ctx()
        # x_pct=1.0 → 100 + 1000*1.0 = 1100
        # y_pct=1.0 → 50 + 500*1.0 = 550
        with patch("lib.actions._input") as mock_input:
            _actions_mod.click_relative(ctx, 1.0, 1.0)
            mock_input.click.assert_called_once_with(1100, 550)

    def test_click_relative_marks_telemetry(self):
        ctx = _make_ctx()
        with patch("lib.actions._input"):
            _actions_mod.click_relative(ctx, 0.5, 0.5)
        ctx.telemetry.mark.assert_called_once_with(
            "action", data={"name": "click_relative", "x_pct": 0.5, "y_pct": 0.5}
        )


# ===========================================================================
# Assertion action tests
# ===========================================================================


class TestAssertNoErrors:
    def test_assert_no_errors_passes(self):
        ctx = _make_ctx()
        ctx.app.get_errors.return_value = []
        # Should not raise
        _actions_mod.assert_no_errors(ctx)

    def test_assert_no_errors_fails(self):
        ctx = _make_ctx()
        ctx.app.get_errors.return_value = ["[ERROR] something went wrong"]
        try:
            _actions_mod.assert_no_errors(ctx)
            assert False, "Expected AssertionError"
        except AssertionError as exc:
            assert "errors" in str(exc).lower() or "Expected" in str(exc)

    def test_assert_no_errors_fails_with_multiple(self):
        ctx = _make_ctx()
        ctx.app.get_errors.return_value = ["err1", "err2", "err3"]
        try:
            _actions_mod.assert_no_errors(ctx)
            assert False, "Expected AssertionError"
        except AssertionError:
            pass


class TestAssertNoNewErrors:
    def test_assert_no_new_errors_passes_when_counts_match(self):
        ctx = _make_ctx(errors_at_start=2)
        ctx.app.get_errors.return_value = ["err1", "err2"]
        # new_errors() returns get_errors()[2:] which is empty
        _actions_mod.assert_no_new_errors(ctx)

    def test_assert_no_new_errors_fails_when_new_errors_exist(self):
        ctx = _make_ctx(errors_at_start=1)
        ctx.app.get_errors.return_value = ["err1", "err2"]
        try:
            _actions_mod.assert_no_new_errors(ctx)
            assert False, "Expected AssertionError"
        except AssertionError:
            pass

    def test_assert_no_new_errors_empty_log_passes(self):
        ctx = _make_ctx(errors_at_start=0)
        ctx.app.get_errors.return_value = []
        _actions_mod.assert_no_new_errors(ctx)


# ===========================================================================
# Registry completeness test
# ===========================================================================


class TestActionRegistry:
    REQUIRED_ACTIONS = [
        "press_key",
        "type_text",
        "wait_ms",
        "click_relative",
        "assert_no_errors",
        "screenshot",
        "checkpoint",
        "wait_ready",
        "open_folder",
        "toolbar_settings",
        "toolbar_analytics",
        "search_palette",
        "save_file",
        "close_file",
        "scroll_down",
        "scroll_up",
        "focus_window",
        "maximize_window",
        "snapshot_performance",
        "wait_for_log",
        "wait_stable",
    ]

    def test_registry_contains_core_actions(self):
        for name in self.REQUIRED_ACTIONS:
            assert name in ACTION_REGISTRY, f"ACTION_REGISTRY missing: {name!r}"

    def test_registry_has_at_least_35_actions(self):
        assert len(ACTION_REGISTRY) >= 35, (
            f"Expected >= 35 actions, found {len(ACTION_REGISTRY)}: {sorted(ACTION_REGISTRY.keys())}"
        )

    def test_registry_values_are_callable(self):
        for name, fn in ACTION_REGISTRY.items():
            assert callable(fn), f"Registry entry {name!r} is not callable"


# ===========================================================================
# Additional coverage tests
# ===========================================================================


class TestScrollActions:
    def test_scroll_down(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.scroll_down(ctx, 5)
            mock_input.scroll.assert_called_once_with("down", 5)

    def test_scroll_up(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.scroll_up(ctx, 2)
            mock_input.scroll.assert_called_once_with("up", 2)


class TestWindowActions:
    def test_focus_window(self):
        ctx = _make_ctx()
        with patch("lib.actions._window") as mock_win:
            _actions_mod.focus_window(ctx)
            mock_win.focus.assert_called_once_with("reasonance")

    def test_maximize_window(self):
        ctx = _make_ctx()
        with patch("lib.actions._window") as mock_win:
            _actions_mod.maximize_window(ctx)
            mock_win.maximize.assert_called_once_with("reasonance")

    def test_get_window_rect_updates_ctx(self):
        ctx = _make_ctx()
        new_rect = {"x": 0, "y": 0, "width": 1280, "height": 800}
        with patch("lib.actions._window") as mock_win:
            mock_win.get_geometry.return_value = new_rect
            result = _actions_mod.get_window_rect(ctx)
        assert result == new_rect
        assert ctx.window_rect == new_rect


class TestScreenshotActions:
    def test_screenshot(self):
        ctx = _make_ctx()
        with patch("lib.actions._screen") as mock_screen:
            mock_screen.screenshot.return_value = "/tmp/test.png"
            path = _actions_mod.screenshot(ctx, "my_shot")
        mock_screen.screenshot.assert_called_once_with("my_shot", "/tmp/test-screenshots")
        assert path == "/tmp/test.png"

    def test_checkpoint(self):
        ctx = _make_ctx()
        with patch("lib.actions._screen") as mock_screen:
            mock_screen.screenshot.return_value = "/tmp/chk.png"
            path = _actions_mod.checkpoint(ctx, "step1")
        mock_screen.screenshot.assert_called_once_with("step1", "/tmp/test-screenshots")
        assert path == "/tmp/chk.png"

    def test_screenshot_active(self):
        ctx = _make_ctx()
        with patch("lib.actions._screen") as mock_screen:
            mock_screen.screenshot_active.return_value = "/tmp/active.png"
            path = _actions_mod.screenshot_active(ctx, "active_shot")
        mock_screen.screenshot_active.assert_called_once_with("active_shot", "/tmp/test-screenshots")


class TestNavigationActions:
    def test_search_palette(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.search_palette(ctx)
            mock_input.key.assert_called_once_with("ctrl+p")

    def test_save_file(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.save_file(ctx)
            mock_input.key.assert_called_once_with("ctrl+s")

    def test_close_file(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.close_file(ctx)
            mock_input.key.assert_called_once_with("ctrl+w")

    def test_undo(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.undo(ctx)
            mock_input.key.assert_called_once_with("ctrl+z")

    def test_redo(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.redo(ctx)
            mock_input.key.assert_called_once_with("ctrl+shift+z")

    def test_find_in_file(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.find_in_file(ctx)
            mock_input.key.assert_called_once_with("ctrl+f")

    def test_find_in_files(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.find_in_files(ctx)
            mock_input.key.assert_called_once_with("ctrl+shift+f")

    def test_toggle_sidebar(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.toggle_sidebar(ctx)
            mock_input.key.assert_called_once_with("ctrl+b")

    def test_close_dialog(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.close_dialog(ctx)
            mock_input.key.assert_called_once_with("Escape")


class TestToolbarActions:
    def test_toolbar_settings(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.toolbar_settings(ctx)
            mock_input.key.assert_called_once_with("ctrl+comma")

    def test_toolbar_analytics(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.toolbar_analytics(ctx)
            mock_input.key.assert_called_once_with("ctrl+shift+a")

    def test_toolbar_memory_clicks_correct_position(self):
        ctx = _make_ctx()
        # window: x=100, y=50, w=1000, h=500
        # memory x_pct=0.845 → 100 + 1000*0.845 = 945
        # toolbar_y_pct=0.025 → 50 + 500*0.025 = 62
        with patch("lib.actions._input") as mock_input:
            _actions_mod.toolbar_memory(ctx)
            mock_input.click.assert_called_once_with(945, 62)

    def test_toolbar_hive_clicks_correct_position(self):
        ctx = _make_ctx()
        # hive x_pct=0.885 → 100 + 1000*0.885 = 985
        # toolbar_y_pct=0.025 → 50 + 500*0.025 = 62
        with patch("lib.actions._input") as mock_input:
            _actions_mod.toolbar_hive(ctx)
            mock_input.click.assert_called_once_with(985, 62)

    def test_toolbar_git_clicks_correct_position(self):
        ctx = _make_ctx()
        # git x_pct=0.760 → 100 + 1000*0.760 = 860
        # toolbar_y_pct=0.025 → 50 + 500*0.025 = 62
        with patch("lib.actions._input") as mock_input:
            _actions_mod.toolbar_git(ctx)
            mock_input.click.assert_called_once_with(860, 62)


class TestAssertLogContains:
    def test_passes_when_pattern_found(self):
        ctx = _make_ctx()
        ctx.app.logs.return_value = "some [INFO] setup complete log line"
        _actions_mod.assert_log_contains(ctx, "setup complete")

    def test_fails_when_pattern_absent(self):
        ctx = _make_ctx()
        ctx.app.logs.return_value = "nothing here"
        try:
            _actions_mod.assert_log_contains(ctx, "setup complete")
            assert False, "Expected AssertionError"
        except AssertionError:
            pass


class TestAssertLogNotContains:
    def test_passes_when_pattern_absent(self):
        ctx = _make_ctx()
        ctx.app.logs.return_value = "normal log output"
        _actions_mod.assert_log_not_contains(ctx, "[ERROR]")

    def test_fails_when_pattern_found(self):
        ctx = _make_ctx()
        ctx.app.logs.return_value = "[ERROR] something bad happened"
        try:
            _actions_mod.assert_log_not_contains(ctx, "[ERROR]")
            assert False, "Expected AssertionError"
        except AssertionError:
            pass


class TestSnapshotPerformance:
    def test_returns_rss_dict(self):
        ctx = _make_ctx()
        ctx.telemetry.rss_mb.return_value = 128.5
        result = _actions_mod.snapshot_performance(ctx)
        assert result == {"rss_mb": 128.5}

    def test_returns_none_rss_when_unavailable(self):
        ctx = _make_ctx()
        ctx.telemetry.rss_mb.return_value = None
        result = _actions_mod.snapshot_performance(ctx)
        assert result == {"rss_mb": None}


class TestOpenFolder:
    def test_open_folder_types_path_and_presses_enter(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input, \
             patch("lib.actions.time") as mock_time:
            _actions_mod.open_folder(ctx, "/home/user/project")
            # Should type the path
            mock_input.type_text.assert_called_once_with("/home/user/project")
            # Should press Return
            mock_input.key.assert_called_once_with("Return")
            # Should click twice (File menu + first item)
            assert mock_input.click.call_count == 2
            # Should sleep between clicks
            assert mock_time.sleep.call_count >= 2

    def test_open_folder_marks_telemetry(self):
        ctx = _make_ctx()
        with patch("lib.actions._input"), patch("lib.actions.time"):
            _actions_mod.open_folder(ctx, "/some/path")
        ctx.telemetry.mark.assert_called_once_with(
            "action", data={"name": "open_folder", "path": "/some/path"}
        )


class TestWaitActions:
    def test_wait_stable(self):
        ctx = _make_ctx()
        with patch("lib.actions.time") as mock_time:
            _actions_mod.wait_stable(ctx, 200)
            mock_time.sleep.assert_called_once_with(0.2)

    def test_wait_ready_delegates_to_app(self):
        ctx = _make_ctx()
        _actions_mod.wait_ready(ctx, timeout=30)
        ctx.app.wait_ready.assert_called_once_with(30)

    def test_wait_for_log_delegates_to_app(self):
        ctx = _make_ctx()
        _actions_mod.wait_for_log(ctx, "setup complete", timeout=15)
        ctx.app.wait_for_log.assert_called_once_with("setup complete", 15)


class TestRightClickAndDoubleClick:
    def test_right_click_relative(self):
        ctx = _make_ctx()
        # x_pct=0.1 → 100 + 1000*0.1 = 200
        # y_pct=0.2 → 50 + 500*0.2 = 150
        with patch("lib.actions._input") as mock_input:
            _actions_mod.right_click_relative(ctx, 0.1, 0.2)
            mock_input.right_click.assert_called_once_with(200, 150)

    def test_double_click_relative(self):
        ctx = _make_ctx()
        # x_pct=0.2 → 100 + 1000*0.2 = 300
        # y_pct=0.4 → 50 + 500*0.4 = 250
        with patch("lib.actions._input") as mock_input:
            _actions_mod.double_click_relative(ctx, 0.2, 0.4)
            mock_input.double_click.assert_called_once_with(300, 250)


class TestClickAt:
    def test_click_at(self):
        ctx = _make_ctx()
        with patch("lib.actions._input") as mock_input:
            _actions_mod.click_at(ctx, 500, 400)
            mock_input.click.assert_called_once_with(500, 400)
