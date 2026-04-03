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
    focus_window,
    press_key,
    type_text,
    wait_ms,
)
from lib.window import minimize_others
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
RE_PROJECT_OPENED = re.compile(
    r"set_project_root\(path="
)

# Default test project for ensure_project_open
DEFAULT_TEST_PROJECT = "/home/uh1/VIBEPROJECTS/reasonance-test"



# ---------------------------------------------------------------------------
# Project helpers
# ---------------------------------------------------------------------------


def ensure_project_open(
    ctx: TestContext,
    project_path: str = DEFAULT_TEST_PROJECT,
) -> None:
    """Ensure a project is open before interacting with the chat UI.

    Checks logs for ``set_project_root``.  If not found, waits up to 10 s
    (the user or a prior test may still be opening a folder).  Raises if the
    app is still on the WelcomeScreen after the grace period.

    When running with ``--no-launch``, open the project manually before
    starting the tests.
    """
    # If logs are available, check for set_project_root
    logs = ctx.app.logs()
    if logs:
        deadline = time.time() + 10
        while time.time() < deadline:
            if RE_PROJECT_OPENED.search(ctx.app.logs()):
                return
            time.sleep(1)
        raise RuntimeError(
            "ensure_project_open: no project is open (no set_project_root "
            "in logs).  Open a project manually before running LLM tests, "
            "or run the full suite (--all) which opens one via smoke_02."
        )
    # --no-launch mode: no logs available, trust that the user opened a project


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
# Terminal column layout constants
# ---------------------------------------------------------------------------
# The terminal column is on the right side of the 3-column layout.
# Default width: 300px in a 1280px window.
# FileTree: 200px, Editor: flex, Terminal: 300px
# Terminal column x range: ~0.766 to 1.0 (center ~0.883)

TERMINAL_COL_CENTER_X = 0.83  # center of terminal column (1920px screen)
TERMINAL_TAB_Y = 0.05  # tab bar y position (below toolbar)
CHAT_INPUT_Y = 0.95  # ChatInput at bottom (above status bar)


# ---------------------------------------------------------------------------
# Provider card positions in the terminal empty state
# ---------------------------------------------------------------------------
# When no sessions exist, the terminal column shows provider cards.
# The cards are stacked vertically, centered in the column.
# Calibrated for 1920x1080 maximized window with 6 LLMs:
# Claude, Gemini, Ollama, Kimi, Qwen, Codex

PROVIDER_CARD_Y = {
    0: 0.46,  # CLAUDE
    1: 0.51,  # GEMINI
    2: 0.55,  # OLLAMA
    3: 0.60,  # KIMI
    4: 0.64,  # QWEN
    5: 0.69,  # CODEX
}


# ---------------------------------------------------------------------------
# Chat interaction helpers
# ---------------------------------------------------------------------------


def _click_provider_card(ctx: TestContext, index: int = 0) -> None:
    """Click a provider card in the terminal empty state.

    *index* is the 0-based position of the provider in the discovered list.
    Claude is typically index 0.
    """
    y = PROVIDER_CARD_Y.get(index, 0.40)
    click_relative(ctx, TERMINAL_COL_CENTER_X, y)
    time.sleep(1.5)


def _click_add_tab(ctx: TestContext) -> None:
    """Click the '+' tab button at the end of the terminal tab bar."""
    # The '+' button is at the right edge of the tab bar
    click_relative(ctx, 0.97, TERMINAL_TAB_Y)
    time.sleep(0.5)


def _click_chat_input(ctx: TestContext) -> None:
    """Click the ChatInput textarea at the bottom of the terminal column."""
    click_relative(ctx, TERMINAL_COL_CENTER_X, CHAT_INPUT_Y)
    time.sleep(0.2)


def send_chat_message(ctx: TestContext, text: str) -> None:
    """Click the chat input area, type *text*, and press Return."""
    _click_chat_input(ctx)
    type_text(ctx, text)
    time.sleep(0.2)
    press_key(ctx, "Return")


def send_chat(
    ctx: TestContext,
    provider: str,
    prompt: str,
    yolo: bool = True,  # noqa: ARG001
) -> Optional[str]:
    """Full flow: create LLM session, send prompt, return session_id.

    Steps:
    1. Record the current log line count as a baseline.
    2. Click a provider card in the terminal column to start a new session.
       If a session already exists, click the '+' tab and select from dropdown.
    3. Wait for session creation in logs.
    4. Type the prompt in ChatInput and press Enter.
    5. Wait for ``Transport: send request`` and return session_id.

    Returns the session_id string, or None if no request was logged.
    """
    # Bring Reasonance to front via Alt+Tab (KWin scripts unreliable)
    press_key(ctx, "alt+Tab")
    time.sleep(1)

    baseline = log_line_count(ctx)
    # Full logs available only when the runner launched the app (contains
    # startup marker).  The Tauri plugin LogDir is too small/filtered.
    has_logs = "setup complete" in ctx.app.logs()

    # Determine provider index in discovered order
    provider_lower = provider.lower()
    # Discovery order: Claude, Gemini, Aider, Copilot, Ollama, Interpreter, Kimi, Qwen, Codex
    # Only found ones appear as cards. Typical config: Claude=0, Qwen=4-ish
    provider_index = {"claude": 0, "qwen": 4}.get(provider_lower, 0)

    # Click a provider card in the terminal column to start a new session.
    _click_provider_card(ctx, provider_index)
    time.sleep(2)

    if has_logs:
        # Check if a session was created
        m = wait_for_log_after(ctx, RE_SESSION_STARTED, after_line=baseline, timeout=5)
        if not m:
            # Provider card click didn't work — terminal already has tabs.
            _click_add_tab(ctx)
            time.sleep(0.5)
            dropdown_y = TERMINAL_TAB_Y + 0.04 + (provider_index * 0.035)
            click_relative(ctx, 0.95, dropdown_y)
            time.sleep(1.5)

    # Now type the prompt in the ChatInput
    send_chat_message(ctx, prompt)

    if has_logs:
        # Wait for Transport: send request
        m = wait_for_log_after(ctx, RE_SEND_REQUEST, after_line=baseline, timeout=30)
        if m:
            return m.group(3)
        return None
    else:
        # No logs available (--no-launch mode): wait and return placeholder
        time.sleep(5)
        return "no-log-session"
