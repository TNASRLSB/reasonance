"""Telemetry collector — timeline, RSS polling, log parsing."""

import threading
import time
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from lib.app import ReasonanceApp


class Telemetry:
    def __init__(self, app: "ReasonanceApp"):
        self._app = app
        self._timeline: list[dict] = []
        self._rss_samples: list[float] = []
        self._lock = threading.Lock()
        self._poll_thread: threading.Thread | None = None
        self._stop_event = threading.Event()

    # ------------------------------------------------------------------
    # Timeline
    # ------------------------------------------------------------------

    def mark(self, event: str, **kwargs) -> None:
        """Append a named event to the timeline."""
        entry = {
            "event": event,
            "timestamp": time.time(),
            "data": kwargs,
        }
        with self._lock:
            self._timeline.append(entry)

    def timeline(self) -> list[dict]:
        """Return a snapshot copy of the timeline."""
        with self._lock:
            return list(self._timeline)

    # ------------------------------------------------------------------
    # Background RSS polling
    # ------------------------------------------------------------------

    def start(self, interval: float = 2.0) -> None:
        """Start a daemon thread that polls RSS every *interval* seconds."""
        self._stop_event.clear()

        def _poll():
            while not self._stop_event.wait(interval):
                sample = self.rss_mb()
                if sample is not None:
                    with self._lock:
                        self._rss_samples.append(sample)

        self._poll_thread = threading.Thread(target=_poll, daemon=True)
        self._poll_thread.start()

    def stop(self) -> dict:
        """Stop RSS polling and return the final summary."""
        self._stop_event.set()
        if self._poll_thread is not None:
            self._poll_thread.join(timeout=5)
            self._poll_thread = None
        return self.summary()

    # ------------------------------------------------------------------
    # RSS reading
    # ------------------------------------------------------------------

    def rss_mb(self) -> float | None:
        """Read VmRSS from /proc/{pid}/status and return MB, or None."""
        if self._app._process is None:
            return None
        pid = self._app._process.pid
        proc_path = f"/proc/{pid}/status"
        try:
            with open(proc_path) as fh:
                for line in fh:
                    if line.startswith("VmRSS:"):
                        # "VmRSS:    20480 kB"
                        kb = int(line.split()[1])
                        return kb / 1024.0
        except (FileNotFoundError, OSError, ValueError):
            return None
        return None

    # ------------------------------------------------------------------
    # Log parsing helpers
    # ------------------------------------------------------------------

    def errors_since(self, t: float) -> list[str]:
        """Return error lines from the log written after timestamp *t*."""
        lines = self._read_log_lines()
        return self._parse_errors(lines)

    def warnings_since(self, t: float) -> list[str]:
        """Return warning lines from the log written after timestamp *t*."""
        lines = self._read_log_lines()
        return self._parse_warnings(lines)

    def _read_log_lines(self) -> list[str]:
        """Read all lines from the app log file."""
        try:
            with open(self._app._log_file) as fh:
                return fh.readlines()
        except (FileNotFoundError, OSError):
            return []

    def _parse_errors(self, lines: list[str]) -> list[str]:
        """Filter lines that contain error-level markers."""
        error_markers = [
            "[ERROR]",
            "panicked",
            "SIGABRT",
            "Unhandled rejection",
            "Uncaught",
            "fatal",
        ]
        result = []
        for line in lines:
            stripped = line.strip()
            if any(marker in stripped for marker in error_markers):
                result.append(stripped)
        return result

    def _parse_warnings(self, lines: list[str]) -> list[str]:
        """Filter lines that contain warning-level markers."""
        result = []
        for line in lines:
            stripped = line.strip()
            if "[WARN]" in stripped:
                result.append(stripped)
        return result

    # ------------------------------------------------------------------
    # Summary
    # ------------------------------------------------------------------

    def summary(self) -> dict:
        """Return a summary dict of all collected telemetry."""
        with self._lock:
            tl_copy = list(self._timeline)
            samples_copy = list(self._rss_samples)

        rss_peak = max(samples_copy) if samples_copy else None
        rss_final = samples_copy[-1] if samples_copy else None

        lines = self._read_log_lines()
        return {
            "event_count": len(tl_copy),
            "rss_peak_mb": rss_peak,
            "rss_final_mb": rss_final,
            "rss_samples": samples_copy,
            "errors": self._parse_errors(lines),
            "warnings": self._parse_warnings(lines),
        }
