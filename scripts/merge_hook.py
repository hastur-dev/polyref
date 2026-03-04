"""Merge polyref enforce hook into existing Claude Code settings.json.

Usage: python merge_hook.py <settings_path> <hook_command>

Reads the existing settings.json, adds the enforce hook to PostToolUse
(if not already present), and writes back. Creates the file if missing.
"""

import json
import sys
import os

MATCHER = "Write|Edit|MultiEdit|NotebookEdit"


def merge_hook(settings_path: str, hook_command: str) -> None:
    # Load existing settings or start fresh
    if os.path.exists(settings_path):
        with open(settings_path, "r", encoding="utf-8") as f:
            settings = json.load(f)
    else:
        settings = {}

    hooks = settings.setdefault("hooks", {})
    post_tool = hooks.setdefault("PostToolUse", [])

    # Check if this hook command is already installed
    for entry in post_tool:
        for h in entry.get("hooks", []):
            if h.get("command") == hook_command:
                print(f"Hook already installed in {settings_path}")
                return

    # Add new hook entry
    new_entry = {
        "matcher": MATCHER,
        "hooks": [
            {
                "type": "command",
                "command": hook_command,
                "timeout": 30,
            }
        ],
    }
    post_tool.append(new_entry)

    # Write back
    os.makedirs(os.path.dirname(settings_path), exist_ok=True)
    with open(settings_path, "w", encoding="utf-8") as f:
        json.dump(settings, f, indent=2)
        f.write("\n")

    print(f"Hook added to {settings_path}")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <settings_path> <hook_command>")
        sys.exit(1)
    merge_hook(sys.argv[1], sys.argv[2])
