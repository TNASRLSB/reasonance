# Field Test Suite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Python-based field test infrastructure for Reasonance that Claude Code uses to systematically test the app on the real desktop (KDE Wayland).

**Architecture:** Python helper library wraps system tools (spectacle, dotool, qdbus6) for screenshots, input, and window management. YAML scenario files define 67 tests + fuzzing. A CLI runner orchestrates app lifecycle and scenario execution. Claude Code acts as the AI test engine — reading screenshots, making decisions, executing actions.

**Tech Stack:** Python 3.14, PyYAML, subprocess, Pillow (image comparison), Jinja2 (HTML reports), spectacle, dotool, qdbus6

---

## File Structure

```
tests/field/
├── lib/
│   ├── __init__.py          # Package init, exports all modules
│   ├── screen.py            # Screenshot capture via spectacle, image comparison via Pillow
│   ├── input.py             # dotool wrappers: click, type, key, mouseto, wait
│   ├── window.py            # KWin D-Bus window management via qdbus6
│   ├── app.py               # Reasonance lifecycle: launch, wait_ready, kill, log parsing
│   └── report.py            # JSON + HTML report generation
├── scenarios/
│   ├── smoke.yaml           # Tests 1-7
│   ├── e2e.yaml             # Tests 8-25
│   ├── stress.yaml          # Tests 26-33
│   ├── edge.yaml            # Tests 34-44
│   ├── cross.yaml           # Tests 45-52
│   ├── security.yaml        # Tests 53-57
│   ├── visual.yaml          # Tests 58-62
│   └── integrity.yaml       # Tests 63-67
├── runner.py                # CLI entry point
├── requirements.txt         # Python dependencies
├── screenshots/
│   ├── baseline/            # (created at runtime)
│   └── runs/                # (created at runtime)
├── reports/                 # (created at runtime)
└── bugs/                    # (created at runtime)
```

---

### Task 1: Project scaffolding and screen.py

**Files:**
- Create: `tests/field/lib/__init__.py`
- Create: `tests/field/lib/screen.py`
- Create: `tests/field/requirements.txt`
- Create: `tests/field/tests/test_screen.py`

- [ ] **Step 1: Create directory structure and requirements.txt**

```bash
mkdir -p tests/field/lib tests/field/scenarios tests/field/screenshots/baseline tests/field/screenshots/runs tests/field/reports tests/field/bugs tests/field/tests
```

```
# tests/field/requirements.txt
pyyaml>=6.0
Pillow>=10.0
jinja2>=3.1
```

- [ ] **Step 2: Write test for screen.py**

```python
# tests/field/tests/test_screen.py
import os
import subprocess
from unittest.mock import patch, MagicMock
import pytest

# Add parent to path so we can import lib
import sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from lib.screen import screenshot, screenshot_active, get_screenshot_dir


class TestScreenshot:
    def test_screenshot_returns_path(self, tmp_path):
        with patch('lib.screen.subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            path = screenshot("test_shot", directory=str(tmp_path))
            assert path.endswith(".png")
            assert "test_shot" in path

    def test_screenshot_calls_spectacle_fullscreen(self, tmp_path):
        with patch('lib.screen.subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            screenshot("full", directory=str(tmp_path))
            args = mock_run.call_args[0][0]
            assert "spectacle" in args
            assert "-f" in args
            assert "-b" in args
            assert "-n" in args

    def test_screenshot_active_calls_spectacle_active(self, tmp_path):
        with patch('lib.screen.subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            screenshot_active("active", directory=str(tmp_path))
            args = mock_run.call_args[0][0]
            assert "-a" in args

    def test_screenshot_creates_timestamped_filename(self, tmp_path):
        with patch('lib.screen.subprocess.run') as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            path = screenshot("mytest", directory=str(tmp_path))
            filename = os.path.basename(path)
            # Format: HHMMSS-mytest.png
            assert filename.endswith("-mytest.png")

    def test_get_screenshot_dir_creates_run_dir(self, tmp_path):
        with patch('lib.screen.SCREENSHOTS_BASE', str(tmp_path)):
            d = get_screenshot_dir()
            assert os.path.isdir(d)
            assert "runs" in d
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd tests/field && python -m pytest tests/test_screen.py -v
```

Expected: FAIL — `ModuleNotFoundError: No module named 'lib.screen'`

- [ ] **Step 4: Implement screen.py**

```python
# tests/field/lib/__init__.py
"""Reasonance field test helpers."""

# tests/field/lib/screen.py
"""Screenshot capture via KDE Spectacle."""

import os
import subprocess
from datetime import datetime
from pathlib import Path

FIELD_DIR = Path(__file__).parent.parent
SCREENSHOTS_BASE = str(FIELD_DIR / "screenshots")


def get_screenshot_dir() -> str:
    """Get or create the current run's screenshot directory."""
    now = datetime.now().strftime("%Y-%m-%d-%H%M%S")
    run_dir = os.path.join(SCREENSHOTS_BASE, "runs", now)
    os.makedirs(run_dir, exist_ok=True)
    return run_dir


def _capture(name: str, flags: list[str], directory: str = None) -> str:
    """Internal: run spectacle with given flags, return saved path."""
    if directory is None:
        directory = get_screenshot_dir()
    os.makedirs(directory, exist_ok=True)
    timestamp = datetime.now().strftime("%H%M%S")
    filename = f"{timestamp}-{name}.png"
    path = os.path.join(directory, filename)
    cmd = ["spectacle", "-b", "-n"] + flags + ["-o", path]
    subprocess.run(cmd, capture_output=True, timeout=10)
    return path


def screenshot(name: str, directory: str = None) -> str:
    """Take full-screen screenshot, return file path."""
    return _capture(name, ["-f"], directory)


def screenshot_active(name: str, directory: str = None) -> str:
    """Screenshot active window only, return file path."""
    return _capture(name, ["-a"], directory)


def compare_images(current: str, baseline: str) -> float:
    """Perceptual similarity between two images (0.0-1.0).

    Uses average pixel difference. Requires Pillow.
    Returns 1.0 for identical images, 0.0 for completely different.
    """
    try:
        from PIL import Image
        import math

        img1 = Image.open(current).convert("RGB")
        img2 = Image.open(baseline).convert("RGB")

        # Resize to same dimensions for comparison
        size = (min(img1.width, img2.width), min(img1.height, img2.height))
        img1 = img1.resize(size)
        img2 = img2.resize(size)

        pixels1 = list(img1.getdata())
        pixels2 = list(img2.getdata())

        if len(pixels1) != len(pixels2):
            return 0.0

        total_diff = 0
        for p1, p2 in zip(pixels1, pixels2):
            total_diff += sum(abs(a - b) for a, b in zip(p1, p2))

        max_diff = len(pixels1) * 3 * 255
        similarity = 1.0 - (total_diff / max_diff)
        return similarity
    except Exception:
        return 0.0
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cd tests/field && python -m pytest tests/test_screen.py -v
```

Expected: 5 passed

- [ ] **Step 6: Commit**

```bash
git add tests/field/
git commit -m "feat(field-test): add project scaffold and screen.py helper"
```

---

### Task 2: input.py — dotool wrappers

**Files:**
- Create: `tests/field/lib/input.py`
- Create: `tests/field/tests/test_input.py`

- [ ] **Step 1: Write tests for input.py**

```python
# tests/field/tests/test_input.py
import os
import sys
from unittest.mock import patch, MagicMock, call

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from lib.input import click, click_pct, type_text, key, wait


class TestInput:
    def test_click_sends_mouseto_and_click(self):
        with patch('lib.input._dotool') as mock:
            click(500, 300)
            calls = mock.call_args_list
            # Should send mouseto as absolute coordinates converted to percentage
            assert len(calls) >= 1
            assert "click left" in calls[-1][0][0]

    def test_click_pct_sends_mouseto(self):
        with patch('lib.input._dotool') as mock:
            click_pct(0.5, 0.3)
            cmd = mock.call_args_list[0][0][0]
            assert "mouseto 0.5 0.3" in cmd

    def test_type_text_sends_type(self):
        with patch('lib.input._dotool') as mock:
            type_text("hello world")
            cmd = mock.call_args[0][0]
            assert "type hello world" in cmd

    def test_key_sends_key_command(self):
        with patch('lib.input._dotool') as mock:
            key("ctrl+p")
            cmd = mock.call_args[0][0]
            assert "key ctrl+p" in cmd

    def test_key_maps_common_names(self):
        with patch('lib.input._dotool') as mock:
            key("Return")
            cmd = mock.call_args[0][0]
            assert "key Return" in cmd

    def test_wait_sleeps(self):
        with patch('lib.input.time.sleep') as mock_sleep:
            wait(500)
            mock_sleep.assert_called_once_with(0.5)
```

- [ ] **Step 2: Run test — expect failure**

```bash
cd tests/field && python -m pytest tests/test_input.py -v
```

- [ ] **Step 3: Implement input.py**

```python
# tests/field/lib/input.py
"""Input simulation via dotool (mouse, keyboard)."""

import subprocess
import time

# Default screen resolution for absolute-to-percentage conversion
SCREEN_WIDTH = 1920
SCREEN_HEIGHT = 1080


def _dotool(command: str):
    """Send a command to dotool via stdin pipe."""
    subprocess.run(
        ["dotool"],
        input=command + "\n",
        text=True,
        capture_output=True,
        timeout=5,
    )


def click(x: int, y: int):
    """Click at absolute pixel coordinates.

    Converts to percentage for dotool mouseto, then clicks.
    """
    x_pct = x / SCREEN_WIDTH
    y_pct = y / SCREEN_HEIGHT
    _dotool(f"mouseto {x_pct:.4f} {y_pct:.4f}")
    time.sleep(0.05)
    _dotool("click left")


def click_pct(x_pct: float, y_pct: float):
    """Click at percentage coordinates (0.0-1.0)."""
    _dotool(f"mouseto {x_pct} {y_pct}")
    time.sleep(0.05)
    _dotool("click left")


def right_click(x: int, y: int):
    """Right-click at absolute pixel coordinates."""
    x_pct = x / SCREEN_WIDTH
    y_pct = y / SCREEN_HEIGHT
    _dotool(f"mouseto {x_pct:.4f} {y_pct:.4f}")
    time.sleep(0.05)
    _dotool("click right")


def double_click(x: int, y: int):
    """Double-click at absolute pixel coordinates."""
    x_pct = x / SCREEN_WIDTH
    y_pct = y / SCREEN_HEIGHT
    _dotool(f"mouseto {x_pct:.4f} {y_pct:.4f}")
    time.sleep(0.05)
    _dotool("click left")
    time.sleep(0.05)
    _dotool("click left")


def type_text(text: str, delay_ms: int = 50):
    """Type text string."""
    if delay_ms != 50:
        _dotool(f"typedelay {delay_ms}")
    _dotool(f"type {text}")


def key(combo: str):
    """Press key or key combination.

    Examples: 'ctrl+p', 'Return', 'Escape', 'ctrl+shift+s'
    """
    _dotool(f"key {combo}")


def scroll(direction: str = "down", amount: int = 3):
    """Scroll wheel. direction: 'up' or 'down'."""
    value = amount if direction == "down" else -amount
    _dotool(f"wheel {value}")


def wait(ms: int):
    """Pause between actions."""
    time.sleep(ms / 1000.0)
```

- [ ] **Step 4: Run tests**

```bash
cd tests/field && python -m pytest tests/test_input.py -v
```

Expected: 6 passed

- [ ] **Step 5: Commit**

```bash
git add tests/field/lib/input.py tests/field/tests/test_input.py
git commit -m "feat(field-test): add input.py dotool wrapper"
```

---

### Task 3: window.py — KWin D-Bus window management

**Files:**
- Create: `tests/field/lib/window.py`
- Create: `tests/field/tests/test_window.py`

- [ ] **Step 1: Write tests for window.py**

```python
# tests/field/tests/test_window.py
import os
import sys
from unittest.mock import patch, MagicMock

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from lib.window import focus, minimize_others, _build_kwin_script


class TestWindow:
    def test_build_focus_script(self):
        script = _build_kwin_script("focus", "reasonance")
        assert "resourceClass" in script
        assert "reasonance" in script
        assert "activeWindow" in script

    def test_build_minimize_others_script(self):
        script = _build_kwin_script("minimize_others", "reasonance")
        assert "minimized = true" in script
        assert "reasonance" in script

    def test_focus_calls_qdbus6(self):
        with patch('lib.window._run_kwin_script') as mock:
            mock.return_value = True
            focus("reasonance")
            mock.assert_called_once()

    def test_minimize_others_calls_qdbus6(self):
        with patch('lib.window._run_kwin_script') as mock:
            mock.return_value = True
            minimize_others("reasonance")
            mock.assert_called_once()
```

- [ ] **Step 2: Run test — expect failure**

```bash
cd tests/field && python -m pytest tests/test_window.py -v
```

- [ ] **Step 3: Implement window.py**

```python
# tests/field/lib/window.py
"""KWin D-Bus window management for KDE Wayland."""

import subprocess
import tempfile
import time

_script_counter = 0


def _build_kwin_script(action: str, resource_class: str) -> str:
    """Build a KWin JavaScript snippet for the given action."""
    if action == "focus":
        return f"""
var clients = workspace.windowList();
for (var i = 0; i < clients.length; i++) {{
    if (clients[i].resourceClass === "{resource_class}") {{
        clients[i].minimized = false;
        workspace.activeWindow = clients[i];
        break;
    }}
}}
"""
    elif action == "minimize_others":
        return f"""
var clients = workspace.windowList();
for (var i = 0; i < clients.length; i++) {{
    var c = clients[i];
    if (c.resourceClass === "{resource_class}") {{
        c.minimized = false;
        workspace.activeWindow = c;
    }} else if (c.resourceClass !== "plasmashell") {{
        c.minimized = true;
    }}
}}
"""
    elif action == "maximize":
        return f"""
var clients = workspace.windowList();
for (var i = 0; i < clients.length; i++) {{
    if (clients[i].resourceClass === "{resource_class}") {{
        clients[i].setMaximize(true, true);
        workspace.activeWindow = clients[i];
        break;
    }}
}}
"""
    elif action == "list":
        return """
var clients = workspace.windowList();
var out = [];
for (var i = 0; i < clients.length; i++) {
    var c = clients[i];
    out.push(c.caption + "|" + c.resourceClass + "|" + c.internalId);
}
print(out.join("\\n"));
"""
    else:
        raise ValueError(f"Unknown action: {action}")


def _run_kwin_script(script_content: str) -> bool:
    """Write a KWin script to a temp file, load and run it via qdbus6."""
    global _script_counter
    _script_counter += 1
    name = f"field_test_{_script_counter}"

    with tempfile.NamedTemporaryFile(mode='w', suffix='.js', delete=False) as f:
        f.write(script_content)
        script_path = f.name

    try:
        result = subprocess.run(
            ["qdbus6", "org.kde.KWin", "/Scripting", "loadScript", script_path, name],
            capture_output=True, text=True, timeout=5,
        )
        script_id = result.stdout.strip()
        if not script_id or script_id == "-1":
            return False

        subprocess.run(
            ["qdbus6", "org.kde.KWin", f"/Scripting/Script{script_id}", "org.kde.kwin.Script.run"],
            capture_output=True, text=True, timeout=5,
        )
        time.sleep(0.5)
        return True
    except Exception:
        return False
    finally:
        import os
        os.unlink(script_path)


def focus(resource_class: str):
    """Bring window to front and give it focus."""
    script = _build_kwin_script("focus", resource_class)
    _run_kwin_script(script)


def minimize_others(keep: str):
    """Minimize all windows except the specified one."""
    script = _build_kwin_script("minimize_others", keep)
    _run_kwin_script(script)


def maximize(resource_class: str):
    """Maximize the specified window."""
    script = _build_kwin_script("maximize", resource_class)
    _run_kwin_script(script)


def get_geometry(resource_class: str) -> dict:
    """Return {x, y, width, height} of window via qdbus6 queryWindowInfo.

    Note: queryWindowInfo returns info about the currently active window.
    Focus the target first.
    """
    focus(resource_class)
    time.sleep(0.3)
    result = subprocess.run(
        ["qdbus6", "org.kde.KWin", "/KWin", "org.kde.KWin.queryWindowInfo"],
        capture_output=True, text=True, timeout=5,
    )
    info = {}
    for line in result.stdout.splitlines():
        if ":" in line:
            k, v = line.split(":", 1)
            info[k.strip()] = v.strip()
    return {
        "x": int(info.get("x", 0)),
        "y": int(info.get("y", 0)),
        "width": int(info.get("width", 1280)),
        "height": int(info.get("height", 800)),
    }
```

- [ ] **Step 4: Run tests**

```bash
cd tests/field && python -m pytest tests/test_window.py -v
```

Expected: 4 passed

- [ ] **Step 5: Commit**

```bash
git add tests/field/lib/window.py tests/field/tests/test_window.py
git commit -m "feat(field-test): add window.py KWin D-Bus management"
```

---

### Task 4: app.py — Reasonance lifecycle and log parsing

**Files:**
- Create: `tests/field/lib/app.py`
- Create: `tests/field/tests/test_app.py`

- [ ] **Step 1: Write tests for app.py**

```python
# tests/field/tests/test_app.py
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
```

- [ ] **Step 2: Run test — expect failure**

```bash
cd tests/field && python -m pytest tests/test_app.py -v
```

- [ ] **Step 3: Implement app.py**

```python
# tests/field/lib/app.py
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
        self._log_file = f"/tmp/reasonance-field-{datetime.now().strftime('%H%M%S')}.log"
        self._launch_time = None

    def launch(self, env: dict = None):
        """Start Reasonance via npx tauri dev with logging."""
        launch_env = os.environ.copy()
        launch_env["RUST_LOG"] = "info"
        launch_env["RUST_BACKTRACE"] = "1"
        if env:
            launch_env.update(env)

        log_fd = open(self._log_file, "w")
        self._launch_time = time.time()
        self._process = subprocess.Popen(
            ["npx", "tauri", "dev"],
            cwd=str(PROJECT_ROOT),
            stdout=log_fd,
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
                return False  # Process exited
            time.sleep(2)
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

    def is_running(self) -> bool:
        """Check if the app process is still running."""
        return self._process is not None and self._process.poll() is None

    def logs(self) -> str:
        """Return full log file content."""
        try:
            with open(self._log_file, "r") as f:
                return f.read()
        except FileNotFoundError:
            return ""

    def get_errors(self) -> list[str]:
        """Extract error lines from backend logs."""
        return self._parse_errors(self.logs())

    def get_frontend_errors(self) -> list[str]:
        """Extract frontend unhandled rejection errors."""
        return self._parse_frontend_errors(self.logs())

    def startup_time_ms(self) -> int | None:
        """Milliseconds from launch to setup complete."""
        return self._parse_startup_time(self.logs())

    def log_file_path(self) -> str:
        """Return path to the log file for external inspection."""
        return self._log_file

    # --- Internal parsing methods (testable without launching app) ---

    def _parse_errors(self, log_text: str) -> list[str]:
        """Extract ERROR, WARN, and panic lines."""
        errors = []
        for line in log_text.splitlines():
            if any(marker in line for marker in ["[ERROR]", "panicked", "SIGABRT", "thread '", "fatal"]):
                errors.append(line.strip())
        return errors

    def _parse_frontend_errors(self, log_text: str) -> list[str]:
        """Extract Vite-reported frontend errors."""
        errors = []
        for line in log_text.splitlines():
            if "Unhandled rejection" in line or "Uncaught" in line:
                errors.append(line.strip())
        return errors

    def _parse_startup_time(self, log_text: str) -> int | None:
        """Parse milliseconds between 'setup starting' and 'setup complete'."""
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
        """Check if setup completed successfully."""
        return "setup complete" in log_text
```

- [ ] **Step 4: Run tests**

```bash
cd tests/field && python -m pytest tests/test_app.py -v
```

Expected: 5 passed

- [ ] **Step 5: Commit**

```bash
git add tests/field/lib/app.py tests/field/tests/test_app.py
git commit -m "feat(field-test): add app.py Reasonance lifecycle manager"
```

---

### Task 5: report.py — JSON + HTML report generation

**Files:**
- Create: `tests/field/lib/report.py`
- Create: `tests/field/tests/test_report.py`

- [ ] **Step 1: Write tests**

```python
# tests/field/tests/test_report.py
import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from lib.report import generate_report, generate_bug_report

SAMPLE_RESULTS = [
    {
        "id": "smoke_01",
        "name": "App startup",
        "suite": "smoke",
        "status": "pass",
        "duration_ms": 3200,
        "screenshots": ["/tmp/shot1.png"],
        "errors": [],
        "notes": "",
    },
    {
        "id": "smoke_02",
        "name": "Open project",
        "suite": "smoke",
        "status": "fail",
        "duration_ms": 5100,
        "screenshots": ["/tmp/shot2.png", "/tmp/shot3.png"],
        "errors": ["Unhandled rejection: [object Object]"],
        "notes": "File tree did not populate",
    },
]


class TestReport:
    def test_generate_report_creates_json(self, tmp_path):
        generate_report(SAMPLE_RESULTS, str(tmp_path))
        json_files = list(tmp_path.glob("*.json"))
        assert len(json_files) == 1
        data = json.loads(json_files[0].read_text())
        assert data["total"] == 2
        assert data["passed"] == 1
        assert data["failed"] == 1

    def test_generate_report_creates_html(self, tmp_path):
        generate_report(SAMPLE_RESULTS, str(tmp_path))
        html_files = list(tmp_path.glob("*.html"))
        assert len(html_files) == 1
        html = html_files[0].read_text()
        assert "smoke_01" in html
        assert "smoke_02" in html
        assert "FAIL" in html.upper()

    def test_generate_bug_report_creates_markdown(self, tmp_path):
        generate_bug_report(SAMPLE_RESULTS[1], str(tmp_path))
        md_files = list(tmp_path.glob("*.md"))
        assert len(md_files) == 1
        md = md_files[0].read_text()
        assert "smoke_02" in md
        assert "Unhandled rejection" in md
        assert "Steps to Reproduce" in md
```

- [ ] **Step 2: Run test — expect failure**

```bash
cd tests/field && python -m pytest tests/test_report.py -v
```

- [ ] **Step 3: Implement report.py**

```python
# tests/field/lib/report.py
"""Test report generation (JSON + HTML + bug reports)."""

import json
import os
from datetime import datetime


def generate_report(results: list[dict], output_dir: str):
    """Generate JSON and HTML reports from test results."""
    os.makedirs(output_dir, exist_ok=True)
    timestamp = datetime.now().strftime("%Y-%m-%d-%H%M%S")

    passed = sum(1 for r in results if r["status"] == "pass")
    failed = sum(1 for r in results if r["status"] == "fail")
    skipped = sum(1 for r in results if r["status"] == "skip")

    report_data = {
        "timestamp": timestamp,
        "total": len(results),
        "passed": passed,
        "failed": failed,
        "skipped": skipped,
        "results": results,
    }

    # JSON report
    json_path = os.path.join(output_dir, f"report-{timestamp}.json")
    with open(json_path, "w") as f:
        json.dump(report_data, f, indent=2)

    # HTML report
    html = _render_html(report_data)
    html_path = os.path.join(output_dir, f"report-{timestamp}.html")
    with open(html_path, "w") as f:
        f.write(html)


def _render_html(data: dict) -> str:
    """Render an HTML report from results data."""
    rows = ""
    for r in data["results"]:
        status_color = "#2e7d32" if r["status"] == "pass" else "#c62828" if r["status"] == "fail" else "#f57f17"
        errors_html = "<br>".join(r.get("errors", []))
        screenshots_html = " ".join(
            f'<a href="file://{s}" target="_blank">[img]</a>' for s in r.get("screenshots", [])
        )
        rows += f"""<tr>
            <td>{r['id']}</td>
            <td>{r['name']}</td>
            <td>{r.get('suite', '')}</td>
            <td style="color:{status_color};font-weight:bold">{r['status'].upper()}</td>
            <td>{r.get('duration_ms', '')}ms</td>
            <td>{errors_html}</td>
            <td>{screenshots_html}</td>
            <td>{r.get('notes', '')}</td>
        </tr>"""

    return f"""<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>Reasonance Field Test Report</title>
<style>
body {{ font-family: system-ui; margin: 2em; background: #fafafa; }}
table {{ border-collapse: collapse; width: 100%; }}
th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; font-size: 14px; }}
th {{ background: #333; color: white; }}
tr:nth-child(even) {{ background: #f2f2f2; }}
.summary {{ display: flex; gap: 2em; margin-bottom: 1em; }}
.summary div {{ padding: 1em; border-radius: 8px; font-size: 1.2em; font-weight: bold; }}
.pass {{ background: #e8f5e9; color: #2e7d32; }}
.fail {{ background: #ffebee; color: #c62828; }}
.skip {{ background: #fff8e1; color: #f57f17; }}
</style></head><body>
<h1>Reasonance Field Test Report</h1>
<p>{data['timestamp']}</p>
<div class="summary">
    <div class="pass">PASS: {data['passed']}</div>
    <div class="fail">FAIL: {data['failed']}</div>
    <div class="skip">SKIP: {data['skipped']}</div>
    <div>TOTAL: {data['total']}</div>
</div>
<table>
<tr><th>ID</th><th>Name</th><th>Suite</th><th>Status</th><th>Duration</th><th>Errors</th><th>Screenshots</th><th>Notes</th></tr>
{rows}
</table></body></html>"""


def generate_bug_report(result: dict, output_dir: str):
    """Generate a markdown bug report for a failed test."""
    os.makedirs(output_dir, exist_ok=True)
    bug_id = result["id"].replace("_", "-")
    name_slug = result["name"].lower().replace(" ", "-")[:40]
    filename = f"BUG-{bug_id}-{name_slug}.md"
    path = os.path.join(output_dir, filename)

    errors_block = "\n".join(result.get("errors", ["(no errors captured)"]))
    screenshots_block = "\n".join(
        f"![{os.path.basename(s)}]({s})" for s in result.get("screenshots", [])
    )

    md = f"""# BUG: {result['name']}

**Severity:** High
**Found by:** Test {result['id']} — {result['name']}
**Suite:** {result.get('suite', 'unknown')}
**Date:** {datetime.now().strftime('%Y-%m-%d')}

## Steps to Reproduce

Test scenario {result['id']} triggered this failure.

## Expected Behavior

Test should pass all criteria.

## Actual Behavior

{result.get('notes', 'Test failed.')}

## Screenshots

{screenshots_block}

## Relevant Logs

```
{errors_block}
```

## Probable Cause

To be determined during investigation.
"""

    with open(path, "w") as f:
        f.write(md)
```

- [ ] **Step 4: Run tests**

```bash
cd tests/field && python -m pytest tests/test_report.py -v
```

Expected: 3 passed

- [ ] **Step 5: Commit**

```bash
git add tests/field/lib/report.py tests/field/tests/test_report.py
git commit -m "feat(field-test): add report.py JSON + HTML + bug report generator"
```

---

### Task 6: runner.py — CLI test orchestrator

**Files:**
- Create: `tests/field/runner.py`

- [ ] **Step 1: Implement runner.py**

```python
#!/usr/bin/env python3
"""Reasonance Field Test Runner.

Usage:
    python runner.py --all                    Run all test suites
    python runner.py --suite smoke            Run specific suite
    python runner.py --test smoke_01          Run single test
    python runner.py --fuzz --duration 30     Fuzzing mode (minutes)
    python runner.py --update-baseline        Update visual baselines
    python runner.py --list                   List all scenarios
"""

import argparse
import json
import os
import sys
import time
import yaml
from datetime import datetime
from pathlib import Path

from lib.app import ReasonanceApp
from lib.screen import screenshot, get_screenshot_dir
from lib.window import focus, minimize_others
from lib.report import generate_report, generate_bug_report

FIELD_DIR = Path(__file__).parent
SCENARIOS_DIR = FIELD_DIR / "scenarios"
REPORTS_DIR = FIELD_DIR / "reports"
BUGS_DIR = FIELD_DIR / "bugs"

SUITES = ["smoke", "e2e", "stress", "edge", "cross", "security", "visual", "integrity"]


def load_scenarios(suite: str = None, test_id: str = None) -> list[dict]:
    """Load test scenarios from YAML files."""
    scenarios = []
    files = [SCENARIOS_DIR / f"{s}.yaml" for s in SUITES] if suite is None else [SCENARIOS_DIR / f"{suite}.yaml"]

    for f in files:
        if not f.exists():
            print(f"Warning: {f} not found, skipping")
            continue
        with open(f) as fh:
            data = yaml.safe_load(fh)
            if data:
                scenarios.extend(data)

    if test_id:
        scenarios = [s for s in scenarios if s["id"] == test_id]

    return scenarios


def list_scenarios():
    """Print all available test scenarios."""
    for suite in SUITES:
        scenarios = load_scenarios(suite=suite)
        if scenarios:
            print(f"\n--- {suite.upper()} ({len(scenarios)} tests) ---")
            for s in scenarios:
                llm = " [LLM]" if s.get("requires_llm") else ""
                print(f"  {s['id']}: {s['name']}{llm}")


def run_scenario(scenario: dict, app: ReasonanceApp, screenshot_dir: str) -> dict:
    """Execute a single test scenario.

    This function captures the mechanical parts. The actual test logic
    (analyzing screenshots, deciding actions) is handled by Claude Code
    when it reads and executes this runner interactively.
    """
    start = time.time()
    result = {
        "id": scenario["id"],
        "name": scenario["name"],
        "suite": scenario.get("suite", "unknown"),
        "status": "pending",
        "duration_ms": 0,
        "screenshots": [],
        "errors": [],
        "notes": "",
    }

    try:
        # Take initial screenshot
        shot = screenshot(f"{scenario['id']}-before", directory=screenshot_dir)
        result["screenshots"].append(shot)

        # The actual test steps are executed by Claude Code interactively.
        # This runner provides the framework; Claude provides the intelligence.
        #
        # When running non-interactively, we do basic log-based checks:
        errors = app.get_errors()
        frontend_errors = app.get_frontend_errors()

        if errors:
            result["errors"].extend(errors)
        if frontend_errors:
            result["errors"].extend(frontend_errors)

        # Take final screenshot
        shot = screenshot(f"{scenario['id']}-after", directory=screenshot_dir)
        result["screenshots"].append(shot)

        # Basic auto-pass: no errors = pass (Claude overrides this with visual judgment)
        result["status"] = "fail" if result["errors"] else "pass"

    except Exception as e:
        result["status"] = "fail"
        result["errors"].append(str(e))

    result["duration_ms"] = int((time.time() - start) * 1000)
    return result


def main():
    parser = argparse.ArgumentParser(description="Reasonance Field Test Runner")
    parser.add_argument("--all", action="store_true", help="Run all suites")
    parser.add_argument("--suite", type=str, help="Run specific suite")
    parser.add_argument("--test", type=str, help="Run single test by ID")
    parser.add_argument("--fuzz", action="store_true", help="Run fuzzing mode")
    parser.add_argument("--duration", type=int, default=30, help="Fuzzing duration in minutes")
    parser.add_argument("--update-baseline", action="store_true", help="Update visual baselines")
    parser.add_argument("--list", action="store_true", help="List all scenarios")
    parser.add_argument("--no-launch", action="store_true", help="Don't launch app (assume already running)")
    args = parser.parse_args()

    if args.list:
        list_scenarios()
        return

    # Determine which scenarios to run
    if args.test:
        scenarios = load_scenarios(test_id=args.test)
    elif args.suite:
        scenarios = load_scenarios(suite=args.suite)
    elif args.all:
        scenarios = load_scenarios()
    elif args.fuzz:
        print(f"Fuzzing mode: {args.duration} minutes")
        print("Launch Reasonance and use Claude Code interactively for fuzzing.")
        return
    else:
        parser.print_help()
        return

    if not scenarios:
        print("No scenarios found.")
        return

    print(f"Running {len(scenarios)} test(s)...")

    # Setup
    app = ReasonanceApp()
    screenshot_dir = get_screenshot_dir()
    results = []

    if not args.no_launch:
        print("Launching Reasonance...")
        app.launch()
        if not app.wait_ready(timeout=120):
            print("ERROR: App failed to start!")
            print(app.logs()[-500:])
            return

        print("App ready. Preparing window...")
        time.sleep(2)
        focus("reasonance")
        minimize_others("reasonance")
        time.sleep(1)

    # Run scenarios
    for scenario in scenarios:
        print(f"  [{scenario['id']}] {scenario['name']}...", end=" ", flush=True)

        if scenario.get("requires_llm") and not os.environ.get("FIELD_TEST_LLM"):
            print("SKIP (requires LLM)")
            results.append({
                "id": scenario["id"],
                "name": scenario["name"],
                "suite": scenario.get("suite", "unknown"),
                "status": "skip",
                "duration_ms": 0,
                "screenshots": [],
                "errors": [],
                "notes": "Skipped: requires_llm=true, set FIELD_TEST_LLM=1 to enable",
            })
            continue

        result = run_scenario(scenario, app, screenshot_dir)
        results.append(result)
        print(result["status"].upper())

    # Cleanup
    if not args.no_launch:
        print("Stopping Reasonance...")
        app.kill()

    # Generate reports
    os.makedirs(str(REPORTS_DIR), exist_ok=True)
    os.makedirs(str(BUGS_DIR), exist_ok=True)
    generate_report(results, str(REPORTS_DIR))

    # Generate bug reports for failures
    for r in results:
        if r["status"] == "fail":
            generate_bug_report(r, str(BUGS_DIR))

    # Summary
    passed = sum(1 for r in results if r["status"] == "pass")
    failed = sum(1 for r in results if r["status"] == "fail")
    skipped = sum(1 for r in results if r["status"] == "skip")
    print(f"\nResults: {passed} passed, {failed} failed, {skipped} skipped / {len(results)} total")
    print(f"Screenshots: {screenshot_dir}")
    print(f"Report: {REPORTS_DIR}")
    if failed:
        print(f"Bug reports: {BUGS_DIR}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Verify runner loads**

```bash
cd tests/field && python runner.py --list
```

Expected: lists all scenarios (or "Warning: ... not found" if YAML files don't exist yet)

- [ ] **Step 3: Commit**

```bash
git add tests/field/runner.py
git commit -m "feat(field-test): add runner.py CLI orchestrator"
```

---

### Task 7: Smoke test scenarios (YAML)

**Files:**
- Create: `tests/field/scenarios/smoke.yaml`

- [ ] **Step 1: Write smoke.yaml**

Copy the 7 smoke test scenarios from the spec verbatim into `tests/field/scenarios/smoke.yaml`:

```yaml
# tests/field/scenarios/smoke.yaml
- id: smoke_01
  name: App startup
  suite: smoke
  steps:
    - action: "Launch Reasonance with RUST_LOG=info"
      verify: "App window appears, no panic in logs"
  pass_criteria:
    - "No panic in backend logs"
    - "No 'Unhandled rejection' in frontend"
    - "Startup completes within 30 seconds"
    - "Window appears and is visible"

- id: smoke_02
  name: Open project
  suite: smoke
  steps:
    - action: "Click 'APRI CARTELLA' button"
      verify: "File dialog opens"
    - action: "Select /home/uh1/VIBEPROJECTS/REASONANCE"
      verify: "File tree populates with project files"
  pass_criteria:
    - "File tree shows src/, src-tauri/, package.json"
    - "No red error banner"
    - "No errors in backend logs"

- id: smoke_03
  name: Open file
  suite: smoke
  steps:
    - action: "Click on a .ts file in the file tree (e.g. src/lib/adapter/tauri.ts)"
      verify: "Editor tab opens with syntax highlighting"
  pass_criteria:
    - "Editor tab shows file name"
    - "Content visible with syntax highlighting"
    - "No errors"

- id: smoke_04
  name: Open terminal
  suite: smoke
  steps:
    - action: "Click on an LLM provider in the terminal panel (e.g. CLAUDE)"
      verify: "Terminal/chat session initializes"
  pass_criteria:
    - "Terminal panel shows session or LLM selection"
    - "No crash or error"

- id: smoke_05
  name: Open each panel
  suite: smoke
  steps:
    - action: "Click SETTINGS button in toolbar"
      verify: "Settings panel opens"
    - action: "Click ANALYTICS button"
      verify: "Analytics panel opens"
    - action: "Click MEMORY button"
      verify: "Memory panel opens"
    - action: "Click HIVE button"
      verify: "HIVE canvas opens"
    - action: "Click GIT button"
      verify: "Git panel opens"
  pass_criteria:
    - "Each panel opens without crash"
    - "Each panel shows relevant content"

- id: smoke_06
  name: Theme switch
  suite: smoke
  steps:
    - action: "Open Settings panel"
      verify: "Settings visible"
    - action: "Find theme selector and switch theme"
      verify: "Theme changes visually"
  pass_criteria:
    - "Theme changes without flash of unstyled content"
    - "All components update consistently"
    - "No errors"

- id: smoke_07
  name: State persistence
  suite: smoke
  steps:
    - action: "Note current state (open files, active tab, panel sizes)"
      verify: "State captured"
    - action: "Kill the app"
      verify: "App terminates"
    - action: "Relaunch the app"
      verify: "App starts"
    - action: "Verify state restored"
      verify: "Previous state matches"
  pass_criteria:
    - "Last project auto-restored"
    - "Previously open files restored in tabs"
    - "Panel sizes approximately restored"
```

- [ ] **Step 2: Verify YAML loads**

```bash
cd tests/field && python -c "import yaml; print(len(yaml.safe_load(open('scenarios/smoke.yaml'))))"
```

Expected: `7`

- [ ] **Step 3: Commit**

```bash
git add tests/field/scenarios/smoke.yaml
git commit -m "feat(field-test): add smoke test scenarios (7 tests)"
```

---

### Task 8: E2E test scenarios (YAML)

**Files:**
- Create: `tests/field/scenarios/e2e.yaml`

- [ ] **Step 1: Write e2e.yaml**

Create the file with all 18 E2E scenarios (tests 8-25) from the spec. Each scenario has id, name, suite, steps (action + verify pairs), pass_criteria, and optional requires_llm/providers fields.

The complete file should contain scenarios for: file tree navigation (8), editor full workflow (9), chat with real LLM (10), terminal PTY (11), workflow HIVE (12), file operations with undo (13), git integration (14), permissions (15), agent memory (16), settings full (17), analytics (18), search (19), i18n (20), accessibility (21), PTY resilience (22), circuit breaker (23), CLI updater (24), state persistence full (25).

Follow the exact same YAML structure as smoke.yaml. For tests 10, 22, 23, 30, 42, 49, 52: set `requires_llm: true`.

- [ ] **Step 2: Verify YAML loads**

```bash
cd tests/field && python -c "import yaml; print(len(yaml.safe_load(open('scenarios/e2e.yaml'))))"
```

Expected: `18`

- [ ] **Step 3: Commit**

```bash
git add tests/field/scenarios/e2e.yaml
git commit -m "feat(field-test): add E2E test scenarios (18 tests)"
```

---

### Task 9: Stress + Edge + Cross scenarios (YAML)

**Files:**
- Create: `tests/field/scenarios/stress.yaml` (8 tests: 26-33)
- Create: `tests/field/scenarios/edge.yaml` (11 tests: 34-44)
- Create: `tests/field/scenarios/cross.yaml` (8 tests: 45-52)

- [ ] **Step 1: Write stress.yaml, edge.yaml, cross.yaml**

Create all three files following the same YAML structure. Copy scenarios from the spec:
- stress.yaml: 50+ files (26), large file tree (27), large file (28), PTY volume (29), concurrent agents (30), memory leak (31), rapid UI (32), startup benchmark (33)
- edge.yaml: binary file (34), unicode (35), long lines (36), empty project (37), deep dirs (38), symlinks (39), external modification (40), corrupt config (41), network disconnect (42), permission denied (43), no-git project (44)
- cross.yaml: edit→git (45), delete→editor (46), rename→editor (47), settings→live (48), agent→filesystem (49), workflow→terminal (50), theme consistency (51), multi-provider normalizer (52)

- [ ] **Step 2: Verify all load**

```bash
cd tests/field && python -c "
import yaml
for f in ['stress', 'edge', 'cross']:
    data = yaml.safe_load(open(f'scenarios/{f}.yaml'))
    print(f'{f}: {len(data)} tests')
"
```

Expected: `stress: 8`, `edge: 11`, `cross: 8`

- [ ] **Step 3: Commit**

```bash
git add tests/field/scenarios/stress.yaml tests/field/scenarios/edge.yaml tests/field/scenarios/cross.yaml
git commit -m "feat(field-test): add stress, edge case, and cross-feature scenarios (27 tests)"
```

---

### Task 10: Security + Visual + Integrity scenarios (YAML)

**Files:**
- Create: `tests/field/scenarios/security.yaml` (5 tests: 53-57)
- Create: `tests/field/scenarios/visual.yaml` (5 tests: 58-62)
- Create: `tests/field/scenarios/integrity.yaml` (5 tests: 63-67)

- [ ] **Step 1: Write security.yaml, visual.yaml, integrity.yaml**

Create all three files following the same YAML structure. Copy scenarios from the spec:
- security.yaml: workspace trust (53), permission deny (54), policy regex (55), symlink escape (56), prompt injection (57)
- visual.yaml: baseline screenshots (58), dark theme (59), light theme (60), responsive layout (61), error states (62)
- integrity.yaml: kill during save (63), transaction semantics (64), full state restore (65), config migration (66), concurrent writes (67)

- [ ] **Step 2: Verify all load**

```bash
cd tests/field && python -c "
import yaml
for f in ['security', 'visual', 'integrity']:
    data = yaml.safe_load(open(f'scenarios/{f}.yaml'))
    print(f'{f}: {len(data)} tests')
"
```

Expected: `security: 5`, `visual: 5`, `integrity: 5`

- [ ] **Step 3: Verify total count**

```bash
cd tests/field && python runner.py --list 2>&1 | tail -5
```

Should list 67 total tests across 8 suites.

- [ ] **Step 4: Commit**

```bash
git add tests/field/scenarios/
git commit -m "feat(field-test): add security, visual, and integrity scenarios (15 tests)"
```

---

### Task 11: Integration test — run smoke suite end-to-end

**Files:**
- No new files — this task verifies everything works together

- [ ] **Step 1: Install Python dependencies**

```bash
cd tests/field && pip install -r requirements.txt
```

- [ ] **Step 2: Run unit tests for all helpers**

```bash
cd tests/field && python -m pytest tests/ -v
```

Expected: all tests pass (screen, input, window, app, report)

- [ ] **Step 3: Run the runner in list mode**

```bash
cd tests/field && python runner.py --list
```

Expected: 67 tests listed across 8 suites

- [ ] **Step 4: Run smoke suite with live app**

```bash
cd tests/field && python runner.py --suite smoke
```

Expected: app launches, 7 smoke tests run, report generated. Some may fail (known bug #2) but the infrastructure works.

- [ ] **Step 5: Check generated report**

```bash
ls tests/field/reports/
```

Expected: JSON + HTML report files

- [ ] **Step 6: Final commit**

```bash
git add tests/field/
git commit -m "feat(field-test): complete field test infrastructure — 67 scenarios, 5 helper modules, CLI runner"
```
