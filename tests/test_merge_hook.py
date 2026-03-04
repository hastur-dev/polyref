"""Tests for merge_hook.py"""
import json
import os
import sys
import tempfile

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "scripts"))
from merge_hook import merge_hook

HOOK_CMD = 'bash "scripts/enforce-pipeline.sh"'


def test_creates_new_file():
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, ".claude", "settings.json")
        merge_hook(path, HOOK_CMD)

        assert os.path.exists(path)
        with open(path) as f:
            data = json.load(f)
        assert "hooks" in data
        assert len(data["hooks"]["PostToolUse"]) == 1
        assert data["hooks"]["PostToolUse"][0]["hooks"][0]["command"] == HOOK_CMD


def test_merges_into_existing():
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "settings.json")
        existing = {
            "someKey": True,
            "hooks": {
                "PostToolUse": [
                    {
                        "matcher": "Write",
                        "hooks": [{"type": "command", "command": "echo old"}],
                    }
                ],
                "SessionStart": [
                    {"hooks": [{"type": "command", "command": "echo start"}]}
                ],
            },
        }
        with open(path, "w") as f:
            json.dump(existing, f)

        merge_hook(path, HOOK_CMD)

        with open(path) as f:
            data = json.load(f)
        # Original key preserved
        assert data["someKey"] is True
        # Original hook preserved
        assert len(data["hooks"]["PostToolUse"]) == 2
        assert data["hooks"]["PostToolUse"][0]["hooks"][0]["command"] == "echo old"
        # New hook added
        assert data["hooks"]["PostToolUse"][1]["hooks"][0]["command"] == HOOK_CMD
        # Other hook events preserved
        assert len(data["hooks"]["SessionStart"]) == 1


def test_idempotent():
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "settings.json")
        with open(path, "w") as f:
            json.dump({}, f)

        merge_hook(path, HOOK_CMD)
        merge_hook(path, HOOK_CMD)  # second call

        with open(path) as f:
            data = json.load(f)
        # Should only have one entry, not two
        assert len(data["hooks"]["PostToolUse"]) == 1


def test_empty_existing_file():
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "settings.json")
        with open(path, "w") as f:
            json.dump({}, f)

        merge_hook(path, HOOK_CMD)

        with open(path) as f:
            data = json.load(f)
        assert len(data["hooks"]["PostToolUse"]) == 1


def test_no_hooks_section():
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "settings.json")
        with open(path, "w") as f:
            json.dump({"someOtherSetting": 42}, f)

        merge_hook(path, HOOK_CMD)

        with open(path) as f:
            data = json.load(f)
        assert data["someOtherSetting"] == 42
        assert len(data["hooks"]["PostToolUse"]) == 1


if __name__ == "__main__":
    test_creates_new_file()
    test_merges_into_existing()
    test_idempotent()
    test_empty_existing_file()
    test_no_hooks_section()
    print("All 5 tests passed!")
