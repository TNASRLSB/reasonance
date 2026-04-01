"""KWin D-Bus window management for KDE Wayland."""

import logging
import os
import subprocess
import tempfile
import time

_script_counter = 0


def _build_kwin_script(action: str, resource_class: str) -> str:
    """Build a KWin JavaScript snippet for the given action."""
    if '"' in resource_class or '\\' in resource_class:
        raise ValueError(f"Unsafe resource_class: {resource_class!r}")
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
    except Exception as e:
        logging.getLogger(__name__).warning("KWin script failed: %s", e)
        return False
    finally:
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


def get_screen_resolution() -> tuple[int, int]:
    """Get screen resolution via xdpyinfo or xrandr as fallback."""
    try:
        result = subprocess.run(
            ["xdpyinfo"], capture_output=True, text=True, timeout=3,
        )
        for line in result.stdout.splitlines():
            if "dimensions:" in line:
                # e.g. "  dimensions:    1920x1080 pixels (...)"
                parts = line.split()
                idx = parts.index("dimensions:") + 1
                w, h = parts[idx].split("x")
                return int(w), int(h)
    except Exception:
        pass
    try:
        result = subprocess.run(
            ["xrandr", "--current"], capture_output=True, text=True, timeout=3,
        )
        for line in result.stdout.splitlines():
            if " connected" in line and "x" in line:
                # e.g. "eDP-1 connected primary 1920x1080+0+0 ..."
                for part in line.split():
                    if "x" in part and "+" in part:
                        res = part.split("+")[0]
                        w, h = res.split("x")
                        return int(w), int(h)
    except Exception:
        pass
    return 1920, 1080


def get_geometry(resource_class: str) -> dict:
    """Return {x, y, width, height} of window via qdbus6.

    Falls back to screen resolution (assuming maximized) if qdbus6 fails.
    """
    focus(resource_class)
    time.sleep(0.3)
    try:
        result = subprocess.run(
            ["qdbus6", "org.kde.KWin", "/KWin", "org.kde.KWin.queryWindowInfo"],
            capture_output=True, text=True, timeout=5,
        )
        if result.returncode == 0:
            info = {}
            for line in result.stdout.splitlines():
                if ":" in line:
                    k, v = line.split(":", 1)
                    info[k.strip()] = v.strip()
            if all(k in info for k in ("x", "y", "width", "height")):
                return {
                    "x": int(info["x"]),
                    "y": int(info["y"]),
                    "width": int(info["width"]),
                    "height": int(info["height"]),
                }
    except Exception:
        pass

    # Fallback: use screen resolution (window is maximized)
    w, h = get_screen_resolution()
    return {"x": 0, "y": 0, "width": w, "height": h}
