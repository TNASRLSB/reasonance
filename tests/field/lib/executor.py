"""YamlExecutor — runs YAML scenario steps against a live TestContext."""

from __future__ import annotations

import time
from typing import Callable, TYPE_CHECKING

if TYPE_CHECKING:
    from lib.context import TestContext


class YamlExecutor:
    """Execute the steps of a parsed YAML scenario dict.

    Parameters
    ----------
    action_registry:
        Mapping of action name → callable ``fn(ctx, **params)``.
    """

    def __init__(self, action_registry: dict[str, Callable]):
        self._registry = action_registry

    def execute(self, scenario: dict, ctx: "TestContext") -> dict:
        """Execute all steps in *scenario* and return a result dict.

        Execution never aborts early: failures are collected and the next step
        always runs.  The overall status is ``"fail"`` if any errors occurred,
        ``"pass"`` otherwise.
        """
        start = time.monotonic()
        errors: list[str] = []
        steps = scenario.get("steps", [])
        steps_executed = 0

        for step in steps:
            # 1. Resolve action name
            action_name = step.get("action")
            if action_name is None:
                errors.append("step is missing required 'action' key")
                continue

            # 2. Log step_start to telemetry
            ctx.telemetry.mark("step_start", action=action_name)

            # 3. Look up function in registry
            fn = self._registry.get(action_name)
            if fn is None:
                msg = f"unknown action '{action_name}'"
                errors.append(msg)
                ctx.telemetry.mark("step_fail", action=action_name, reason=msg)
                steps_executed += 1
                continue

            # 4. Build params (everything except "action")
            params = {k: v for k, v in step.items() if k != "action"}

            # 5. Call the action; capture any exception
            try:
                fn(ctx, **params)
                ctx.telemetry.mark("step_pass", action=action_name)
            except Exception as exc:  # noqa: BLE001
                msg = str(exc)
                errors.append(msg)
                ctx.telemetry.mark("step_fail", action=action_name, reason=msg)

            steps_executed += 1

        duration_ms = int((time.monotonic() - start) * 1000)

        return {
            "id": scenario["id"],
            "name": scenario["name"],
            "suite": scenario.get("suite", "unknown"),
            "status": "fail" if errors else "pass",
            "duration_ms": duration_ms,
            "screenshots": [],
            "errors": errors,
            "notes": "",
            "steps_executed": steps_executed,
            "steps_total": len(steps),
        }
