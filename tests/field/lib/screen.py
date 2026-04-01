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
    result = subprocess.run(cmd, capture_output=True, timeout=10)
    if result.returncode != 0:
        raise RuntimeError(
            f"spectacle failed (exit {result.returncode}): {result.stderr.decode()}"
        )
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

        img1 = Image.open(current).convert("RGB")
        img2 = Image.open(baseline).convert("RGB")

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
    except ImportError:
        raise
    except Exception as e:
        import logging
        logging.getLogger(__name__).warning("compare_images failed: %s", e)
        return 0.0
