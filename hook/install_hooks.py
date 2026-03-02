#!/usr/bin/env python3
"""
Install PolyRef hooks into Claude Code global settings.

Updates ~/.claude/settings.json with hooks for SessionStart, PostToolUse, and Stop.
Replaces any existing rust-ref-guard hooks.

Usage:
  python hook/install_hooks.py [--hook-dir PATH]

Options:
  --hook-dir PATH   Directory containing polyref_hook.py
                    (default: same directory as this script)
  --uninstall       Remove polyref hooks from settings
"""
import json
import os
import shutil
import sys


def get_settings_path():
    """Get path to ~/.claude/settings.json."""
    home = os.path.expanduser("~")
    return os.path.join(home, ".claude", "settings.json")


def load_settings(settings_path):
    """Load existing settings or return empty dict."""
    if os.path.exists(settings_path):
        with open(settings_path) as f:
            return json.load(f)
    return {}


def save_settings(settings_path, settings):
    """Save settings, creating parent directory if needed."""
    os.makedirs(os.path.dirname(settings_path), exist_ok=True)

    # Back up existing settings
    if os.path.exists(settings_path):
        backup_path = settings_path + ".bak"
        shutil.copy2(settings_path, backup_path)

    with open(settings_path, "w") as f:
        json.dump(settings, f, indent=2)
        f.write("\n")


def is_polyref_hook(hook_entry):
    """Check if a hook entry is a polyref or rust-ref-guard hook."""
    for h in hook_entry.get("hooks", []):
        cmd = h.get("command", "")
        if "polyref" in cmd or "rust-ref-guard" in cmd or "ref-guard" in cmd:
            return True
    return False


def install_hooks(hook_dir=None):
    """Install polyref hooks into ~/.claude/settings.json."""
    if hook_dir is None:
        hook_dir = os.path.dirname(os.path.abspath(__file__))

    hook_script = os.path.join(hook_dir, "polyref_hook.py")
    if not os.path.exists(hook_script):
        print(f"Error: polyref_hook.py not found at {hook_script}", file=sys.stderr)
        sys.exit(1)

    # Use forward slashes for cross-platform compatibility in commands
    hook_script_normalized = hook_script.replace("\\", "/")

    settings_path = get_settings_path()
    settings = load_settings(settings_path)

    # Ensure hooks dict exists
    if "hooks" not in settings:
        settings["hooks"] = {}

    hooks = settings["hooks"]

    # Define the polyref hook entries
    polyref_hooks = {
        "SessionStart": [
            {
                "hooks": [
                    {
                        "type": "command",
                        "command": f'python "{hook_script_normalized}" SessionStart',
                    }
                ]
            }
        ],
        "PostToolUse": [
            {
                "matcher": "Write|Edit|MultiEdit",
                "hooks": [
                    {
                        "type": "command",
                        "command": f'python "{hook_script_normalized}" PostToolUse',
                        "timeout": 30,
                    }
                ]
            }
        ],
        "Stop": [
            {
                "hooks": [
                    {
                        "type": "command",
                        "command": f'python "{hook_script_normalized}" Stop',
                        "timeout": 60,
                    }
                ]
            }
        ],
    }

    # For each event, remove old polyref/ref-guard hooks and add new ones
    for event_name, new_entries in polyref_hooks.items():
        existing = hooks.get(event_name, [])

        # Filter out old polyref or rust-ref-guard hooks
        kept = [entry for entry in existing if not is_polyref_hook(entry)]

        # Add new polyref hooks
        kept.extend(new_entries)
        hooks[event_name] = kept

    settings["hooks"] = hooks
    save_settings(settings_path, settings)

    print(f"PolyRef hooks installed in {settings_path}")
    print(f"Hook script: {hook_script}")
    print("Events configured:")
    print("  SessionStart — generate references, inject context")
    print("  PostToolUse  — validate changed files (Write/Edit/MultiEdit)")
    print("  Stop         — full validation on end_turn")


def uninstall_hooks():
    """Remove polyref hooks from ~/.claude/settings.json."""
    settings_path = get_settings_path()
    settings = load_settings(settings_path)

    hooks = settings.get("hooks", {})
    changed = False

    for event_name in list(hooks.keys()):
        original = hooks[event_name]
        filtered = [entry for entry in original if not is_polyref_hook(entry)]
        if len(filtered) != len(original):
            changed = True
            if filtered:
                hooks[event_name] = filtered
            else:
                del hooks[event_name]

    if changed:
        settings["hooks"] = hooks
        save_settings(settings_path, settings)
        print(f"PolyRef hooks removed from {settings_path}")
    else:
        print("No polyref hooks found to remove.")


if __name__ == "__main__":
    if "--uninstall" in sys.argv:
        uninstall_hooks()
    else:
        hook_dir = None
        if "--hook-dir" in sys.argv:
            idx = sys.argv.index("--hook-dir")
            if idx + 1 < len(sys.argv):
                hook_dir = sys.argv[idx + 1]
        install_hooks(hook_dir)
