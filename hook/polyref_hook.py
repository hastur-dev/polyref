#!/usr/bin/env python3
"""
PolyRef Claude Code Hook

Integrates polyref validation into Claude Code's event system.
Reads event JSON from stdin, outputs additionalContext JSON to stdout.

Install globally via: python hook/install_hooks.py

Events:
  - SessionStart: Generate reference files and inject context
  - PostToolUse: Validate changed source files incrementally
  - Stop: Full validation report (only on end_turn)
"""
import subprocess
import json
import sys
import os
import shutil
import glob as globmod


SOURCE_EXTENSIONS = {".rs", ".py", ".ts", ".tsx"}
MANIFEST_FILES = {"Cargo.toml", "pyproject.toml", "requirements.txt", "package.json", "Pipfile"}


def get_polyref_binary():
    """Find the polyref binary in PATH or known locations."""
    binary = shutil.which("polyref")
    if binary:
        return binary

    # Check common build output locations
    for base_dir in [
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))),  # repo root
        os.getcwd(),
    ]:
        for profile in ["release", "debug"]:
            for ext in ["", ".exe"]:
                candidate = os.path.join(base_dir, "target", profile, f"polyref{ext}")
                if os.path.exists(candidate):
                    return candidate

    # Check ~/.cargo/bin/polyref
    home = os.path.expanduser("~")
    for ext in ["", ".exe"]:
        candidate = os.path.join(home, ".cargo", "bin", f"polyref{ext}")
        if os.path.exists(candidate):
            return candidate

    return None


def run_polyref(args, cwd="."):
    """Run polyref with given args in cwd, return parsed JSON output."""
    binary = get_polyref_binary()
    if not binary:
        return None

    cmd = [binary, "--output", "json", "--project", cwd] + args
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30, cwd=cwd)
    except subprocess.TimeoutExpired:
        print("polyref timed out", file=sys.stderr)
        return None
    except (FileNotFoundError, OSError) as e:
        print(f"polyref unavailable: {e}", file=sys.stderr)
        return None

    if result.returncode != 0 and result.stderr:
        print(f"polyref error: {result.stderr.strip()}", file=sys.stderr)

    if result.stdout.strip():
        try:
            return json.loads(result.stdout)
        except json.JSONDecodeError:
            return None
    return None


def find_refs_info(cwd):
    """Find reference files in a project directory."""
    # Check for polyref.toml to find refs_dir
    refs_dir = os.path.join(cwd, "refs")
    config_path = os.path.join(cwd, "polyref.toml")
    if os.path.exists(config_path):
        try:
            with open(config_path) as f:
                for line in f:
                    if "refs_dir" in line and "=" in line and not line.strip().startswith("#"):
                        val = line.split("=", 1)[1].strip().strip('"').strip("'")
                        if not os.path.isabs(val):
                            val = os.path.join(cwd, val)
                        refs_dir = val
                        break
        except OSError:
            pass

    if not os.path.isdir(refs_dir):
        return None, []

    # Collect reference file names across all language subdirs
    ref_names = []
    for lang_dir in ["rust", "python", "typescript"]:
        lang_path = os.path.join(refs_dir, lang_dir)
        if not os.path.isdir(lang_path):
            continue
        for f in sorted(globmod.glob(os.path.join(lang_path, "lib_*"))):
            name = os.path.basename(f)
            # Strip lib_ prefix and extension
            base = os.path.splitext(name)[0]
            if base.startswith("lib_"):
                ref_names.append(f"{lang_dir}/{base[4:]}")

    return refs_dir, ref_names


def on_session_start(input_data):
    """Generate reference files and inject context about available refs."""
    cwd = input_data.get("cwd", os.getcwd())

    # Check if this project has any supported manifest files
    has_manifest = any(
        os.path.exists(os.path.join(cwd, m)) for m in MANIFEST_FILES
    )
    if not has_manifest:
        sys.exit(0)

    # Generate references
    run_polyref(["generate"], cwd)

    # Report available refs
    refs_dir, ref_names = find_refs_info(cwd)
    if not ref_names:
        sys.exit(0)

    context = f"""polyref is active for this project.

Reference files are available in `{refs_dir}` for these libraries:
{', '.join(ref_names)}

When writing code that uses these libraries, consult the reference files for correct API usage.
After you write or edit source files, polyref will automatically validate your library API usage.
After you edit a manifest file (Cargo.toml, package.json, etc.), polyref will auto-generate reference files for new dependencies."""

    print(json.dumps({"additionalContext": context}))
    sys.exit(0)


def on_post_tool_use(input_data):
    """Validate changed files after Write/Edit tool use."""
    file_path = input_data.get("tool_input", {}).get("file_path", "")
    if not file_path:
        sys.exit(0)

    cwd = input_data.get("cwd", os.getcwd())
    ext = os.path.splitext(file_path)[1]
    basename = os.path.basename(file_path)

    # If a manifest file was edited, regenerate refs
    if basename in MANIFEST_FILES:
        run_polyref(["generate"], cwd)
        result = run_polyref(["check"], cwd)
    elif ext in SOURCE_EXTENSIONS:
        result = run_polyref(["check"], cwd)
    else:
        sys.exit(0)

    if not result:
        sys.exit(0)

    # Extract issues from result
    total_errors = 0
    issues = []

    # Handle both possible output formats
    if isinstance(result, dict):
        total_errors = result.get("summary", {}).get("total_errors", 0)
        for lang_result in result.get("results", []):
            for issue in lang_result.get("issues", []):
                if issue.get("severity") == "error":
                    msg = f"  {issue.get('file', '?')}:{issue.get('line', '?')} — {issue.get('message', '')}"
                    suggestion = issue.get("suggestion")
                    if suggestion:
                        msg += f" (did you mean '{suggestion}'?)"
                    issues.append(msg)
    elif isinstance(result, list):
        for lang_result in result:
            for issue in lang_result.get("issues", []):
                if issue.get("severity") == "error":
                    total_errors += 1
                    issues.append(
                        f"  {issue.get('file', '?')}:{issue.get('line', '?')} — {issue.get('message', '')}"
                    )

    if issues:
        msg = f"polyref found {total_errors} issue(s):\n" + "\n".join(issues[:10])
        if len(issues) > 10:
            msg += f"\n  ... and {len(issues) - 10} more"
        print(json.dumps({"additionalContext": msg}))

    sys.exit(0)


def on_stop(input_data):
    """Full validation report when Claude finishes a response."""
    cwd = input_data.get("cwd", os.getcwd())
    stop_reason = input_data.get("stop_reason", "")

    # Only run on end_turn (Claude finished naturally)
    if stop_reason != "end_turn":
        sys.exit(0)

    # Check if this project has any supported manifest files
    has_manifest = any(
        os.path.exists(os.path.join(cwd, m)) for m in MANIFEST_FILES
    )
    if not has_manifest:
        sys.exit(0)

    result = run_polyref(["check"], cwd)
    if not result:
        sys.exit(0)

    total_errors = 0
    issues = []

    if isinstance(result, dict):
        total_errors = result.get("summary", {}).get("total_errors", 0)
        for lang_result in result.get("results", []):
            for issue in lang_result.get("issues", []):
                if issue.get("severity") == "error":
                    msg = f"  {issue.get('file', '?')}:{issue.get('line', '?')} — {issue.get('message', '')}"
                    suggestion = issue.get("suggestion")
                    if suggestion:
                        msg += f" (did you mean '{suggestion}'?)"
                    issues.append(msg)
    elif isinstance(result, list):
        for lang_result in result:
            for issue in lang_result.get("issues", []):
                if issue.get("severity") == "error":
                    total_errors += 1
                    issues.append(
                        f"  {issue.get('file', '?')}:{issue.get('line', '?')} — {issue.get('message', '')}"
                    )

    if total_errors > 0:
        msg = f"polyref final check: {total_errors} issue(s) found:\n" + "\n".join(issues[:20])
        if len(issues) > 20:
            msg += f"\n  ... and {len(issues) - 20} more"
        print(json.dumps({"additionalContext": msg}))

    sys.exit(0)


def main():
    """Entry point — reads event JSON from stdin, dispatches to handler."""
    try:
        input_data = json.load(sys.stdin)
    except (json.JSONDecodeError, EOFError):
        input_data = {}

    # Event type comes from argv (Claude Code passes it as the command suffix)
    if len(sys.argv) < 2:
        print("Usage: polyref_hook.py <SessionStart|PostToolUse|Stop>", file=sys.stderr)
        sys.exit(1)

    event_type = sys.argv[1]

    handlers = {
        "SessionStart": on_session_start,
        "PostToolUse": on_post_tool_use,
        "Stop": on_stop,
    }

    handler = handlers.get(event_type)
    if handler:
        handler(input_data)
    # Unknown events silently exit 0
    sys.exit(0)


if __name__ == "__main__":
    main()
