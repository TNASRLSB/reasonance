"""Input simulation via dotool (mouse, keyboard)."""

import subprocess
import time

SCREEN_WIDTH = 1920
SCREEN_HEIGHT = 1080


def _dotool(command: str):
    """Send a command to dotool via stdin pipe."""
    result = subprocess.run(
        ["dotool"],
        input=command + "\n",
        text=True,
        capture_output=True,
        timeout=5,
    )
    if result.returncode != 0:
        raise RuntimeError(f"dotool failed (exit {result.returncode}): {result.stderr}")


def click(x: int, y: int):
    """Click at absolute pixel coordinates."""
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
    """Press key or key combination."""
    _dotool(f"key {combo}")


def scroll(direction: str = "down", amount: int = 3):
    """Scroll wheel."""
    value = amount if direction == "down" else -amount
    _dotool(f"wheel {value}")


def wait(ms: int):
    """Pause between actions."""
    time.sleep(ms / 1000.0)
