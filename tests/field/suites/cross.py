"""Cross-feature tests — agent file I/O and multi-provider schema parity."""

from __future__ import annotations

import os
import re
import time

from lib.actions import (
    assert_no_new_errors,
    screenshot,
    wait_ms,
)
from lib.context import TestContext
from lib.llm import (
    RE_PIPELINE_RULE,
    RE_SESSION_ENDED,
    assert_log_has,
    cleanup_session,
    count_log_events,
    send_chat,
    wait_for_done,
)

_OUTPUT_FILE = "/tmp/reasonance-test-output.txt"


def test_cross_49(ctx: TestContext) -> None:
    """Agent writes file: verify tool use and on-disk result."""
    session_id = None
    try:
        # Pre-clean to avoid stale file from a previous run
        if os.path.exists(_OUTPUT_FILE):
            os.remove(_OUTPUT_FILE)

        session_id = send_chat(
            ctx,
            "claude",
            f"Create a file at {_OUTPUT_FILE} with the content 'field test ok'",
        )
        assert session_id is not None, "send_chat did not return a session_id"

        done = wait_for_done(ctx, session_id, timeout=90)
        assert done is not None, f"Session {session_id} did not emit Done within 90 s"

        # Count events — expect at least one ToolUse or tool_use event
        wait_ms(ctx, 2000)
        events = count_log_events(ctx, session_id)
        tool_count = events.get("ToolUse", 0) + events.get("tool_use", 0)
        assert tool_count >= 1, (
            f"Expected at least 1 ToolUse/tool_use event, got events={events}"
        )

        # Verify the file was actually written to disk
        assert os.path.exists(_OUTPUT_FILE), (
            f"Expected file {_OUTPUT_FILE} to exist after agent tool use"
        )
        with open(_OUTPUT_FILE) as fh:
            content = fh.read()
        assert "field test ok" in content, (
            f"Expected 'field test ok' in file content, got: {content!r}"
        )

        screenshot(ctx, "cross-49-file-written")
        assert_no_new_errors(ctx)
    finally:
        if session_id is not None:
            cleanup_session(ctx, session_id)
        try:
            os.remove(_OUTPUT_FILE)
        except FileNotFoundError:
            pass


def test_cross_52(ctx: TestContext) -> None:
    """Multi-provider normalizer + schema parity: claude vs qwen."""
    session_claude = None
    session_qwen = None
    prompt = "Explain in 2 sentences what a binary search tree is"
    try:
        # --- Phase 1: claude ---
        session_claude = send_chat(ctx, "claude", prompt)
        assert session_claude is not None, "send_chat(claude) did not return a session_id"

        done_claude = wait_for_done(ctx, session_claude, timeout=60)
        assert done_claude is not None, (
            f"Claude session {session_claude} did not emit Done within 60 s"
        )
        wait_ms(ctx, 2000)

        # --- Phase 2: qwen ---
        session_qwen = send_chat(ctx, "qwen", prompt)
        assert session_qwen is not None, "send_chat(qwen) did not return a session_id"

        done_qwen = wait_for_done(ctx, session_qwen, timeout=60)
        assert done_qwen is not None, (
            f"Qwen session {session_qwen} did not emit Done within 60 s"
        )

        # --- Pipeline rule assertions ---
        logs = ctx.app.logs()

        claude_rule_pattern = re.compile(r"Pipeline\[claude\]: matched rule")
        assert claude_rule_pattern.search(logs), (
            "Expected 'Pipeline[claude]: matched rule' in logs"
        )

        qwen_rule_pattern = re.compile(r"Pipeline\[qwen\]: matched rule")
        assert qwen_rule_pattern.search(logs), (
            "Expected 'Pipeline[qwen]: matched rule' in logs"
        )

        # --- Event count assertions per session ---
        events_claude = count_log_events(ctx, session_claude)
        assert events_claude.get("Text", 0) >= 1, (
            f"Expected at least 1 Text event for claude, got events={events_claude}"
        )
        assert events_claude.get("Done", 0) >= 1, (
            f"Expected at least 1 Done event for claude, got events={events_claude}"
        )

        events_qwen = count_log_events(ctx, session_qwen)
        assert events_qwen.get("Text", 0) >= 1, (
            f"Expected at least 1 Text event for qwen, got events={events_qwen}"
        )
        assert events_qwen.get("Done", 0) >= 1, (
            f"Expected at least 1 Done event for qwen, got events={events_qwen}"
        )

        # --- RE_SESSION_ENDED must appear at least once ---
        assert_log_has(
            ctx,
            RE_SESSION_ENDED,
            "Expected 'Session ended: ...' log line but none found",
        )

        # --- Schema parity: compare event type sets (warn only, don't fail) ---
        types_claude = set(events_claude.keys())
        types_qwen = set(events_qwen.keys())
        if types_claude != types_qwen:
            only_claude = types_claude - types_qwen
            only_qwen = types_qwen - types_claude
            print(
                f"WARNING: schema parity mismatch — "
                f"claude-only={sorted(only_claude)}, "
                f"qwen-only={sorted(only_qwen)}"
            )

        screenshot(ctx, "cross-52-multi-provider")
    finally:
        if session_claude is not None:
            cleanup_session(ctx, session_claude)
        if session_qwen is not None:
            cleanup_session(ctx, session_qwen)
