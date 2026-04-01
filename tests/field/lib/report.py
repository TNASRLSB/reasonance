"""Test report generation (JSON + HTML + bug reports)."""

import json
import os
from datetime import datetime
from html import escape


def generate_report(results: list[dict], output_dir: str) -> tuple[str, str]:
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

    json_path = os.path.join(output_dir, f"report-{timestamp}.json")
    with open(json_path, "w") as f:
        json.dump(report_data, f, indent=2)

    html = _render_html(report_data)
    html_path = os.path.join(output_dir, f"report-{timestamp}.html")
    with open(html_path, "w") as f:
        f.write(html)

    return json_path, html_path


def _render_html(data: dict) -> str:
    """Render an HTML report from results data."""
    rows = ""
    for r in data["results"]:
        status_color = "#2e7d32" if r["status"] == "pass" else "#c62828" if r["status"] == "fail" else "#f57f17"
        errors_html = "<br>".join(escape(e) for e in r.get("errors", []))
        screenshots_html = " ".join(
            f'<a href="file://{escape(s, quote=True)}" target="_blank">[img]</a>' for s in r.get("screenshots", [])
        )
        rows += f"""<tr>
            <td>{escape(r['id'])}</td>
            <td>{escape(r['name'])}</td>
            <td>{escape(r.get('suite', ''))}</td>
            <td style="color:{status_color};font-weight:bold">{r['status'].upper()}</td>
            <td>{r.get('duration_ms', '')}ms</td>
            <td>{errors_html}</td>
            <td>{screenshots_html}</td>
            <td>{escape(r.get('notes', ''))}</td>
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


def generate_bug_report(result: dict, output_dir: str) -> str:
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

    return path
