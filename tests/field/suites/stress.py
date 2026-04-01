"""Stress test suite — Python overrides.

Functions named test_{id}(ctx) override YAML scenarios of the same ID.
"""

from __future__ import annotations

import os
import re
import signal
import threading
import time

from lib.actions import (
    assert_no_new_errors,
    close_file,
    focus_window,
    press_key,
    screenshot,
    search_palette,
    snapshot_performance,
    toolbar_settings,
    type_text,
    wait_ms,
)
from lib.context import TestContext
from lib.llm import (
    cleanup_session,
    extract_cli_pid,
    send_chat,
    wait_for_done,
)

# File names cycled when we need many distinct filenames
_CYCLE_FILES = [
    "lib.rs",
    "Cargo.toml",
    "tauri.conf.json",
    "App.svelte",
    "main.ts",
    "package.json",
    "README.md",
    "tsconfig.json",
    "vite.config.ts",
    "svelte.config.js",
]


def test_stress_26(ctx: TestContext) -> None:
    """50+ files open."""
    snapshot_performance(ctx)

    # Open 50 files by cycling through available filenames
    for i in range(50):
        filename = _CYCLE_FILES[i % len(_CYCLE_FILES)]
        search_palette(ctx)
        wait_ms(ctx, 300)
        type_text(ctx, filename)
        wait_ms(ctx, 300)
        press_key(ctx, "Return")
        wait_ms(ctx, 500)

    screenshot(ctx, "stress-26-50-files-open")
    snapshot_performance(ctx)
    assert_no_new_errors(ctx)


def test_stress_28(ctx: TestContext) -> None:
    """Large file in editor."""
    large_file = "/tmp/reasonance-large-test.txt"

    # Generate a ~10 MB file
    line = "A" * 99 + "\n"  # 100 bytes per line
    total_lines = 10 * 1024 * 1024 // 100  # ~100 000 lines
    try:
        with open(large_file, "w") as fh:
            for _ in range(total_lines):
                fh.write(line)

        # Open via search palette (absolute path)
        search_palette(ctx)
        wait_ms(ctx, 500)
        type_text(ctx, large_file)
        wait_ms(ctx, 500)
        press_key(ctx, "Return")
        wait_ms(ctx, 5000)

        screenshot(ctx, "stress-28-large-file")
        snapshot_performance(ctx)
    finally:
        if os.path.exists(large_file):
            os.unlink(large_file)


def test_stress_31(ctx: TestContext) -> None:
    """Memory leak check: 100 open/edit/close cycles."""
    snapshot_performance(ctx)

    for i in range(100):
        search_palette(ctx)
        wait_ms(ctx, 300)
        type_text(ctx, "lib.rs")
        wait_ms(ctx, 300)
        press_key(ctx, "Return")
        wait_ms(ctx, 500)
        type_text(ctx, "x")
        wait_ms(ctx, 100)
        close_file(ctx)
        wait_ms(ctx, 200)

        # Snapshot every 25 iterations
        if (i + 1) % 25 == 0:
            snapshot_performance(ctx)

    snapshot_performance(ctx)
    assert_no_new_errors(ctx)


def test_stress_32(ctx: TestContext) -> None:
    """Rapid UI interaction."""
    # Open 5 files
    open_files = ["lib.rs", "Cargo.toml", "tauri.conf.json", "App.svelte", "main.ts"]
    for filename in open_files:
        search_palette(ctx)
        wait_ms(ctx, 300)
        type_text(ctx, filename)
        wait_ms(ctx, 300)
        press_key(ctx, "Return")
        wait_ms(ctx, 500)

    # Rapid tab switches: cycle ctrl+1 through ctrl+5, 50 times total
    tab_keys = ["ctrl+1", "ctrl+2", "ctrl+3", "ctrl+4", "ctrl+5"]
    for i in range(50):
        press_key(ctx, tab_keys[i % len(tab_keys)])
        wait_ms(ctx, 100)

    # Rapid settings toggle: open and close 10 times
    for _ in range(10):
        toolbar_settings(ctx)
        wait_ms(ctx, 100)
        press_key(ctx, "Escape")
        wait_ms(ctx, 100)

    screenshot(ctx, "stress-32-rapid-ui")
    assert_no_new_errors(ctx)


def test_stress_33(ctx: TestContext) -> None:
    """Startup benchmark: 5 launches, measure times."""
    startup_times: list[float] = []

    for i in range(5):
        ctx.app.kill()
        wait_ms(ctx, 2000)

        t_start = time.time()
        ctx.app.launch()
        ready = ctx.app.wait_ready(30)
        elapsed = time.time() - t_start

        if ready:
            focus_window(ctx)
            wait_ms(ctx, 500)

        startup_times.append(elapsed)
        snapshot_performance(ctx)

    avg = sum(startup_times) / len(startup_times)
    max_time = max(startup_times)

    assert avg < 10, f"Average startup time {avg:.1f}s exceeds 10s limit"
    assert max_time < 15, f"Max startup time {max_time:.1f}s exceeds 15s limit"


def test_stress_30(ctx: TestContext) -> None:
    """Multiple concurrent agents — 3 simultaneous LLM sessions."""
    sessions_config = [
        ("claude", "List 5 sorting algorithms"),
        ("claude", "List 5 data structures"),
        ("qwen", "List 5 design patterns"),
    ]

    session_ids: list[str | None] = [None] * len(sessions_config)
    done_results: list[object] = [None] * len(sessions_config)
    errors: list[Exception | None] = [None] * len(sessions_config)

    def run_session(index: int, provider: str, prompt: str) -> None:
        try:
            sid = send_chat(ctx, provider, prompt)
            session_ids[index] = sid
            if sid:
                done_results[index] = wait_for_done(ctx, sid, timeout=90)
        except Exception as exc:  # noqa: BLE001
            errors[index] = exc

    threads = []
    for i, (provider, prompt) in enumerate(sessions_config):
        t = threading.Thread(target=run_session, args=(i, provider, prompt), daemon=True)
        threads.append(t)
        t.start()
        time.sleep(0.5)

    t_start = time.time()
    try:
        for t in threads:
            t.join(timeout=100)

        elapsed = time.time() - t_start

        # All 3 sessions must have completed
        for i, (provider, prompt) in enumerate(sessions_config):
            assert session_ids[i] is not None, (
                f"Session {i} ({provider}: {prompt!r}) never sent a request"
            )
            assert done_results[i] is not None, (
                f"Session {i} ({provider}: {prompt!r}) never reached Done"
            )
            assert errors[i] is None, (
                f"Session {i} ({provider}: {prompt!r}) raised an error: {errors[i]}"
            )

        logs = ctx.app.logs()
        assert "panicked" not in logs, "Rust panic detected during concurrent sessions"
        assert elapsed < 120, f"Concurrent sessions took {elapsed:.1f}s, expected < 120s"

        screenshot(ctx, "stress-30-concurrent-agents")
    finally:
        for sid in session_ids:
            if sid:
                cleanup_session(ctx, sid)


def test_stress_30b(ctx: TestContext) -> None:
    """Circuit breaker trip and recovery."""
    session_ids: list[str | None] = []

    try:
        # Phase 1: Force 3 failures by killing the CLI mid-request
        for i in range(3):
            try:
                sid = send_chat(ctx, "claude", f"Circuit breaker test {i + 1}")
                session_ids.append(sid)
                wait_ms(ctx, 1000)
                if sid:
                    pid = extract_cli_pid(ctx, sid)
                    if pid:
                        os.kill(pid, signal.SIGKILL)
                wait_ms(ctx, 2000)
            except Exception:  # noqa: BLE001
                # Failures are expected here
                pass

        # Check whether the circuit breaker has opened
        logs = ctx.app.logs()
        circuit_open = bool(re.search(r"circuit open", logs, re.IGNORECASE))
        if circuit_open:
            # Wait for the circuit breaker cooldown period
            time.sleep(65)

        # Phase 2: Recovery — send a normal prompt and assert it succeeds
        recovery_sid = send_chat(ctx, "claude", "Say OK")
        assert recovery_sid is not None, "Recovery request did not produce a session_id"

        done = wait_for_done(ctx, recovery_sid, timeout=90)
        assert done is not None, "Recovery session never reached Done — circuit breaker did not reset"

        screenshot(ctx, "stress-30b-circuit-breaker-recovery")
    finally:
        # Always attempt a successful prompt to leave the circuit breaker in a
        # clean state for subsequent tests
        try:
            reset_sid = send_chat(ctx, "claude", "Reset circuit breaker state")
            if reset_sid:
                wait_for_done(ctx, reset_sid, timeout=60)
        except Exception:  # noqa: BLE001
            pass
        for sid in session_ids:
            if sid:
                cleanup_session(ctx, sid)
