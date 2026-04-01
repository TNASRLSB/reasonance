import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from lib.app import ReasonanceApp


SAMPLE_LOGS = """
[2026-04-01][10:23:02][INFO][reasonance_lib] Reasonance setup starting
[2026-04-01][10:23:02][INFO][reasonance_lib::capability] Capabilities negotiated for provider='claude'
[2026-04-01][10:23:02][INFO][reasonance_lib] Reasonance setup complete — all systems wired
10:23:05 [vite] (client) [Unhandled rejection] Unknown Error: [object Object]
[2026-04-01][10:23:10][ERROR][reasonance_lib::transport] Connection refused
thread 'main' (12345) panicked at src/lib.rs:253:62:
there is no reactor running
""".strip()


class TestLogParsing:
    def test_get_errors_finds_error_lines(self):
        app = ReasonanceApp.__new__(ReasonanceApp)
        errors = app._parse_errors(SAMPLE_LOGS)
        assert any("Connection refused" in e for e in errors)

    def test_get_errors_finds_panics(self):
        app = ReasonanceApp.__new__(ReasonanceApp)
        errors = app._parse_errors(SAMPLE_LOGS)
        assert any("panicked" in e for e in errors)

    def test_get_frontend_errors(self):
        app = ReasonanceApp.__new__(ReasonanceApp)
        errors = app._parse_frontend_errors(SAMPLE_LOGS)
        assert len(errors) == 1
        assert "Unhandled rejection" in errors[0]

    def test_startup_time_parsed(self):
        app = ReasonanceApp.__new__(ReasonanceApp)
        ms = app._parse_startup_time(SAMPLE_LOGS)
        assert ms is not None
        assert ms >= 0

    def test_is_ready_checks_setup_complete(self):
        app = ReasonanceApp.__new__(ReasonanceApp)
        assert app._is_ready(SAMPLE_LOGS) is True
        assert app._is_ready("just starting...") is False
