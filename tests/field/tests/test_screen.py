import os
import subprocess
from unittest.mock import patch, MagicMock
import pytest

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
