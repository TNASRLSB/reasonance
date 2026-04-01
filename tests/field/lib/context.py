"""TestContext dataclass — passed to every action and test function."""

from __future__ import annotations

import time
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from lib.app import ReasonanceApp
    from lib.telemetry import Telemetry


@dataclass
class TestContext:
    app: ReasonanceApp
    telemetry: Telemetry
    screenshot_dir: str
    test_id: str
    test_name: str
    verbose: bool
    window_rect: dict  # {x, y, width, height}
    start_time: float = field(default_factory=time.time)
    errors_at_start: int = 0

    def to_absolute(self, x_pct: float, y_pct: float) -> tuple[int, int]:
        """Convert window-relative percentage coordinates to absolute screen pixels."""
        x = int(self.window_rect["x"] + self.window_rect["width"] * x_pct)
        y = int(self.window_rect["y"] + self.window_rect["height"] * y_pct)
        return x, y

    def new_errors(self) -> list[str]:
        """Return errors logged after the current errors_at_start checkpoint."""
        all_errors = self.app.get_errors()
        return all_errors[self.errors_at_start:]

    def mark_error_checkpoint(self) -> None:
        """Reset errors_at_start to the current error count."""
        self.errors_at_start = len(self.app.get_errors())
