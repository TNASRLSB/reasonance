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
