import os
import sys
from unittest.mock import MagicMock

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from lib.context import TestContext


class TestTestContext:
    def test_create_context(self):
        app = MagicMock()
        telemetry = MagicMock()
        ctx = TestContext(
            app=app,
            telemetry=telemetry,
            screenshot_dir="/tmp/screenshots",
            test_id="smoke_01",
            test_name="App startup",
            verbose=True,
            window_rect={"x": 0, "y": 0, "width": 1920, "height": 1080},
        )
        assert ctx.app is app
        assert ctx.telemetry is telemetry
        assert ctx.screenshot_dir == "/tmp/screenshots"
        assert ctx.test_id == "smoke_01"
        assert ctx.test_name == "App startup"
        assert ctx.verbose is True
        assert ctx.window_rect == {"x": 0, "y": 0, "width": 1920, "height": 1080}
        assert ctx.errors_at_start == 0
        assert ctx.start_time > 0

    def test_context_relative_coords(self):
        app = MagicMock()
        telemetry = MagicMock()
        ctx = TestContext(
            app=app,
            telemetry=telemetry,
            screenshot_dir="/tmp/screenshots",
            test_id="smoke_01",
            test_name="App startup",
            verbose=False,
            window_rect={"x": 100, "y": 50, "width": 1280, "height": 800},
        )
        # 50% across a 1280-wide window starting at x=100 → 100 + 1280*0.5 = 740
        # 50% down an 800-tall window starting at y=50  → 50 + 800*0.5 = 450
        x, y = ctx.to_absolute(0.5, 0.5)
        assert x == 740
        assert y == 450

    def test_context_error_checkpoint(self):
        app = MagicMock()
        app.get_errors.return_value = ["err1", "err2", "err3"]
        telemetry = MagicMock()

        ctx = TestContext(
            app=app,
            telemetry=telemetry,
            screenshot_dir="/tmp/screenshots",
            test_id="smoke_01",
            test_name="App startup",
            verbose=False,
            window_rect={"x": 0, "y": 0, "width": 1920, "height": 1080},
            errors_at_start=2,
        )
        # Only errors beyond index 2 should be returned
        errors = ctx.new_errors()
        assert errors == ["err3"]
