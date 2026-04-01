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
    """Return {x, y, width, height} of window via qdbus6."""
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
