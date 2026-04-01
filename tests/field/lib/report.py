"""Test report generation (JSON + HTML + bug reports)."""

import base64
import json
import os
from datetime import datetime
from html import escape


def generate_report(
    results: list[dict],
    output_dir: str,
    telemetry_summary: dict = None,
    timeline: list[dict] = None,
) -> tuple[str, str]:
    """Generate JSON and HTML reports from test results."""
    os.makedirs(output_dir, exist_ok=True)
    timestamp = datetime.now().strftime("%Y-%m-%d-%H%M%S")

    passed = sum(1 for r in results if r["status"] == "pass")
    failed = sum(1 for r in results if r["status"] == "fail")
    skipped = sum(1 for r in results if r["status"] == "skip")

    total_duration_ms = sum(r.get("duration_ms", 0) for r in results)

    report_data = {
        "timestamp": timestamp,
        "total": len(results),
        "passed": passed,
        "failed": failed,
        "skipped": skipped,
        "total_duration_ms": total_duration_ms,
        "telemetry": telemetry_summary or {},
        "timeline": timeline or [],
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


def _screenshot_thumb(path: str) -> str:
    """Return <img> with base64 data if file < 500KB, else <a> link."""
    try:
        size = os.path.getsize(path)
    except OSError:
        return f'<a href="file://{escape(path, quote=True)}" target="_blank">[img]</a>'

    if size < 500 * 1024:
        try:
            with open(path, "rb") as f:
                data = f.read()
            b64 = base64.b64encode(data).decode("ascii")
            ext = os.path.splitext(path)[1].lstrip(".").lower() or "png"
            mime = "image/jpeg" if ext in ("jpg", "jpeg") else f"image/{ext}"
            safe_path = escape(path, quote=True)
            return (
                f'<a href="file://{safe_path}" target="_blank">'
                f'<img src="data:{mime};base64,{b64}" '
                f'style="max-width:80px;max-height:60px;border:1px solid #444;border-radius:3px;" '
                f'title="{escape(os.path.basename(path))}">'
                f"</a>"
            )
        except OSError:
            pass

    safe_path = escape(path, quote=True)
    return f'<a href="file://{safe_path}" target="_blank">[img]</a>'


def _render_html(data: dict) -> str:
    """Render an HTML report (v2, dark theme) from results data."""
    # --- results table rows ---
    rows = ""
    for r in data["results"]:
        status = r["status"]
        if status == "pass":
            status_color = "#4caf50"
        elif status == "fail":
            status_color = "#ef5350"
        else:
            status_color = "#ffa726"

        errors_html = "<br>".join(escape(e) for e in r.get("errors", []))

        screenshots = r.get("screenshots", [])
        screenshots_html = " ".join(_screenshot_thumb(s) for s in screenshots)

        steps_done = r.get("steps_executed", "")
        steps_total = r.get("steps_total", "")
        steps_cell = (
            f"{steps_done}/{steps_total}"
            if steps_done != "" and steps_total != ""
            else str(steps_done or steps_total or "")
        )

        duration = r.get("duration_ms", "")
        duration_cell = f"{duration}ms" if duration != "" else ""

        rows += f"""<tr>
            <td>{escape(str(r['id']))}</td>
            <td>{escape(r['name'])}</td>
            <td>{escape(r.get('suite', ''))}</td>
            <td style="color:{status_color};font-weight:bold">{status.upper()}</td>
            <td>{steps_cell}</td>
            <td>{duration_cell}</td>
            <td class="err-cell">{errors_html}</td>
            <td class="thumb-cell">{screenshots_html}</td>
        </tr>"""

    # --- timeline section ---
    tl_events = data.get("timeline", [])[-200:]
    timeline_rows = ""
    for ev in tl_events:
        name = escape(str(ev.get("event", ev.get("name", ""))))
        ev_data = ev.get("data", ev.get("payload", {}))
        snippet = escape(json.dumps(ev_data, default=str)[:120])
        ts = escape(str(ev.get("ts", ev.get("timestamp", ""))))
        timeline_rows += f"<tr><td>{ts}</td><td>{name}</td><td>{snippet}</td></tr>"

    timeline_section = ""
    if timeline_rows:
        timeline_section = f"""
<details>
  <summary style="cursor:pointer;font-size:1.1em;font-weight:bold;color:#90caf9;margin:1.5em 0 0.5em;">
    Timeline (last {len(tl_events)} events)
  </summary>
  <table>
    <tr><th>Timestamp</th><th>Event</th><th>Data snippet</th></tr>
    {timeline_rows}
  </table>
</details>"""

    # --- telemetry / summary bar extras ---
    telemetry = data.get("telemetry") or {}
    rss_peak = telemetry.get("rss_peak_mb", "")
    rss_cell = f'<div class="stat">RSS Peak: {rss_peak} MB</div>' if rss_peak != "" else ""
    total_ms = data.get("total_duration_ms", "")
    dur_cell = f'<div class="stat">Duration: {total_ms} ms</div>' if total_ms != "" else ""

    return f"""<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8">
<title>Reasonance Field Test Report</title>
<style>
*, *::before, *::after {{ box-sizing: border-box; }}
body {{
  font-family: system-ui, sans-serif;
  margin: 2em;
  background: #1a1a2e;
  color: #eee;
}}
h1 {{ color: #90caf9; margin-bottom: 0.25em; }}
p.ts {{ color: #888; margin-top: 0; }}
.summary {{
  display: flex;
  flex-wrap: wrap;
  gap: 1em;
  margin-bottom: 1.5em;
  align-items: center;
}}
.summary div {{
  padding: 0.6em 1.2em;
  border-radius: 8px;
  font-size: 1.05em;
  font-weight: bold;
}}
.pass {{ background: #1b5e20; color: #a5d6a7; }}
.fail {{ background: #b71c1c; color: #ef9a9a; }}
.skip {{ background: #e65100; color: #ffcc80; }}
.stat {{ background: #263238; color: #b0bec5; }}
table {{
  border-collapse: collapse;
  width: 100%;
  margin-bottom: 1.5em;
}}
th, td {{
  border: 1px solid #37474f;
  padding: 7px 10px;
  text-align: left;
  font-size: 13px;
  vertical-align: top;
}}
th {{
  background: #0d47a1;
  color: #e3f2fd;
  font-size: 13px;
}}
tr:nth-child(even) {{ background: #1e2a3a; }}
tr:nth-child(odd) {{ background: #162032; }}
.err-cell {{ color: #ef9a9a; font-size: 12px; max-width: 260px; word-break: break-word; }}
.thumb-cell {{ white-space: nowrap; }}
details summary::-webkit-details-marker {{ color: #90caf9; }}
</style>
</head><body>
<h1>Reasonance Field Test Report</h1>
<p class="ts">{data['timestamp']}</p>
<div class="summary">
  <div class="pass">PASS: {data['passed']}</div>
  <div class="fail">FAIL: {data['failed']}</div>
  <div class="skip">SKIP: {data['skipped']}</div>
  <div class="stat">TOTAL: {data['total']}</div>
  {rss_cell}
  {dur_cell}
</div>
<table>
<tr>
  <th>ID</th><th>Name</th><th>Suite</th><th>Status</th>
  <th>Steps</th><th>Duration</th><th>Errors</th><th>Screenshots</th>
</tr>
{rows}
</table>
{timeline_section}
</body></html>"""


def generate_bug_report(
    result: dict,
    output_dir: str,
    log_excerpt: str = "",
    timeline_excerpt: list[dict] = None,
) -> str:
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

    duration = result.get("duration_ms", "")
    duration_line = f"**Duration:** {duration} ms\n" if duration != "" else ""

    steps_done = result.get("steps_executed", "")
    steps_total = result.get("steps_total", "")
    if steps_done != "" or steps_total != "":
        steps_line = f"**Steps executed:** {steps_done}/{steps_total}\n"
    else:
        steps_line = ""

    # Timeline excerpt section
    timeline_section = ""
    if timeline_excerpt:
        lines = []
        for ev in timeline_excerpt:
            name = ev.get("event", ev.get("name", ""))
            ts = ev.get("ts", ev.get("timestamp", ""))
            ev_data = ev.get("data", ev.get("payload", {}))
            snippet = json.dumps(ev_data, default=str)[:100]
            lines.append(f"  [{ts}] {name}: {snippet}")
        timeline_section = "\n## Timeline Excerpt\n\n```\n" + "\n".join(lines) + "\n```\n"

    # Log excerpt section
    log_section = ""
    if log_excerpt and log_excerpt.strip():
        log_section = f"\n## Log Excerpt\n\n```\n{log_excerpt.strip()}\n```\n"

    md = f"""# BUG: {result['name']}

**Severity:** High
**Found by:** Test {result['id']} — {result['name']}
**Suite:** {result.get('suite', 'unknown')}
**Date:** {datetime.now().strftime('%Y-%m-%d')}
{duration_line}{steps_line}
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
{timeline_section}{log_section}
## Probable Cause

To be determined during investigation.
"""

    with open(path, "w") as f:
        f.write(md)

    return path
