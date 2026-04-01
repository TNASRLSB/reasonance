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
