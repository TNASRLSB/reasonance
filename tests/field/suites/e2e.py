"""E2E test suite — Python overrides.

Functions named test_{id}(ctx) override YAML scenarios of the same ID.
"""

from __future__ import annotations

import re
import time

from lib.actions import (
    assert_no_new_errors,
    checkpoint,
    close_file,
    focus_window,
    press_key,
    screenshot,
    search_palette,
    snapshot_performance,
    type_text,
    undo,
    wait_ms,
)
from lib.context import TestContext
from lib.llm import (
    RE_PERMISSION_ARGS,
    RE_PIPELINE_RULE,
    RE_SEND_REQUEST,
    RE_SESSION_CREATED,
    RE_SESSION_ENDED,
    RE_SESSION_TRACKING_STARTED,
    RE_SM_LOADED,
    RE_SPAWN_CLI,
    assert_log_has,
    cleanup_session,
    count_log_events,
    open_sessions_panel,
    send_chat,
    wait_for_done,
    wait_for_log,
    wait_for_log_after,
    RE_TRUST_LEVEL,
    log_line_count,
)


def test_e2e_09(ctx: TestContext) -> None:
    """Editor full workflow: open, edit, undo, multi-tab, close."""
    # Open lib.rs
    search_palette(ctx)
    wait_ms(ctx, 500)
    type_text(ctx, "lib.rs")
    wait_ms(ctx, 500)
    press_key(ctx, "Return")
    wait_ms(ctx, 1500)

    # Make an edit then undo
    type_text(ctx, "// field test edit")
    wait_ms(ctx, 300)
    undo(ctx)
    wait_ms(ctx, 300)

    # Open a second file in a new tab
    search_palette(ctx)
    wait_ms(ctx, 500)
    type_text(ctx, "Cargo.toml")
    wait_ms(ctx, 500)
    press_key(ctx, "Return")
    wait_ms(ctx, 1500)

    # Close the current tab
    close_file(ctx)
    wait_ms(ctx, 500)

    assert_no_new_errors(ctx)


def test_e2e_11(ctx: TestContext) -> None:
    """Terminal PTY: open, type command, verify."""
    press_key(ctx, "ctrl+grave")
    wait_ms(ctx, 2000)
    type_text(ctx, "echo FIELD_TEST_OK")
    wait_ms(ctx, 300)
    press_key(ctx, "Return")
    wait_ms(ctx, 1000)
    screenshot(ctx, "e2e-11-terminal")
    assert_no_new_errors(ctx)


def test_e2e_22(ctx: TestContext) -> None:
    """PTY resilience: verify terminal works."""
    press_key(ctx, "ctrl+grave")
    wait_ms(ctx, 2000)
    type_text(ctx, "echo $$")
    wait_ms(ctx, 300)
    press_key(ctx, "Return")
    wait_ms(ctx, 1000)
    checkpoint(ctx, "e2e-22-pty-resilience")
    assert_no_new_errors(ctx)


def test_e2e_25(ctx: TestContext) -> None:
    """Full state persistence: open 3 files, kill, verify."""
    files = ["lib.rs", "Cargo.toml", "tauri.conf.json"]
    for filename in files:
        search_palette(ctx)
        wait_ms(ctx, 500)
        type_text(ctx, filename)
        wait_ms(ctx, 500)
        press_key(ctx, "Return")
        wait_ms(ctx, 1500)

    screenshot(ctx, "e2e-25-state-before-kill")

    ctx.app.kill()
    wait_ms(ctx, 2000)

    ctx.app.launch()
    assert ctx.app.wait_ready(60), "App did not become ready after relaunch"

    focus_window(ctx)
    wait_ms(ctx, 3000)
    screenshot(ctx, "e2e-25-state-after-relaunch")
    snapshot_performance(ctx)


def test_e2e_10(ctx: TestContext) -> None:
    """Chat with real LLM: send a Rust snippet, verify response and token counts."""
    session_id = None
    try:
        screenshot(ctx, "e2e-10-before")
        session_id = send_chat(
            ctx,
            "claude",
            "Analyze this Rust snippet and suggest one improvement: fn add(a: i32, b: i32) -> i32 { return a + b; }",
        )
        assert session_id is not None, "No session_id returned from send_chat"

        done = wait_for_done(ctx, session_id, timeout=60)
        assert done is not None, f"Session {session_id} did not complete within 60s"

        assert_log_has(ctx, RE_SEND_REQUEST, "Expected Transport: send request log line")
        assert_log_has(ctx, RE_SPAWN_CLI, "Expected Transport: spawning CLI log line")

        events = count_log_events(ctx, session_id)
        assert events.get("Text", 0) >= 1, f"Expected at least 1 Text event, got {events}"
        assert events.get("Done", 0) >= 1, f"Expected at least 1 Done event, got {events}"

        m = assert_log_has(ctx, RE_SESSION_ENDED, "Expected Session ended log line with token counts")
        output_tokens = int(m.group(3))
        assert output_tokens > 0, f"Expected output_tokens > 0, got {output_tokens}"

        screenshot(ctx, "e2e-10-after")
        assert_no_new_errors(ctx)
    finally:
        if session_id:
            cleanup_session(ctx, session_id)


def test_e2e_10f(ctx: TestContext) -> None:
    """Streaming UX verification: confirm progressive text delivery, not batch."""
    session_id = None
    try:
        baseline = log_line_count(ctx)
        session_id = send_chat(
            ctx,
            "claude",
            "Write a detailed explanation of how quicksort works, step by step",
        )
        assert session_id is not None, "No session_id returned from send_chat"

        first_text_pattern = re.compile(
            r"StreamReader\[" + re.escape(session_id) + r"\]: emitting.*Text"
        )
        first_text_match = wait_for_log_after(ctx, first_text_pattern, after_line=baseline, timeout=30)
        assert first_text_match is not None, "No Text event received within 30s"

        stream_start = time.time()
        wait_ms(ctx, 3000)
        screenshot(ctx, "e2e-10f-mid-stream")

        done = wait_for_done(ctx, session_id, timeout=90)
        assert done is not None, f"Session {session_id} did not complete within 90s"
        stream_end = time.time()

        events = count_log_events(ctx, session_id)
        assert events.get("Text", 0) >= 3, (
            f"Expected >= 3 Text events to prove streaming, got {events}"
        )

        stream_duration = stream_end - stream_start
        assert stream_duration > 2.0, (
            f"Expected stream_duration > 2.0s to prove progressive delivery, got {stream_duration:.2f}s"
        )

        screenshot(ctx, "e2e-10f-final")
    finally:
        if session_id:
            cleanup_session(ctx, session_id)


def test_e2e_10d(ctx: TestContext) -> None:
    """Tool use round-trip (Claude only): read a file via tool and report version."""
    session_id = None
    try:
        session_id = send_chat(
            ctx,
            "claude",
            "Read the file src-tauri/Cargo.toml and tell me the project version number",
        )
        assert session_id is not None, "No session_id returned from send_chat"

        done = wait_for_done(ctx, session_id, timeout=90)
        assert done is not None, f"Session {session_id} did not complete within 90s"

        assert_log_has(ctx, RE_PIPELINE_RULE, "Expected Pipeline matched rule log line")

        events = count_log_events(ctx, session_id)
        tool_use_count = events.get("ToolUse", 0) + events.get("tool_use", 0)
        tool_result_count = events.get("ToolResult", 0) + events.get("tool_result", 0)
        assert tool_use_count >= 1, (
            f"Expected at least 1 ToolUse event, got {events}"
        )
        assert tool_result_count >= 1, (
            f"Expected at least 1 ToolResult event, got {events}"
        )
        assert events.get("Text", 0) >= 1, (
            f"Expected at least 1 Text event in tool response, got {events}"
        )

        screenshot(ctx, "e2e-10d-tool-use")
    finally:
        if session_id:
            cleanup_session(ctx, session_id)


def test_e2e_10e(ctx: TestContext) -> None:
    """Permission flow: verify permission args and trust level are logged."""
    session_id = None
    try:
        session_id = send_chat(ctx, "claude", "What is 2+2?", yolo=True)
        assert session_id is not None, "No session_id returned from send_chat"

        done = wait_for_done(ctx, session_id, timeout=60)
        assert done is not None, f"Session {session_id} did not complete within 60s"

        assert_log_has(ctx, RE_PERMISSION_ARGS, "Expected Transport: permission_args log line")
        assert_log_has(ctx, RE_TRUST_LEVEL, "Expected Transport: trust_level log line")

        screenshot(ctx, "e2e-10e-permission-flow")
    finally:
        if session_id:
            cleanup_session(ctx, session_id)


def test_e2e_10h(ctx: TestContext) -> None:
    """Analytics pipeline: verify session tracking and token counts are emitted."""
    session_id = None
    try:
        session_id = send_chat(ctx, "claude", "What is 2+2?")
        assert session_id is not None, "No session_id returned from send_chat"

        done = wait_for_done(ctx, session_id, timeout=60)
        assert done is not None, f"Session {session_id} did not complete within 60s"

        wait_ms(ctx, 2000)

        assert_log_has(
            ctx,
            RE_SESSION_TRACKING_STARTED,
            "Expected Session tracking started log line",
        )

        m = assert_log_has(ctx, RE_SESSION_ENDED, "Expected Session ended log line")
        input_tokens = int(m.group(2))
        output_tokens = int(m.group(3))
        assert input_tokens > 0, f"Expected input_tokens > 0, got {input_tokens}"
        assert output_tokens > 0, f"Expected output_tokens > 0, got {output_tokens}"
    finally:
        if session_id:
            cleanup_session(ctx, session_id)


def test_e2e_10g(ctx: TestContext) -> None:
    """Session rename/delete lifecycle: send a chat and verify session created."""
    session_id = None
    try:
        session_id = send_chat(ctx, "claude", "Say hello")
        assert session_id is not None, "No session_id returned from send_chat"

        done = wait_for_done(ctx, session_id, timeout=60)
        assert done is not None, f"Session {session_id} did not complete within 60s"

        open_sessions_panel(ctx)
        screenshot(ctx, "e2e-10g-sessions-panel")

        assert_log_has(
            ctx,
            RE_SESSION_CREATED,
            "Expected SessionManager: session created log line",
        )

        screenshot(ctx, "e2e-10g-session-lifecycle")
    finally:
        if session_id:
            cleanup_session(ctx, session_id)


def test_e2e_10c(ctx: TestContext) -> None:
    """Session fork: send a chat and verify session created (fork precondition)."""
    session_id = None
    try:
        session_id = send_chat(ctx, "claude", "List 3 programming languages")
        assert session_id is not None, "No session_id returned from send_chat"

        done = wait_for_done(ctx, session_id, timeout=60)
        assert done is not None, f"Session {session_id} did not complete within 60s"

        open_sessions_panel(ctx)

        assert_log_has(
            ctx,
            RE_SESSION_CREATED,
            "Expected SessionManager: session created log line",
        )

        screenshot(ctx, "e2e-10c-session-fork")
    finally:
        if session_id:
            cleanup_session(ctx, session_id)


def test_e2e_10b(ctx: TestContext) -> None:
    """Session persistence: send message, restart app, verify session survives."""
    session_id = None
    try:
        # Phase 1: create a session with a response
        session_id = send_chat(ctx, "claude", "What is the capital of France?")
        done = wait_for_done(ctx, session_id, timeout=60)
        assert done, "Initial prompt did not complete"
        screenshot(ctx, "e2e-10b-before-restart")

        # Phase 2: kill and relaunch
        ctx.app.kill()
        wait_ms(ctx, 3000)
        ctx.app.launch()
        ready = ctx.app.wait_ready(120)
        assert ready, "App did not become ready after relaunch"
        focus_window(ctx)
        wait_ms(ctx, 3000)

        # Phase 3: verify sessions were loaded from disk
        m = wait_for_log(ctx, RE_SM_LOADED, timeout=10)
        assert m, "SessionManager did not log loading sessions from index"
        loaded_count = int(m.group(1))
        assert loaded_count >= 1, f"Expected >= 1 loaded sessions, got {loaded_count}"

        screenshot(ctx, "e2e-10b-after-restart")
    finally:
        if session_id:
            cleanup_session(ctx, session_id)
