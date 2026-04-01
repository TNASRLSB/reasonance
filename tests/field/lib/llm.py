"""Shared LLM test helpers for Reasonance field tests.

All functions take a TestContext as first argument and operate on the
running app's log stream to observe LLM session lifecycle events.
"""

from __future__ import annotations

import re
import subprocess
import time
from re import Match
from typing import Optional

from lib.actions import (
    click_relative,
    press_key,
    type_text,
    wait_ms,
)
from lib.context import TestContext

# ---------------------------------------------------------------------------
# Compiled regex constants (module-level, reused across tests)
# ---------------------------------------------------------------------------

RE_SEND_REQUEST = re.compile(
    r"Transport: send request provider=(\S+) model=(\S+) session_id=(\S+)"
)
RE_SPAWN_CLI = re.compile(
    r"Transport: spawning CLI binary=(\S+) .* attempt=(\d+)"
)
RE_SESSION_STARTED = re.compile(
    r"Transport: session=(\S+) started for provider=(\S+)"
)
RE_SESSION_STOPPED = re.compile(
    r"Transport: session=(\S+) stopped"
)
RE_PERMISSION_ARGS = re.compile(
    r"Transport: permission_args=(.+)"
)
RE_TRUST_LEVEL = re.compile(
    r"Transport: trust_level=(\S+)"
)
RE_STREAM_EMIT = re.compile(
    r"StreamReader\[(\S+)\]: emitting (\S+)"
)
RE_STREAM_CLI_SESSION = re.compile(
    r"StreamReader\[(\S+)\]: captured CLI session ID: (\S+)"
)
RE_PIPELINE_RULE = re.compile(
    r"Pipeline\[(\S+)\]: matched rule '(\S+)'"
)
RE_SESSION_ENDED = re.compile(
    r"Session ended: session_id=(\S+), input_tokens=(\d+), output_tokens=(\d+)"
)
RE_SESSION_TRACKING_STARTED = re.compile(
    r"Session tracking started: session_id=(\S+), provider=(\S+)"
)
RE_SESSION_CREATED = re.compile(
    r"SessionManager: session created session_id=(\S+)"
)
RE_SESSION_FORKED = re.compile(
    r"SessionManager: forked session=(\S+) -> new session=(\S+) with (\d+) events"
)
RE_SESSION_RENAMED = re.compile(
    r"SessionManager: session=(\S+) renamed"
)
RE_SESSION_DELETED = re.compile(
    r"SessionManager: session=(\S+) deleted"
)
RE_SESSIONS_LOADED = re.compile(
    r"SessionManager: loaded (\d+) existing sessions"
)
RE_SM_LOADED = RE_SESSIONS_LOADED  # alias used by e2e_10b
RE_SESSION_RESTORED = re.compile(
    r"SessionManager: session=(\S+) restored with (\d+) events"
)


# ---------------------------------------------------------------------------
# Log polling helpers
# ---------------------------------------------------------------------------


def log_line_count(ctx: TestContext) -> int:
    """Return the current number of lines in the app log."""
    return len(ctx.app.logs().splitlines())


def wait_for_log(
    ctx: TestContext,
    pattern: re.Pattern,
    timeout: int = 60,
) -> Optional[Match]:
    """Poll app logs until *pattern* matches any line.

    Returns the first Match object found, or None if *timeout* expires.
    """
    deadline = time.time() + timeout
    while time.time() < deadline:
        text = ctx.app.logs()
        m = pattern.search(text)
        if m:
            return m
        time.sleep(1)
    return None


def wait_for_log_after(
    ctx: TestContext,
    pattern: re.Pattern,
    after_line: int,
    timeout: int = 60,
) -> Optional[Match]:
    """Poll app logs for *pattern*, but only in lines after *after_line* offset.

    *after_line* is a 0-based line index (i.e. the value returned by
    ``log_line_count`` before the action under test was triggered).

    Returns the first Match object found in the new lines, or None on timeout.
    """
    deadline = time.time() + timeout
    while time.time() < deadline:
        lines = ctx.app.logs().splitlines()
        tail = "\n".join(lines[after_line:])
        m = pattern.search(tail)
        if m:
            return m
        time.sleep(1)
    return None


# ---------------------------------------------------------------------------
# Session ID extraction
# ---------------------------------------------------------------------------


def extract_session_id(ctx: TestContext, timeout: int = 30) -> Optional[str]:
    """Wait for a ``Transport: send request`` log line and return session_id.

    Returns the session_id string, or None if *timeout* expires.
    """
    m = wait_for_log(ctx, RE_SEND_REQUEST, timeout=timeout)
    if m:
        return m.group(3)
    return None


# ---------------------------------------------------------------------------
# Stream completion helpers
# ---------------------------------------------------------------------------


def wait_for_done(
    ctx: TestContext,
    session_id: str,
    timeout: int = 60,
) -> Optional[Match]:
    """Wait until ``StreamReader[<session_id>]: emitting Done`` appears.

    Returns the Match, or None on timeout.
    """
    pattern = re.compile(
        r"StreamReader\[" + re.escape(session_id) + r"\]: emitting Done"
    )
    return wait_for_log(ctx, pattern, timeout=timeout)


def count_log_events(ctx: TestContext, session_id: str) -> dict[str, int]:
    """Count all StreamReader emit events for *session_id*.

    Returns a dict mapping event type (e.g. ``"Text"``, ``"Done"``) to count.
    """
    pattern = re.compile(
        r"StreamReader\[" + re.escape(session_id) + r"\]: emitting (\S+)"
    )
    counts: dict[str, int] = {}
    for m in pattern.finditer(ctx.app.logs()):
        event_type = m.group(1)
        counts[event_type] = counts.get(event_type, 0) + 1
    return counts


# ---------------------------------------------------------------------------
# Assertion helper
# ---------------------------------------------------------------------------


def assert_log_has(
    ctx: TestContext,
    pattern: re.Pattern,
    msg: str,
) -> Match:
    """Assert *pattern* exists anywhere in the app log.

    Returns the first Match on success, raises AssertionError with *msg* if
    the pattern is not found.
    """
    m = pattern.search(ctx.app.logs())
    if not m:
        raise AssertionError(msg)
    return m


# ---------------------------------------------------------------------------
# Process helpers
# ---------------------------------------------------------------------------


def extract_cli_pid(ctx: TestContext, session_id: str) -> Optional[int]:  # noqa: ARG001
    """Find the CLI child process PID spawned for this session.

    Uses ``pgrep -P <app_pid> -f 'claude|qwen'`` to locate the child.
    *session_id* is accepted for API symmetry but is not used in the search
    because the CLI process name carries no session information.

    Returns the integer PID of the first match, or None if not found.
    """
    app_pid = ctx.app.pid
    if app_pid is None:
        return None
    try:
        result = subprocess.run(
            ["pgrep", "-P", str(app_pid), "-f", "claude|qwen"],
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            pids = result.stdout.strip().splitlines()
            if pids:
                return int(pids[0])
    except (OSError, ValueError):
        pass
    return None


# ---------------------------------------------------------------------------
# Session lifecycle helpers
# ---------------------------------------------------------------------------


def cleanup_session(ctx: TestContext, session_id: str) -> None:  # noqa: ARG001
    """Best-effort cancellation of an in-progress streaming session.

    Sends Escape to stop any active stream.  *session_id* is accepted for
    API symmetry but the UI cancel action is not session-specific.
    """
    press_key(ctx, "Escape")
    time.sleep(0.3)


# ---------------------------------------------------------------------------
# Panel helpers
# ---------------------------------------------------------------------------


def open_sessions_panel(ctx: TestContext) -> None:
    """Open the sessions panel via Ctrl+Shift+H and wait 1 s."""
    press_key(ctx, "ctrl+shift+h")
    time.sleep(1)


def close_sessions_panel(ctx: TestContext) -> None:
    """Close the sessions panel via Escape and wait 300 ms."""
    press_key(ctx, "Escape")
    wait_ms(ctx, 300)


# ---------------------------------------------------------------------------
# Chat interaction helpers
# ---------------------------------------------------------------------------


def send_chat_message(ctx: TestContext, text: str) -> None:
    """Click the chat input area, type *text*, and press Return.

    The chat input lives in the right column at approximately 85 % x, 92 % y.
    """
    click_relative(ctx, 0.85, 0.92)
    time.sleep(0.2)
    type_text(ctx, text)
    press_key(ctx, "Return")


def send_chat(
    ctx: TestContext,
    provider: str,  # noqa: ARG001
    prompt: str,
    yolo: bool = True,  # noqa: ARG001
) -> Optional[str]:
    """Full flow to send a chat message and return the resulting session_id.

    Steps:
    1. Record the current log line count as a baseline.
    2. Open the sessions panel.
    3. Send the chat message.
    4. Wait for a ``Transport: send request`` log line that appears after the
       baseline, then return its session_id.

    *provider* and *yolo* are accepted for future use but are not acted upon
    here — provider selection is assumed to already be configured in the UI.

    Returns the session_id string, or None if no request was logged within the
    default timeout.
    """
    baseline = log_line_count(ctx)
    open_sessions_panel(ctx)
    send_chat_message(ctx, prompt)
    m = wait_for_log_after(ctx, RE_SEND_REQUEST, after_line=baseline, timeout=60)
    if m:
        return m.group(3)
    return None
