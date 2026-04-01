"""Reasonance app lifecycle management."""

import os
import re
import signal
import subprocess
import time
from datetime import datetime
from pathlib import Path

PROJECT_ROOT = Path(__file__).parent.parent.parent.parent  # tests/field/../../ = repo root


class ReasonanceApp:
    def __init__(self):
        self._process = None
        self._log_fd = None
        self._log_file = f"/tmp/reasonance-field-{datetime.now().strftime('%H%M%S')}.log"
        self._launch_time = None

    def launch(self, env: dict = None):
        """Start Reasonance via npx tauri dev with logging."""
        launch_env = os.environ.copy()
        launch_env["RUST_LOG"] = "trace"
        launch_env["RUST_BACKTRACE"] = "full"
        launch_env["WEBKIT_DISABLE_COMPOSITING_MODE"] = "1"
        if env:
            launch_env.update(env)

        self._log_fd = open(self._log_file, "w")
        self._launch_time = time.time()
        self._process = subprocess.Popen(
            ["npx", "tauri", "dev"],
            cwd=str(PROJECT_ROOT),
            stdout=self._log_fd,
            stderr=subprocess.STDOUT,
            env=launch_env,
            preexec_fn=os.setsid,
        )

    def wait_ready(self, timeout: int = 120) -> bool:
        """Block until 'setup complete' appears in logs or timeout."""
        deadline = time.time() + timeout
        while time.time() < deadline:
            logs = self.logs()
            if self._is_ready(logs):
                return True
            if self._process and self._process.poll() is not None:
                return False
            time.sleep(2)
        return False

    def wait_for_log(self, pattern: str, timeout: int = 10) -> bool:
        deadline = time.time() + timeout
        while time.time() < deadline:
            if pattern in self.logs():
                return True
            if self._process and self._process.poll() is not None:
                return False
            time.sleep(0.5)
        return False

    def kill(self):
        """Terminate the app and all child processes."""
        if self._process:
            try:
                os.killpg(os.getpgid(self._process.pid), signal.SIGTERM)
                self._process.wait(timeout=5)
            except (ProcessLookupError, subprocess.TimeoutExpired):
                try:
                    os.killpg(os.getpgid(self._process.pid), signal.SIGKILL)
                except ProcessLookupError:
                    pass
            self._process = None
        if self._log_fd:
            self._log_fd.close()
            self._log_fd = None

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.kill()

    def is_running(self) -> bool:
        return self._process is not None and self._process.poll() is None

    def logs(self) -> str:
        try:
            with open(self._log_file, "r") as f:
                return f.read()
        except FileNotFoundError:
            return ""

    def get_errors(self) -> list[str]:
        return self._parse_errors(self.logs())

    def get_frontend_errors(self) -> list[str]:
        return self._parse_frontend_errors(self.logs())

    def startup_time_ms(self) -> int | None:
        return self._parse_startup_time(self.logs())

    def log_file_path(self) -> str:
        return self._log_file

    @property
    def pid(self) -> int | None:
        if self._process and self._process.poll() is None:
            return self._process.pid
        return None

    def log_tail(self, n: int = 100) -> str:
        lines = self.logs().splitlines()
        return "\n".join(lines[-n:])

    def log_window(self, pattern: str, before: int = 10, after: int = 10) -> str:
        lines = self.logs().splitlines()
        for i, line in enumerate(lines):
            if pattern in line:
                start = max(0, i - before)
                end = min(len(lines), i + after + 1)
                return "\n".join(lines[start:end])
        return ""

    def _parse_errors(self, log_text: str) -> list[str]:
        errors = []
        for line in log_text.splitlines():
            if any(marker in line for marker in ["[ERROR]", "panicked", "SIGABRT", "thread '", "fatal"]):
                errors.append(line.strip())
        return errors

    def _parse_frontend_errors(self, log_text: str) -> list[str]:
        errors = []
        for line in log_text.splitlines():
            if "Unhandled rejection" in line or "Uncaught" in line:
                errors.append(line.strip())
        return errors

    def _parse_startup_time(self, log_text: str) -> int | None:
        start_time = None
        end_time = None
        time_pattern = re.compile(r'\[(\d{4}-\d{2}-\d{2})\]\[(\d{2}:\d{2}:\d{2})\]')

        for line in log_text.splitlines():
            match = time_pattern.search(line)
            if not match:
                continue
            ts = datetime.strptime(f"{match.group(1)} {match.group(2)}", "%Y-%m-%d %H:%M:%S")
            if "setup starting" in line and start_time is None:
                start_time = ts
            if "setup complete" in line:
                end_time = ts

        if start_time and end_time:
            delta = end_time - start_time
            return int(delta.total_seconds() * 1000)
        return None

    def _is_ready(self, log_text: str) -> bool:
        return "setup complete" in log_text
