#!/usr/bin/env python3
"""Cursor hooks for docs/CR_INDEX.md freshness.

afterFileEdit: if this conversation (or its parent, for Task/subagents) edited a
CR-corpus path, mark dirty for that owner conversation.
stop: if *this* conversation is dirty, regenerate once, then optionally follow up
when the index changed.
subagentStop: if modified_files include CR-corpus paths, mark the parent dirty when
known; otherwise regenerate immediately (no follow-up).

Dirty state is keyed by conversation owner so a client-only chat does not pick up
another agent's engine edits and inject a CR_INDEX follow-up.

Fail-open: never block the agent. Stdin is the Cursor hook JSON payload.
"""

from __future__ import annotations

import hashlib
import io
import json
import re
import subprocess
import sys
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from typing import Callable

ROOT = Path(__file__).resolve().parents[2]
GENERATOR = ROOT / "scripts" / "gen_cr_index.py"
INDEX = ROOT / "docs" / "CR_INDEX.md"
STATE_DIR = ROOT / ".cursor" / "hooks" / "state"
LEGACY_DIRTY = STATE_DIR / "cr-index-dirty"

# Paths relative to repo root.
TRIGGERS = (
    re.compile(r"^crates/engine/(src|tests)/.+\.rs$"),
    re.compile(r"^scripts/gen_cr_index\.py$"),
)

_SAFE_ID = re.compile(r"[^A-Za-z0-9._-]+")

RegenFn = Callable[[], tuple[int, str, str]]


def path_under_root(file_path: str, root: Path = ROOT) -> Path | None:
    """Resolve file_path and return it only if it lives under root."""
    if not file_path:
        return None
    raw = Path(file_path)
    candidate = raw if raw.is_absolute() else (root / raw)
    try:
        resolved = candidate.resolve()
        root_resolved = root.resolve()
        resolved.relative_to(root_resolved)
        return resolved
    except (ValueError, OSError):
        return None


def should_regen(file_path: str, root: Path = ROOT) -> bool:
    resolved = path_under_root(file_path, root)
    if resolved is None:
        return False
    rel = resolved.relative_to(root.resolve()).as_posix()
    return any(p.match(rel) for p in TRIGGERS)


def dirty_owner_id(payload: dict) -> str:
    """Conversation that should own the dirty marker after an edit.

    Prefer parent_conversation_id so Task/subagent engine edits still regenerate
    when the parent agent stops.
    """
    parent = (payload.get("parent_conversation_id") or "").strip()
    if parent:
        return parent
    return stopping_conversation_id(payload)


def stopping_conversation_id(payload: dict) -> str:
    """Id of the conversation that is ending (stop hook)."""
    return (payload.get("conversation_id") or payload.get("session_id") or "").strip()


def dirty_path(cid: str) -> Path | None:
    """Return the per-conversation dirty marker path, or None if cid is missing.

    The path includes a content hash of the raw id so sanitization cannot collide
    distinct owners onto one marker.
    """
    if not cid:
        return None
    digest = hashlib.sha256(cid.encode("utf-8")).hexdigest()[:16]
    safe = _SAFE_ID.sub("_", cid).strip("._-")[:48]
    label = safe if safe else "id"
    return STATE_DIR / f"dirty-{label}-{digest}"


def clear_legacy_dirty() -> None:
    try:
        LEGACY_DIRTY.unlink()
    except FileNotFoundError:
        pass


def mark_dirty(cid: str) -> None:
    path = dirty_path(cid)
    if path is None:
        return
    STATE_DIR.mkdir(parents=True, exist_ok=True)
    clear_legacy_dirty()
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text("1\n", encoding="utf-8")
    tmp.replace(path)


def clear_dirty(cid: str) -> None:
    clear_legacy_dirty()
    path = dirty_path(cid)
    if path is None:
        return
    try:
        path.unlink()
    except FileNotFoundError:
        pass


def is_dirty(cid: str) -> bool:
    path = dirty_path(cid)
    return path is not None and path.is_file()


def run_generator() -> tuple[int, str, str]:
    result = subprocess.run(
        [sys.executable, str(GENERATOR)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return result.returncode, result.stdout or "", result.stderr or ""


def handle_after_file_edit(
    payload: dict,
    *,
    regenerate: RegenFn = run_generator,
) -> int:
    file_path = payload.get("file_path") or ""
    if not should_regen(file_path):
        return 0
    owner = dirty_owner_id(payload)
    if not owner:
        print(
            "regen-cr-index: missing conversation_id on afterFileEdit; regenerating immediately",
            file=sys.stderr,
        )
        code, stdout, stderr = regenerate()
        if stdout:
            print(stdout, end="", file=sys.stderr)
        if code != 0:
            print(f"regen-cr-index: generator exited {code}", file=sys.stderr)
            if stderr:
                print(stderr, end="", file=sys.stderr)
        return 0
    mark_dirty(owner)
    return 0


def handle_stop(
    payload: dict,
    *,
    regenerate: RegenFn = run_generator,
) -> int:
    # Always emit JSON for stop hooks.
    out: dict = {}
    owner = stopping_conversation_id(payload)
    # Drop the pre-scoping global marker so orphaned dirtiness cannot leak across chats.
    clear_legacy_dirty()
    if not owner or not is_dirty(owner):
        print(json.dumps(out))
        return 0

    if not GENERATOR.is_file():
        print("regen-cr-index: missing generator", file=sys.stderr)
        clear_dirty(owner)
        print(json.dumps(out))
        return 0

    before = INDEX.read_text(encoding="utf-8") if INDEX.is_file() else None
    code, stdout, stderr = regenerate()
    if stdout:
        print(stdout, end="", file=sys.stderr)
    if code != 0:
        print(f"regen-cr-index: generator exited {code}", file=sys.stderr)
        if stderr:
            print(stderr, end="", file=sys.stderr)
        clear_dirty(owner)
        print(json.dumps(out))
        return 0

    clear_dirty(owner)
    after = INDEX.read_text(encoding="utf-8") if INDEX.is_file() else None
    status = payload.get("status") or ""
    if status == "completed" and before != after and after is not None:
        out["followup_message"] = (
            "docs/CR_INDEX.md was regenerated from engine CR citations. "
            "Include it in the change set if you are committing."
        )
    print(json.dumps(out))
    return 0


def handle_subagent_stop(
    payload: dict,
    *,
    regenerate: RegenFn = run_generator,
) -> int:
    """If a Task/subagent touched CR corpus files, keep the parent dirty or regen now."""
    out: dict = {}
    files = payload.get("modified_files") or []
    if not any(should_regen(str(f)) for f in files):
        print(json.dumps(out))
        return 0

    parent = (payload.get("parent_conversation_id") or "").strip()
    if parent:
        mark_dirty(parent)
        print(json.dumps(out))
        return 0

    # No parent to defer to — regenerate now so we do not wait on a stop that may
    # never see a child-only dirty marker.
    print(
        "regen-cr-index: subagentStop without parent_conversation_id; regenerating immediately",
        file=sys.stderr,
    )
    code, stdout, stderr = regenerate()
    if stdout:
        print(stdout, end="", file=sys.stderr)
    if code != 0:
        print(f"regen-cr-index: generator exited {code}", file=sys.stderr)
        if stderr:
            print(stderr, end="", file=sys.stderr)
    print(json.dumps(out))
    return 0


def _capture_stop(payload: dict, *, regenerate: RegenFn) -> str:
    buf = io.StringIO()
    with redirect_stdout(buf), redirect_stderr(io.StringIO()):
        handle_stop(payload, regenerate=regenerate)
    return buf.getvalue().strip()


def self_test() -> None:
    root = ROOT
    cases_true = [
        str(root / "crates/engine/src/apply.rs"),
        str(root / "crates/engine/tests/game.rs"),
        "crates/engine/src/combat.rs",  # relative
        str(root / "scripts/gen_cr_index.py"),
    ]
    cases_false = [
        str(root / "AGENTS.md"),
        str(root / "client/src/Lobby.tsx"),
        str(root / "docs/CR_INDEX.md"),
        "/tmp/other/crates/engine/src/apply.rs",
        str(root / "crates/engine/README.md"),
        "",
    ]
    for path in cases_true:
        if not should_regen(path, root):
            raise AssertionError(f"expected regen for {path!r}")
    for path in cases_false:
        if should_regen(path, root):
            raise AssertionError(f"expected skip for {path!r}")
    # Fake root: absolute path under real ROOT must not match a different root
    other = Path("/tmp/mtgfr-hook-test-root")
    if should_regen(str(root / "crates/engine/src/apply.rs"), other):
        raise AssertionError("path under real ROOT must not match foreign root")

    # Per-conversation dirty isolation
    a, b = "conv-aaa", "conv-bbb"
    clear_dirty(a)
    clear_dirty(b)
    if is_dirty(a) or is_dirty(b):
        raise AssertionError("expected clean dirty markers before test")
    mark_dirty(a)
    if not is_dirty(a):
        raise AssertionError("expected conversation A dirty")
    if is_dirty(b):
        raise AssertionError("conversation B must not see A's dirty marker")
    clear_dirty(a)
    if is_dirty(a):
        raise AssertionError("expected A cleared")
    if dirty_path("") is not None:
        raise AssertionError("empty conversation_id must not create a marker path")
    mark_dirty("")  # no-op
    if is_dirty(""):
        raise AssertionError("empty conversation_id must never be dirty")

    # Sanitization must not collide distinct owners
    if dirty_path("!!!") == dirty_path("..."):
        raise AssertionError("sanitized-empty ids must not share a dirty path")
    if dirty_path("a/b") == dirty_path("a-b"):
        raise AssertionError("distinct ids must not share a dirty path after sanitize")

    # Parent wins over child conversation id
    if (
        dirty_owner_id(
            {
                "conversation_id": "child",
                "parent_conversation_id": "parent",
            }
        )
        != "parent"
    ):
        raise AssertionError("dirty_owner_id must prefer parent_conversation_id")

    engine_rs = str(root / "crates/engine/src/triggers.rs")
    client_tsx = str(root / "client/src/Lobby.tsx")
    regen_calls: list[str] = []

    def fake_regen() -> tuple[int, str, str]:
        regen_calls.append("run")
        return 0, "", ""

    # Cross-chat: engine edit in A must not cause B's stop to regenerate
    clear_dirty(a)
    clear_dirty(b)
    regen_calls.clear()
    handle_after_file_edit(
        {
            "hook_event_name": "afterFileEdit",
            "conversation_id": a,
            "file_path": engine_rs,
        },
        regenerate=fake_regen,
    )
    if not is_dirty(a):
        raise AssertionError("expected A dirty after engine afterFileEdit")
    out = _capture_stop(
        {"hook_event_name": "stop", "conversation_id": b, "status": "completed"},
        regenerate=fake_regen,
    )
    if out != "{}":
        raise AssertionError(f"B stop should no-op, got {out!r}")
    if regen_calls:
        raise AssertionError("B stop must not regenerate for A's dirty marker")
    if not is_dirty(a):
        raise AssertionError("A dirty marker must survive B's stop")
    out = _capture_stop(
        {"hook_event_name": "stop", "conversation_id": a, "status": "completed"},
        regenerate=fake_regen,
    )
    if out != "{}":
        raise AssertionError(f"A stop should emit JSON object, got {out!r}")
    if regen_calls != ["run"]:
        raise AssertionError("A stop must regenerate exactly once")
    if is_dirty(a):
        raise AssertionError("A dirty marker must clear after stop regen")

    # Client-only edit must not dirty
    regen_calls.clear()
    handle_after_file_edit(
        {
            "hook_event_name": "afterFileEdit",
            "conversation_id": a,
            "file_path": client_tsx,
        },
        regenerate=fake_regen,
    )
    if is_dirty(a) or regen_calls:
        raise AssertionError("client edit must not dirty or regenerate")

    # Subagent edit attributes dirty to parent
    clear_dirty("parent")
    handle_after_file_edit(
        {
            "hook_event_name": "afterFileEdit",
            "conversation_id": "child",
            "parent_conversation_id": "parent",
            "file_path": engine_rs,
        },
        regenerate=fake_regen,
    )
    if not is_dirty("parent"):
        raise AssertionError("subagent afterFileEdit must dirty parent")
    if is_dirty("child"):
        raise AssertionError("subagent afterFileEdit must not dirty child only")
    clear_dirty("parent")

    # Missing conversation_id → immediate regen (no silent skip)
    regen_calls.clear()
    err = io.StringIO()
    with redirect_stderr(err):
        handle_after_file_edit(
            {
                "hook_event_name": "afterFileEdit",
                "file_path": engine_rs,
            },
            regenerate=fake_regen,
        )
    if regen_calls != ["run"]:
        raise AssertionError("missing conversation_id must regenerate immediately")
    if "missing conversation_id" not in err.getvalue():
        raise AssertionError("missing conversation_id must log to stderr")

    # subagentStop with parent marks parent; without parent regenerates now
    clear_dirty("parent")
    regen_calls.clear()
    with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
        handle_subagent_stop(
            {
                "hook_event_name": "subagentStop",
                "parent_conversation_id": "parent",
                "modified_files": [engine_rs],
                "status": "completed",
            },
            regenerate=fake_regen,
        )
    if not is_dirty("parent") or regen_calls:
        raise AssertionError("subagentStop with parent must dirty parent, not regen")
    clear_dirty("parent")
    regen_calls.clear()
    with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
        handle_subagent_stop(
            {
                "hook_event_name": "subagentStop",
                "modified_files": [engine_rs],
                "status": "completed",
            },
            regenerate=fake_regen,
        )
    if regen_calls != ["run"]:
        raise AssertionError("subagentStop without parent must regenerate immediately")
    with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
        handle_subagent_stop(
            {
                "hook_event_name": "subagentStop",
                "modified_files": [client_tsx],
                "status": "completed",
            },
            regenerate=fake_regen,
        )
    if regen_calls != ["run"]:
        raise AssertionError("client-only subagentStop must not regenerate again")


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if argv == ["--self-test"]:
        try:
            self_test()
        except AssertionError as e:
            print(f"self-test failed: {e}", file=sys.stderr)
            return 1
        print("self-test ok")
        return 0

    try:
        payload = json.load(sys.stdin)
    except json.JSONDecodeError:
        # stop hooks should still emit JSON when possible
        print("{}")
        return 0

    event = payload.get("hook_event_name") or ""
    if event == "afterFileEdit":
        return handle_after_file_edit(payload)
    if event == "stop":
        return handle_stop(payload)
    if event == "subagentStop":
        return handle_subagent_stop(payload)

    # Unknown / missing event name: try file_path (afterFileEdit shape) else no-op JSON
    if "file_path" in payload:
        return handle_after_file_edit(payload)
    print("{}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
