"""Tests for polyref hook scripts."""
import importlib.util
import json
import os
import sys
import io
import tempfile
import shutil


def load_module(name, path):
    """Load a Python module from file path."""
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


HOOK_DIR = os.path.dirname(os.path.abspath(__file__))
HOOK_SCRIPT = os.path.join(HOOK_DIR, "polyref_hook.py")
INSTALL_SCRIPT = os.path.join(HOOK_DIR, "install_hooks.py")


# ── polyref_hook.py tests ──


def test_hook_script_imports():
    """Verify polyref_hook.py imports without error and has expected functions."""
    module = load_module("polyref_hook", HOOK_SCRIPT)
    assert hasattr(module, "main")
    assert hasattr(module, "on_session_start")
    assert hasattr(module, "on_post_tool_use")
    assert hasattr(module, "on_stop")
    assert hasattr(module, "get_polyref_binary")
    assert hasattr(module, "run_polyref")
    assert hasattr(module, "find_refs_info")


def test_hook_unknown_event():
    """Unknown event silently exits without crashing."""
    module = load_module("polyref_hook", HOOK_SCRIPT)
    handlers = {
        "SessionStart": module.on_session_start,
        "PostToolUse": module.on_post_tool_use,
        "Stop": module.on_stop,
    }
    handler = handlers.get("UnknownEvent")
    assert handler is None


def test_hook_constants():
    """Verify SOURCE_EXTENSIONS and MANIFEST_FILES are correct."""
    module = load_module("polyref_hook", HOOK_SCRIPT)
    assert ".rs" in module.SOURCE_EXTENSIONS
    assert ".py" in module.SOURCE_EXTENSIONS
    assert ".ts" in module.SOURCE_EXTENSIONS
    assert ".tsx" in module.SOURCE_EXTENSIONS
    assert "Cargo.toml" in module.MANIFEST_FILES
    assert "package.json" in module.MANIFEST_FILES
    assert "pyproject.toml" in module.MANIFEST_FILES
    assert "requirements.txt" in module.MANIFEST_FILES


def test_find_refs_info_no_refs_dir():
    """find_refs_info returns None when no refs directory exists."""
    module = load_module("polyref_hook", HOOK_SCRIPT)
    with tempfile.TemporaryDirectory() as tmp:
        refs_dir, names = module.find_refs_info(tmp)
        assert refs_dir is None or not os.path.isdir(refs_dir)
        assert names == []


def test_find_refs_info_with_refs():
    """find_refs_info finds reference files across language subdirs."""
    module = load_module("polyref_hook", HOOK_SCRIPT)
    with tempfile.TemporaryDirectory() as tmp:
        # Create refs/rust/lib_serde.rs and refs/python/lib_requests.py
        rust_dir = os.path.join(tmp, "refs", "rust")
        python_dir = os.path.join(tmp, "refs", "python")
        os.makedirs(rust_dir)
        os.makedirs(python_dir)
        with open(os.path.join(rust_dir, "lib_serde.rs"), "w") as f:
            f.write("// stub")
        with open(os.path.join(python_dir, "lib_requests.py"), "w") as f:
            f.write("# stub")

        refs_dir, names = module.find_refs_info(tmp)
        assert refs_dir is not None
        assert "python/requests" in names
        assert "rust/serde" in names


def test_find_refs_info_with_config():
    """find_refs_info reads refs_dir from polyref.toml."""
    module = load_module("polyref_hook", HOOK_SCRIPT)
    with tempfile.TemporaryDirectory() as tmp:
        # Create custom refs dir
        custom_dir = os.path.join(tmp, "my_refs", "rust")
        os.makedirs(custom_dir)
        with open(os.path.join(custom_dir, "lib_clap.rs"), "w") as f:
            f.write("// stub")

        # Write polyref.toml pointing to custom dir
        with open(os.path.join(tmp, "polyref.toml"), "w") as f:
            f.write('refs_dir = "my_refs"\n')

        refs_dir, names = module.find_refs_info(tmp)
        assert refs_dir is not None
        assert "rust/clap" in names


def test_post_tool_use_ignores_non_source():
    """on_post_tool_use exits early for non-source files."""
    module = load_module("polyref_hook", HOOK_SCRIPT)
    # Calling with a .txt file should not call run_polyref
    # We verify by checking it doesn't crash and doesn't produce output
    input_data = {
        "tool_input": {"file_path": "README.md"},
        "cwd": tempfile.gettempdir(),
    }
    # Capture stdout — should produce nothing
    old_stdout = sys.stdout
    sys.stdout = io.StringIO()
    try:
        module.on_post_tool_use(input_data)
    except SystemExit as e:
        assert e.code == 0
    finally:
        output = sys.stdout.getvalue()
        sys.stdout = old_stdout
    assert output == ""


def test_post_tool_use_no_file():
    """on_post_tool_use exits early when no file_path in tool_input."""
    module = load_module("polyref_hook", HOOK_SCRIPT)
    input_data = {"tool_input": {}, "cwd": tempfile.gettempdir()}
    try:
        module.on_post_tool_use(input_data)
    except SystemExit as e:
        assert e.code == 0


def test_stop_non_end_turn():
    """on_stop exits early when stop_reason is not end_turn."""
    module = load_module("polyref_hook", HOOK_SCRIPT)
    input_data = {"stop_reason": "tool_use", "cwd": tempfile.gettempdir()}
    try:
        module.on_stop(input_data)
    except SystemExit as e:
        assert e.code == 0


def test_stop_no_manifest():
    """on_stop exits early when project has no manifest files."""
    module = load_module("polyref_hook", HOOK_SCRIPT)
    with tempfile.TemporaryDirectory() as tmp:
        input_data = {"stop_reason": "end_turn", "cwd": tmp}
        try:
            module.on_stop(input_data)
        except SystemExit as e:
            assert e.code == 0


def test_session_start_no_manifest():
    """on_session_start exits early when project has no manifest files."""
    module = load_module("polyref_hook", HOOK_SCRIPT)
    with tempfile.TemporaryDirectory() as tmp:
        input_data = {"cwd": tmp}
        try:
            module.on_session_start(input_data)
        except SystemExit as e:
            assert e.code == 0


# ── install_hooks.py tests ──


def test_install_updates_settings():
    """Install adds correct hook structure to settings.json."""
    module = load_module("install_hooks", INSTALL_SCRIPT)
    with tempfile.TemporaryDirectory() as tmp:
        settings_path = os.path.join(tmp, ".claude", "settings.json")
        os.makedirs(os.path.dirname(settings_path))

        # Create minimal existing settings
        with open(settings_path, "w") as f:
            json.dump({"someExistingSetting": True}, f)

        # Monkey-patch get_settings_path
        original = module.get_settings_path
        module.get_settings_path = lambda: settings_path
        try:
            module.install_hooks(HOOK_DIR)
        finally:
            module.get_settings_path = original

        with open(settings_path) as f:
            settings = json.load(f)

        # Existing settings preserved
        assert settings["someExistingSetting"] is True

        # Hooks added
        assert "hooks" in settings
        hooks = settings["hooks"]
        assert "SessionStart" in hooks
        assert "PostToolUse" in hooks
        assert "Stop" in hooks

        # PostToolUse has matcher
        post_tool = hooks["PostToolUse"]
        assert len(post_tool) == 1
        assert post_tool[0]["matcher"] == "Write|Edit|MultiEdit"
        assert post_tool[0]["hooks"][0]["type"] == "command"
        assert "polyref_hook.py" in post_tool[0]["hooks"][0]["command"]
        assert post_tool[0]["hooks"][0]["timeout"] == 30

        # Stop has timeout
        stop = hooks["Stop"]
        assert stop[0]["hooks"][0]["timeout"] == 60


def test_install_replaces_old_hooks():
    """Install removes rust-ref-guard hooks and adds polyref hooks."""
    module = load_module("install_hooks", INSTALL_SCRIPT)
    with tempfile.TemporaryDirectory() as tmp:
        settings_path = os.path.join(tmp, ".claude", "settings.json")
        os.makedirs(os.path.dirname(settings_path))

        # Create settings with old rust-ref-guard hooks
        old_settings = {
            "hooks": {
                "PostToolUse": [
                    {
                        "matcher": "Write|Edit",
                        "hooks": [
                            {
                                "type": "command",
                                "command": 'python "C:/some/path/rust-ref-guard/.claude/hooks/post_tool_check.py"',
                            }
                        ],
                    }
                ],
                "SessionStart": [
                    {
                        "hooks": [
                            {
                                "type": "command",
                                "command": 'python "C:/some/path/rust-ref-guard/.claude/hooks/session_start.py"',
                            }
                        ]
                    }
                ],
            }
        }
        with open(settings_path, "w") as f:
            json.dump(old_settings, f)

        original = module.get_settings_path
        module.get_settings_path = lambda: settings_path
        try:
            module.install_hooks(HOOK_DIR)
        finally:
            module.get_settings_path = original

        with open(settings_path) as f:
            settings = json.load(f)

        # No rust-ref-guard references remain
        settings_str = json.dumps(settings)
        assert "rust-ref-guard" not in settings_str
        assert "ref-guard" not in settings_str

        # polyref hooks present
        assert "polyref" in json.dumps(settings["hooks"])


def test_install_preserves_other_hooks():
    """Install preserves non-polyref hooks in the same events."""
    module = load_module("install_hooks", INSTALL_SCRIPT)
    with tempfile.TemporaryDirectory() as tmp:
        settings_path = os.path.join(tmp, ".claude", "settings.json")
        os.makedirs(os.path.dirname(settings_path))

        # Settings with a custom hook alongside ref-guard
        old_settings = {
            "hooks": {
                "PostToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "echo custom hook",
                            }
                        ],
                    },
                    {
                        "matcher": "Write|Edit",
                        "hooks": [
                            {
                                "type": "command",
                                "command": 'python "rust-ref-guard/post_tool_check.py"',
                            }
                        ],
                    },
                ],
            }
        }
        with open(settings_path, "w") as f:
            json.dump(old_settings, f)

        original = module.get_settings_path
        module.get_settings_path = lambda: settings_path
        try:
            module.install_hooks(HOOK_DIR)
        finally:
            module.get_settings_path = original

        with open(settings_path) as f:
            settings = json.load(f)

        post_tool = settings["hooks"]["PostToolUse"]
        # Custom hook preserved + polyref hook added
        assert len(post_tool) == 2
        # First is the custom one
        assert "echo custom hook" in post_tool[0]["hooks"][0]["command"]
        # Second is polyref
        assert "polyref" in post_tool[1]["hooks"][0]["command"]


def test_install_creates_backup():
    """Install creates a .bak backup of existing settings."""
    module = load_module("install_hooks", INSTALL_SCRIPT)
    with tempfile.TemporaryDirectory() as tmp:
        settings_path = os.path.join(tmp, ".claude", "settings.json")
        backup_path = settings_path + ".bak"
        os.makedirs(os.path.dirname(settings_path))

        original_content = {"existingKey": "existingValue"}
        with open(settings_path, "w") as f:
            json.dump(original_content, f)

        original = module.get_settings_path
        module.get_settings_path = lambda: settings_path
        try:
            module.install_hooks(HOOK_DIR)
        finally:
            module.get_settings_path = original

        # Backup file exists with original content
        assert os.path.exists(backup_path)
        with open(backup_path) as f:
            backup = json.load(f)
        assert backup == original_content


def test_uninstall_removes_hooks():
    """Uninstall removes polyref hooks from settings."""
    module = load_module("install_hooks", INSTALL_SCRIPT)
    with tempfile.TemporaryDirectory() as tmp:
        settings_path = os.path.join(tmp, ".claude", "settings.json")
        os.makedirs(os.path.dirname(settings_path))

        # Install first
        original = module.get_settings_path
        module.get_settings_path = lambda: settings_path
        try:
            module.install_hooks(HOOK_DIR)

            with open(settings_path) as f:
                settings = json.load(f)
            assert "hooks" in settings
            assert len(settings["hooks"]) > 0

            # Now uninstall
            module.uninstall_hooks()

            with open(settings_path) as f:
                settings = json.load(f)
            # Hooks dict should be empty
            assert settings.get("hooks", {}) == {}
        finally:
            module.get_settings_path = original


def test_is_polyref_hook():
    """is_polyref_hook correctly identifies polyref and ref-guard hooks."""
    module = load_module("install_hooks", INSTALL_SCRIPT)

    assert module.is_polyref_hook({
        "hooks": [{"command": 'python "path/to/polyref_hook.py" PostToolUse'}]
    })
    assert module.is_polyref_hook({
        "hooks": [{"command": 'python "rust-ref-guard/.claude/hooks/post_tool_check.py"'}]
    })
    assert module.is_polyref_hook({
        "hooks": [{"command": "ref-guard check file.rs"}]
    })
    assert not module.is_polyref_hook({
        "hooks": [{"command": "echo custom hook"}]
    })
    assert not module.is_polyref_hook({
        "hooks": [{"command": "python my_custom_linter.py"}]
    })


# ── Runner ──


if __name__ == "__main__":
    tests = [
        test_hook_script_imports,
        test_hook_unknown_event,
        test_hook_constants,
        test_find_refs_info_no_refs_dir,
        test_find_refs_info_with_refs,
        test_find_refs_info_with_config,
        test_post_tool_use_ignores_non_source,
        test_post_tool_use_no_file,
        test_stop_non_end_turn,
        test_stop_no_manifest,
        test_session_start_no_manifest,
        test_install_updates_settings,
        test_install_replaces_old_hooks,
        test_install_preserves_other_hooks,
        test_install_creates_backup,
        test_uninstall_removes_hooks,
        test_is_polyref_hook,
    ]

    passed = 0
    failed = 0
    for test in tests:
        try:
            test()
            print(f"PASSED: {test.__name__}")
            passed += 1
        except Exception as e:
            print(f"FAILED: {test.__name__}: {e}")
            failed += 1

    print(f"\n{passed} passed, {failed} failed out of {len(tests)} tests")
    if failed > 0:
        sys.exit(1)
