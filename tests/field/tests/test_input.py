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
