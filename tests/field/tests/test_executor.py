import os
import sys
from unittest.mock import MagicMock, call

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from lib.executor import YamlExecutor
from lib.context import TestContext


def _make_ctx():
    """Return a minimal TestContext backed by MagicMocks."""
    app = MagicMock()
    app.get_errors.return_value = []
    telemetry = MagicMock()
    return TestContext(
        app=app,
        telemetry=telemetry,
        screenshot_dir="/tmp/screenshots",
        test_id="exec_01",
        test_name="Executor test",
        verbose=False,
        window_rect={"x": 0, "y": 0, "width": 1920, "height": 1080},
    )


def _make_scenario(steps: list[dict], *, id: str = "scn_01", name: str = "Test scenario", suite: str = "unit") -> dict:
    return {"id": id, "name": name, "suite": suite, "steps": steps}


class TestExecuteSimpleScenario:
    def test_execute_simple_scenario(self):
        """Two fake actions that succeed → status 'pass', both called in order."""
        call_log = []

        def wait_ms(ctx, duration_ms=0):
            call_log.append(("wait_ms", duration_ms))

        def assert_no_errors(ctx):
            call_log.append(("assert_no_errors",))

        registry = {"wait_ms": wait_ms, "assert_no_errors": assert_no_errors}
        executor = YamlExecutor(registry)
        ctx = _make_ctx()

        scenario = _make_scenario([
            {"action": "wait_ms", "duration_ms": 100},
            {"action": "assert_no_errors"},
        ])

        result = executor.execute(scenario, ctx)

        assert result["status"] == "pass"
        assert result["errors"] == []
        assert call_log == [("wait_ms", 100), ("assert_no_errors",)]


class TestExecuteCapturesFailure:
    def test_execute_captures_failure(self):
        """Action raises AssertionError → status 'fail', message in errors."""

        def bad_action(ctx):
            raise AssertionError("expected blue, got red")

        registry = {"bad_action": bad_action}
        executor = YamlExecutor(registry)
        ctx = _make_ctx()

        scenario = _make_scenario([{"action": "bad_action"}])
        result = executor.execute(scenario, ctx)

        assert result["status"] == "fail"
        assert len(result["errors"]) == 1
        assert "expected blue, got red" in result["errors"][0]


class TestExecuteUnknownAction:
    def test_execute_unknown_action(self):
        """Action name not in registry → status 'fail', error mentions name."""
        registry = {}
        executor = YamlExecutor(registry)
        ctx = _make_ctx()

        scenario = _make_scenario([{"action": "no_such_action"}])
        result = executor.execute(scenario, ctx)

        assert result["status"] == "fail"
        assert any("no_such_action" in e for e in result["errors"])


class TestExecuteContinuesAfterStepFailure:
    def test_execute_continues_after_step_failure(self):
        """3 steps (ok, fail, ok) — all 3 run despite the middle failure."""
        executed = []

        def step_ok(ctx, label=""):
            executed.append(label or "ok")

        def step_fail(ctx):
            executed.append("fail")
            raise RuntimeError("boom")

        registry = {"step_ok": step_ok, "step_fail": step_fail}
        executor = YamlExecutor(registry)
        ctx = _make_ctx()

        scenario = _make_scenario([
            {"action": "step_ok", "label": "first"},
            {"action": "step_fail"},
            {"action": "step_ok", "label": "third"},
        ])

        result = executor.execute(scenario, ctx)

        # All three steps ran
        assert executed == ["first", "fail", "third"]
        # Overall status is fail because middle step errored
        assert result["status"] == "fail"
        assert result["steps_executed"] == 3


class TestResultShape:
    def test_result_includes_duration_and_screenshots(self):
        """Result must have duration_ms (int) and screenshots (list)."""

        def noop(ctx):
            pass

        registry = {"noop": noop}
        executor = YamlExecutor(registry)
        ctx = _make_ctx()

        scenario = _make_scenario([{"action": "noop"}])
        result = executor.execute(scenario, ctx)

        assert isinstance(result["duration_ms"], int)
        assert isinstance(result["screenshots"], list)

    def test_result_metadata_fields(self):
        """Result carries id, name, suite, steps_total from the scenario."""

        def noop(ctx):
            pass

        registry = {"noop": noop}
        executor = YamlExecutor(registry)
        ctx = _make_ctx()

        scenario = _make_scenario(
            [{"action": "noop"}, {"action": "noop"}],
            id="meta_01",
            name="Metadata check",
            suite="integration",
        )
        result = executor.execute(scenario, ctx)

        assert result["id"] == "meta_01"
        assert result["name"] == "Metadata check"
        assert result["suite"] == "integration"
        assert result["steps_total"] == 2

    def test_result_notes_is_string(self):
        """Result 'notes' field is a string (empty by default)."""
        registry = {}
        executor = YamlExecutor(registry)
        ctx = _make_ctx()

        scenario = _make_scenario([])
        result = executor.execute(scenario, ctx)

        assert isinstance(result["notes"], str)

    def test_result_suite_defaults_to_unknown(self):
        """Scenario without 'suite' key → result['suite'] == 'unknown'."""
        registry = {}
        executor = YamlExecutor(registry)
        ctx = _make_ctx()

        scenario = {"id": "s1", "name": "No suite", "steps": []}
        result = executor.execute(scenario, ctx)

        assert result["suite"] == "unknown"


class TestTelemetryIntegration:
    def test_telemetry_step_events_logged(self):
        """Each step logs step_start and either step_pass or step_fail."""

        def good(ctx):
            pass

        def bad(ctx):
            raise ValueError("oops")

        registry = {"good": good, "bad": bad}
        executor = YamlExecutor(registry)
        ctx = _make_ctx()

        scenario = _make_scenario([
            {"action": "good"},
            {"action": "bad"},
        ])
        executor.execute(scenario, ctx)

        mark_calls = ctx.telemetry.mark.call_args_list
        events = [c[0][0] for c in mark_calls]  # first positional arg of each call

        assert "step_start" in events
        # At least one pass and one fail event
        assert any("step_pass" in e for e in events)
        assert any("step_fail" in e for e in events)
