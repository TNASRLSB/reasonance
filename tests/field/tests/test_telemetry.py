import os
import sys
import time
import tempfile
import pytest
from unittest.mock import MagicMock, patch

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from lib.telemetry import Telemetry


class TestTimeline:
    def test_timeline_mark(self):
        app = MagicMock()
        app._process = MagicMock()
        app._process.pid = 12345
        app._log_file = "/tmp/nonexistent.log"

        t = Telemetry(app)
        t.mark("test_event", key="value")

        tl = t.timeline()
        assert len(tl) == 1
        assert tl[0]["event"] == "test_event"
        assert tl[0]["data"]["key"] == "value"
        assert "timestamp" in tl[0]

    def test_timeline_returns_copy(self):
        app = MagicMock()
        app._log_file = "/tmp/nonexistent.log"

        t = Telemetry(app)
        t.mark("evt")

        tl = t.timeline()
        tl.clear()

        assert len(t.timeline()) == 1


class TestErrorsParsing:
    def test_errors_since(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".log", delete=False) as f:
            f.write("[2026-04-01][10:00:01][INFO][lib] starting\n")
            f.write("[2026-04-01][10:00:02][ERROR][lib::transport] Connection refused\n")
            f.write("[2026-04-01][10:00:03][INFO][lib] all good\n")
            log_path = f.name

        try:
            app = MagicMock()
            app._log_file = log_path

            t = Telemetry(app)
            errors = t.errors_since(0.0)

            assert len(errors) == 1
            assert "Connection refused" in errors[0]
        finally:
            os.unlink(log_path)

    def test_errors_since_empty_when_no_errors(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".log", delete=False) as f:
            f.write("[2026-04-01][10:00:01][INFO][lib] all good\n")
            log_path = f.name

        try:
            app = MagicMock()
            app._log_file = log_path

            t = Telemetry(app)
            assert t.errors_since(0.0) == []
        finally:
            os.unlink(log_path)


class TestRssMb:
    def test_rss_mb_returns_none_when_no_process(self):
        app = MagicMock()
        app._process = None
        app._log_file = "/tmp/nonexistent.log"

        t = Telemetry(app)
        result = t.rss_mb()

        assert result is None

    def test_rss_mb_reads_proc_status(self):
        fake_status = "Name:\treasonance\nVmRSS:\t  20480 kB\nVmPeak:\t 30000 kB\n"

        app = MagicMock()
        app._process = MagicMock()
        app._process.pid = 99999
        app._log_file = "/tmp/nonexistent.log"

        t = Telemetry(app)

        with patch("builtins.open", MagicMock(return_value=MagicMock(
            __enter__=lambda s, *a: MagicMock(read=lambda: fake_status),
            __exit__=MagicMock(return_value=False)
        ))):
            with patch("os.path.exists", return_value=True):
                # Simulate reading the proc file directly
                import io
                m = MagicMock()
                m.__enter__ = lambda s: io.StringIO(fake_status)
                m.__exit__ = MagicMock(return_value=False)
                with patch("builtins.open", return_value=m):
                    result = t.rss_mb()
                    # 20480 kB -> 20.0 MB
                    assert result == pytest.approx(20.0, abs=0.1)

    def test_rss_mb_returns_none_on_missing_proc_file(self):
        app = MagicMock()
        app._process = MagicMock()
        app._process.pid = 99999
        app._log_file = "/tmp/nonexistent.log"

        t = Telemetry(app)
        with patch("builtins.open", side_effect=FileNotFoundError):
            result = t.rss_mb()
            assert result is None


class TestSummary:
    def test_summary(self):
        app = MagicMock()
        app._process = None
        app._log_file = "/tmp/nonexistent.log"

        t = Telemetry(app)
        t.mark("start")
        t.mark("end")

        s = t.summary()

        assert "event_count" in s
        assert s["event_count"] == 2
        assert "rss_peak_mb" in s
        assert "rss_final_mb" in s
        assert "rss_samples" in s
        assert "errors" in s
        assert "warnings" in s

    def test_summary_rss_peak_from_samples(self):
        app = MagicMock()
        app._process = None
        app._log_file = "/tmp/nonexistent.log"

        t = Telemetry(app)
        # Manually inject RSS samples
        with t._lock:
            t._rss_samples = [10.0, 25.0, 15.0]

        s = t.summary()
        assert s["rss_peak_mb"] == 25.0
        assert s["rss_final_mb"] == 15.0

    def test_summary_rss_peak_none_when_no_samples(self):
        app = MagicMock()
        app._process = None
        app._log_file = "/tmp/nonexistent.log"

        t = Telemetry(app)
        s = t.summary()
        assert s["rss_peak_mb"] is None
        assert s["rss_final_mb"] is None


class TestParseErrors:
    def _make_telemetry(self):
        app = MagicMock()
        app._log_file = "/tmp/nonexistent.log"
        return Telemetry(app)

    def test_parse_errors_finds_error_marker(self):
        t = self._make_telemetry()
        lines = [
            "[2026-04-01][10:00:01][ERROR][lib] something broke",
            "[2026-04-01][10:00:02][INFO][lib] all good",
        ]
        errors = t._parse_errors(lines)
        assert len(errors) == 1
        assert "something broke" in errors[0]

    def test_parse_errors_finds_panicked(self):
        t = self._make_telemetry()
        lines = [
            "thread 'main' (12345) panicked at src/lib.rs:253:62:",
            "there is no reactor running",
        ]
        errors = t._parse_errors(lines)
        assert any("panicked" in e for e in errors)

    def test_parse_errors_finds_unhandled_rejection(self):
        t = self._make_telemetry()
        lines = [
            "10:23:05 [vite] (client) [Unhandled rejection] Unknown Error: [object Object]",
        ]
        errors = t._parse_errors(lines)
        assert len(errors) == 1
        assert "Unhandled rejection" in errors[0]

    def test_parse_errors_finds_uncaught(self):
        t = self._make_telemetry()
        lines = [
            "Uncaught TypeError: Cannot read property 'x' of undefined",
        ]
        errors = t._parse_errors(lines)
        assert len(errors) == 1

    def test_parse_errors_finds_fatal(self):
        t = self._make_telemetry()
        lines = [
            "fatal error: out of memory",
        ]
        errors = t._parse_errors(lines)
        assert len(errors) == 1

    def test_parse_errors_skips_warn_and_info(self):
        t = self._make_telemetry()
        lines = [
            "[2026-04-01][10:00:01][WARN][lib] low memory",
            "[2026-04-01][10:00:02][INFO][lib] normal",
        ]
        errors = t._parse_errors(lines)
        assert errors == []

    def test_parse_errors_finds_sigabrt(self):
        t = self._make_telemetry()
        lines = ["Process received SIGABRT signal"]
        errors = t._parse_errors(lines)
        assert len(errors) == 1


class TestParseWarnings:
    def _make_telemetry(self):
        app = MagicMock()
        app._log_file = "/tmp/nonexistent.log"
        return Telemetry(app)

    def test_parse_warnings_finds_warn_marker(self):
        t = self._make_telemetry()
        lines = [
            "[2026-04-01][10:00:01][WARN][lib] disk space low",
            "[2026-04-01][10:00:02][INFO][lib] nothing wrong",
        ]
        warnings = t._parse_warnings(lines)
        assert len(warnings) == 1
        assert "disk space low" in warnings[0]

    def test_parse_warnings_skips_error_lines(self):
        t = self._make_telemetry()
        lines = [
            "[2026-04-01][10:00:01][ERROR][lib] connection refused",
            "[2026-04-01][10:00:02][INFO][lib] ok",
        ]
        warnings = t._parse_warnings(lines)
        assert warnings == []

    def test_parse_warnings_multiple(self):
        t = self._make_telemetry()
        lines = [
            "[2026-04-01][10:00:01][WARN][lib] slow response",
            "[2026-04-01][10:00:02][WARN][lib] retry attempted",
            "[2026-04-01][10:00:03][INFO][lib] recovered",
        ]
        warnings = t._parse_warnings(lines)
        assert len(warnings) == 2
