#!/usr/bin/env python3
# phronia-shim: shared@v1
# shim-version: 0.6.17
# DO NOT EDIT. Regenerate with: phronia update --regenerate-shims

"""Phronia hook shim — the only harness adapter for Python-hook platforms.

It reads the harness payload, calls the core (`phronia context`), applies the
returned mutations/effects, and writes the harness envelope. It contains no
context logic of its own.

Usage (from a platform hook config):

    python3 .phronia/hooks/phronia-hook.py --event session-start --platform claude
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

EVENT_BY_FILENAME = {
    "session-start.py": "session-start",
    "inject-workflow-state.py": "user-prompt",
    "inject-subagent-context.py": "subagent-spawn",
    "inject-shell-session-context.py": "pre-shell",
}

PROJECT_ENV_VARS = (
    "ZCODE_PROJECT_DIR",
    "CURSOR_PROJECT_DIR",
    "CODEBUDDY_PROJECT_DIR",
    "FACTORY_PROJECT_DIR",
    "GEMINI_PROJECT_DIR",
    "QODER_PROJECT_DIR",
    "KIRO_PROJECT_DIR",
    "COPILOT_PROJECT_DIR",
    "TRAE_PROJECT_DIR",
    "CLAUDE_PROJECT_DIR",
)


def _arg(argv: list[str], name: str, default: str | None = None) -> str | None:
    try:
        index = argv.index(name)
    except ValueError:
        return default
    return argv[index + 1] if index + 1 < len(argv) else default


def _platform(argv: list[str], payload: dict[str, Any]) -> str:
    explicit = _arg(argv, "--platform")
    if explicit:
        return explicit
    for part in reversed(Path(sys.argv[0]).parts):
        if part.startswith(".") and len(part) > 1:
            return part[1:]
    return str(payload.get("platform") or "claude")


def _project_root(payload: dict[str, Any]) -> str:
    for name in PROJECT_ENV_VARS:
        value = os.environ.get(name)
        if value:
            return value
    cwd = payload.get("cwd")
    if isinstance(cwd, str) and cwd.strip():
        return cwd
    return os.getcwd()


def _resolve_cli() -> list[str]:
    explicit = os.environ.get("PHRONIA_CLI_PATH")
    if explicit:
        return [sys.executable, explicit] if explicit.endswith(".py") else ["node", explicit]
    for name in ("phronia", "trellis"):
        found = shutil.which(name)
        if found:
            return [found]
    sys.stderr.write(
        "phronia-hook: cannot find the phronia CLI; install @phronia/cli or set PHRONIA_CLI_PATH\n"
    )
    raise SystemExit(1)


def _call_core(request: dict[str, Any], render: bool) -> subprocess.CompletedProcess[str]:
    command = [
        *_resolve_cli(),
        "context",
        "--event",
        str(request["event"]),
        "--platform",
        str(request["platform"]["id"]),
        "--project-root",
        str(request["projectRoot"]),
        "--input",
        "-",
        "--json",
    ]
    if render:
        command.append("--render")
    return subprocess.run(
        command,
        input=json.dumps(request["input"]),
        text=True,
        capture_output=True,
        timeout=30,
    )


def _event(argv: list[str], payload: dict[str, Any]) -> str:
    explicit = _arg(argv, "--event")
    if explicit:
        return explicit
    name = Path(sys.argv[0]).name
    if name in EVENT_BY_FILENAME:
        return EVENT_BY_FILENAME[name]
    tool = str(payload.get("tool_name") or payload.get("toolName") or "").lower()
    if tool in ("task", "agent", "subagent"):
        return "subagent-spawn"
    if tool in ("bash", "shell"):
        return "pre-shell"
    return "session-start"


def _apply_effects(response: dict[str, Any]) -> None:
    for effect in response.get("effects", []):
        kind = effect.get("type")
        if kind == "env-export":
            target = effect.get("file")
            line = effect.get("line")
            if not isinstance(target, str) or not isinstance(line, str):
                continue
            try:
                existing = Path(target).read_text(encoding="utf-8") if os.path.exists(target) else ""
                if existing.splitlines() and existing.splitlines()[-1] == line:
                    continue
                with open(target, "a", encoding="utf-8") as handle:
                    handle.write(f"{line}\n")
            except OSError:
                pass
        elif kind in ("shell-ticket", "marker"):
            target = effect.get("path")
            content = effect.get("content")
            if not isinstance(target, str) or not isinstance(content, str):
                continue
            try:
                Path(target).parent.mkdir(parents=True, exist_ok=True)
                Path(target).write_text(content, encoding="utf-8")
            except OSError:
                pass


def _render_mutation(platform: str, payload: dict[str, Any], response: dict[str, Any]) -> str:
    mutations = response.get("mutations", [])
    if not mutations:
        return ""
    mutation = mutations[0]
    value = mutation.get("value", "")
    tool_input = payload.get("tool_input") or payload.get("toolInput") or {}
    if not isinstance(tool_input, dict):
        tool_input = {}
    updated = {**tool_input, "prompt": value}
    if platform in ("kiro", "gemini"):
        return json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "updatedInput": updated}})
    if platform == "zcode":
        return json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "allow",
                    "updatedInput": updated,
                }
            }
        )
    return json.dumps(
        {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "allow",
                "updatedInput": updated,
            },
            "permission": "allow",
            "updated_input": updated,
            "updatedInput": updated,
        }
    )


def main() -> int:
    if os.environ.get("PHRONIA_HOOKS") == "0" or os.environ.get("PHRONIA_DISABLE_HOOKS") == "1":
        return 0
    raw = sys.stdin.read()
    try:
        payload = json.loads(raw) if raw.strip() else {}
    except (json.JSONDecodeError, ValueError):
        payload = {}
    if not isinstance(payload, dict):
        payload = {}

    event = _event(sys.argv[1:], payload)
    platform = _platform(sys.argv[1:], payload)
    request = {
        "schemaVersion": 1,
        "event": event,
        "platform": {"id": platform},
        "projectRoot": _project_root(payload),
        "input": payload,
    }

    plan = _call_core(request, render=False)
    if plan.returncode != 0:
        sys.stderr.write(plan.stderr or "phronia-hook: core failed\n")
        return plan.returncode
    try:
        response = json.loads(plan.stdout)
    except json.JSONDecodeError:
        sys.stderr.write("phronia-hook: core returned invalid JSON\n")
        return 1

    _apply_effects(response)
    if response.get("mutations"):
        output = _render_mutation(platform, payload, response)
        if output:
            print(output)
        return 0

    rendered = _call_core(request, render=True)
    if rendered.returncode != 0:
        sys.stderr.write(rendered.stderr or "phronia-hook: render failed\n")
        return rendered.returncode
    sys.stdout.write(rendered.stdout)
    return 0


if __name__ == "__main__":
    sys.exit(main())
