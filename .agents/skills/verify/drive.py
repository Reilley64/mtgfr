#!/usr/bin/env python3
"""Drive a 2-player precon game via HTTP until stuck or max steps. Used by verify loop."""

from __future__ import annotations

import json
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from http.cookiejar import CookieJar
from typing import Any

BASE = "http://127.0.0.1:8080"
MAX_STEPS = 400


class Client:
    def __init__(self) -> None:
        self.jar = CookieJar()
        self.opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(self.jar))
        self.client_seq = 0

    def request(self, method: str, path: str, body: dict | None = None) -> tuple[int, Any]:
        data = None if body is None else json.dumps(body).encode()
        req = urllib.request.Request(
            BASE + path,
            data=data,
            method=method,
            headers={"Content-Type": "application/json"} if body is not None else {},
        )
        try:
            with self.opener.open(req, timeout=30) as resp:
                raw = resp.read()
                status = resp.status
        except urllib.error.HTTPError as e:
            status = e.code
            raw = e.read()
        if not raw:
            return status, None
        try:
            return status, json.loads(raw)
        except json.JSONDecodeError:
            return status, raw.decode(errors="replace")

    def snapshot(self, table: str) -> dict | None:
        cookies = "; ".join(f"{c.name}={c.value}" for c in self.jar)
        proc = subprocess.run(
            [
                "curl",
                "-sN",
                "--max-time",
                "2",
                "-w",
                "\n%{http_code}",
                "-H",
                f"Cookie: {cookies}",
                f"{BASE}/tables/{urllib.parse.quote(table)}/stream/v1",
            ],
            capture_output=True,
            text=True,
            timeout=5,
        )
        lines = proc.stdout.splitlines()
        if not lines:
            return None
        http_code = lines[-1].strip()
        body_lines = lines[:-1] if http_code.isdigit() else lines
        if http_code == "404":
            return {"__gone__": True}
        for line in body_lines:
            if not line.startswith("data:"):
                continue
            payload = line[5:].lstrip()
            if not payload:
                continue
            frame = json.loads(payload)
            if frame.get("frame") == "snapshot":
                return frame["state"]
        return None

    def intent(self, table: str, player: int, intent: dict) -> tuple[int, Any]:
        self.client_seq += 1
        return self.request(
            "POST",
            "/intent/v1",
            {"table_id": table, "player_id": player, "client_seq": self.client_seq, "intent": intent},
        )


def signup() -> Client:
    c = Client()
    tag = uuid.uuid4().hex[:10]
    status, body = c.request(
        "POST",
        "/auth/signup/v1",
        {"email": f"verify-{tag}@test.local", "password": "pw", "username": f"v{tag}"},
    )
    if status != 200:
        raise SystemExit(f"signup failed: HTTP {status} {body}")
    return c


def start_game(host: Client, guest: Client) -> str:
    status, decks = host.request("GET", "/decks/v1")
    if status != 200:
        raise SystemExit(f"decks failed: {status} {decks}")
    precon = next(d for d in decks if d.get("id", 0) < 0)
    deck_id = precon["id"]

    status, created = host.request("POST", "/tables/v1", {})
    if status != 200:
        raise SystemExit(f"create table failed: {status} {created}")
    table = created["table_id"]

    for c in (host, guest):
        status, joined = c.request("POST", "/tables/join/v1", {"table_id": table, "deck_id": deck_id})
        if status != 200:
            raise SystemExit(f"join failed: {status} {joined}")
        status, _ = c.request("POST", "/tables/ready/v1", {"table_id": table, "ready": True})
        if status != 200:
            raise SystemExit(f"ready failed: {status}")

    status, started = host.request("POST", "/tables/start/v1", {"table_id": table})
    if status != 200 or not started.get("started"):
        raise SystemExit(f"start failed: {status} {started}")
    return table


def answer_choice(state: dict, player: int) -> dict | None:
    pc = state.get("pending_choice")
    if not pc:
        return None
    kind = pc["kind"]
    if kind == "discard":
        items = pc.get("items") or []
        count = pc.get("count", 1)
        pick = items[: min(count, len(items))]
        ids = [c["id"] if isinstance(c, dict) else c for c in pick]
        return {"kind": "discard", "player": player, "cards": ids}
    if kind == "scry":
        items = pc.get("items") or []
        ids = [c["id"] if isinstance(c, dict) else c for c in items]
        return {"kind": "arrange_top", "player": player, "top": ids, "bottom": []}
    if kind == "surveil":
        items = pc.get("items") or []
        ids = [c["id"] if isinstance(c, dict) else c for c in items]
        return {"kind": "arrange_top", "player": player, "top": [], "bottom": ids}
    if kind == "may_yes_no":
        return {"kind": "answer_may", "player": player, "yes": True}
    if kind == "pay_cost":
        return {"kind": "pay_optional_cost", "player": player, "pay": True}
    if kind == "pay_or_counter":
        return {"kind": "pay_optional_cost", "player": player, "pay": False}
    if kind == "search_library":
        items = pc.get("items") or []
        choice = items[0]["id"] if items else None
        return {"kind": "search_library", "player": player, "choice": choice}
    if kind == "put_land_from_hand":
        items = pc.get("items") or []
        choice = items[0]["id"] if items else None
        return {"kind": "put_land_from_hand", "player": player, "choice": choice}
    if kind == "choose_exiled_with_card":
        items = pc.get("items") or []
        choice = items[0]["id"] if items else None
        return {"kind": "choose_exiled_with_card", "player": player, "choice": choice}
    if kind == "select_from_top":
        items = pc.get("items") or []
        up_to = pc.get("up_to", len(items))
        ids = [c["id"] if isinstance(c, dict) else c for c in items[:up_to]]
        return {"kind": "select_from_top", "player": player, "cards": ids}
    if kind == "choose_mode":
        return {"kind": "choose_mode", "player": player, "mode": 0}
    if kind == "order_triggers":
        count = pc.get("count", 1)
        return {"kind": "choose_order", "player": player, "order": list(range(count))}
    if kind == "assign_combat_damage":
        items = pc.get("items") or []
        if not items:
            return None
        blocker = items[0]["id"]
        power = next(
            (o.get("power", 0) for o in state.get("objects", []) if o.get("id") == pc.get("source")),
            1,
        )
        return {
            "kind": "assign_damage",
            "player": player,
            "assignment": [{"blocker": blocker, "amount": power}],
        }
    if kind == "choose_spell_targets":
        items = pc.get("items") or []
        if not items:
            return None
        min_t = pc.get("min", 1)
        pick = items[:min_t]
        targets = []
        for it in pick:
            if it.get("player") is not None:
                targets.append({"kind": "player", "player": it["player"]})
            else:
                targets.append({"kind": "object", "id": it["id"]})
        return {"kind": "choose_targets", "player": player, "targets": targets}
    if kind == "choose_target":
        items = pc.get("items") or []
        if not items:
            return None
        it = items[0]
        if it.get("player") is not None:
            return {"kind": "choose_targets", "player": player, "targets": [{"kind": "player", "player": it["player"]}]}
        return {"kind": "choose_targets", "player": player, "targets": [{"kind": "object", "id": it["id"]}]}
    if kind == "sacrifice_edict":
        items = pc.get("items") or []
        if not items:
            return None
        keep_one = pc.get("keep_one", False)
        ids = [t["id"] if isinstance(t, dict) else t for t in items]
        if keep_one:
            # Keep the first permanent, sacrifice the rest.
            sacrifices = ids[1:]
        else:
            sacrifices = [ids[0]]
        return {"kind": "choose_sacrifices", "player": player, "sacrifices": sacrifices}
    return None


def pick_action(state: dict, player: int, blocked: set[int], blocked_kinds: set[str]) -> dict | None:
    # Actions are already scoped to the snapshot viewer — no per-action player field.
    actions = state.get("actions") or []
    if not actions:
        return None
    for a in actions:
        if a.get("kind") == "play_land" and a["id"] not in blocked and a.get("kind") not in blocked_kinds:
            return {"kind": "take_action", "player": player, "id": a["id"]}
    for a in actions:
        if a["id"] in blocked or a.get("kind") in blocked_kinds:
            continue
        kind = a.get("kind")
        if kind == "declare_attackers":
            attackers: list[dict] = []
            for obj in state.get("objects") or []:
                if obj.get("controller") != player or obj.get("zone") != 2:
                    continue
                if obj.get("kind", {}).get("kind") != "creature":
                    continue
                if obj.get("tapped"):
                    continue
                if obj.get("summoning_sick") and not obj.get("has_haste"):
                    continue
                opp = next(
                    (p["player"] for p in state.get("players") or [] if p["player"] != player and not p.get("lost")),
                    None,
                )
                if opp is not None:
                    attackers = [{"attacker": obj["id"], "defender": opp}]
                    break
            return {"kind": "take_action", "player": player, "id": a["id"], "attackers": attackers}
        if kind == "declare_blockers":
            return {"kind": "take_action", "player": player, "id": a["id"], "blocks": []}
        if kind in ("pass_priority",):
            continue
        intent: dict = {"kind": "take_action", "player": player, "id": a["id"]}
        targets = a.get("targets") or []
        if a.get("needs_target") and targets:
            intent["target"] = targets[0]
        elif a.get("needs_target"):
            continue
        sac_choices = a.get("sacrifice_choices")
        if sac_choices is not None:
            if not sac_choices:
                continue
            intent["sacrifice"] = sac_choices[0]
        disc_choices = a.get("discard_choices")
        if disc_choices is not None:
            n = a.get("discard_count") or 0
            if len(disc_choices) < n:
                continue
            intent["discard_cost"] = disc_choices[:n]
        gy_choices = a.get("graveyard_exile_choices")
        if gy_choices is not None:
            gmin = a.get("graveyard_exile_min") or 0
            if len(gy_choices) < gmin:
                continue
            # Escape/delve: pick the required exile fodder (same as client after Exile confirm).
            intent["graveyard_exile"] = gy_choices[:gmin] if gmin else []
        if a.get("modal"):
            intent["modes"] = [{"index": 0, "target": targets[0] if targets else None}]
        return intent
    return {"kind": "pass_priority", "player": player}


def actor(states: dict[int, dict]) -> int | None:
    for seat, state in states.items():
        pc = state.get("pending_choice")
        if pc is not None:
            return int(pc["player"])
    for seat, state in states.items():
        if state.get("priority") == state.get("viewer"):
            return seat
    for seat, state in states.items():
        if state.get("can_act"):
            return seat
    return None


def drive(table: str, host: Client, guest: Client) -> bool:
    clients = {0: host, 1: guest}
    stuck = 0
    last_sig = None
    blocked: set[int] = set()
    blocked_kinds: set[str] = set()

    for step in range(MAX_STEPS):
        time.sleep(0.05)
        # Refresh both views; act from whoever holds priority / pending choice.
        states: dict[int, dict] = {}
        for seat, c in clients.items():
            snap = c.snapshot(table)
            if snap is not None:
                if snap.get("__gone__"):
                    print(f"game over at step {step} (table evicted)")
                    return True
                states[seat] = snap

        if not states:
            print(f"step {step}: no snapshot")
            stuck += 1
            if stuck > 20:
                raise SystemExit("stuck: no snapshots")
            continue

        state = next(iter(states.values()))
        sig = (
            state.get("turn"),
            state.get("phase"),
            state.get("step"),
            state.get("priority"),
            (state.get("pending_choice") or {}).get("kind"),
            len(state.get("stack") or []),
            tuple(sorted((a["id"], a.get("kind")) for a in (state.get("actions") or []))),
        )
        if sig == last_sig:
            stuck += 1
        else:
            stuck = 0
            last_sig = sig

        if stuck > 30:
            print(f"STUCK at step {step}: turn={state.get('turn')} phase={state.get('phase')} "
                  f"step={state.get('step')} priority={state.get('priority')} "
                  f"pending={(state.get('pending_choice') or {}).get('kind')} "
                  f"actions={len(state.get('actions') or [])}")
            for seat, st in states.items():
                print(f"  seat {seat}: viewer={st.get('viewer')} can_act={st.get('can_act')} "
                      f"actions={[(a.get('kind'), a.get('id'), a.get('label')) for a in (st.get('actions') or [])]} "
                      f"pending={(st.get('pending_choice') or {}).get('kind')}")
            print(json.dumps(state.get("pending_choice"), indent=2)[:1200])
            return False

        players = state.get("players") or []
        if all(p.get("lost") for p in players):
            print(f"game over at step {step}")
            return True

        seat = actor(states)
        if seat is None:
            continue
        c = clients[seat]
        view = states[seat]

        intent = answer_choice(view, seat) or pick_action(view, seat, blocked, blocked_kinds)
        if intent is None:
            stuck += 1
            continue

        status, body = c.intent(table, seat, intent)
        if status not in (200, 204):
            print(f"step {step} seat {seat} intent {intent['kind']} -> HTTP {status}: {body}")
            return False
        if isinstance(body, dict) and body.get("accepted") is False:
            reason = body.get("reason")
            print(f"step {step} seat {seat} intent {intent['kind']} rejected: {reason}")
            if intent.get("kind") == "take_action" and reason in (
                "CannotActivate",
                "CannotPayCost",
                "IllegalDeclaration",
            ):
                blocked.add(intent["id"])
                action = next((a for a in view.get("actions") or [] if a["id"] == intent["id"]), None)
                if action is not None:
                    blocked_kinds.add(action.get("kind", ""))
            # Prefer passing when an action fails so auto-advance can progress the step.
            if intent.get("kind") != "pass_priority":
                status, body = c.intent(table, seat, {"kind": "pass_priority", "player": seat})
                if isinstance(body, dict) and body.get("accepted") is False:
                    return False
            continue

        if step % 25 == 0:
            print(
                f"step {step}: turn={state.get('turn')} phase={state.get('phase')} "
                f"stack={len(state.get('stack') or [])} intent={intent['kind']}"
            )

    print(f"completed {MAX_STEPS} steps without game over")
    return True


def main() -> None:
    rounds = int(sys.argv[1]) if len(sys.argv) > 1 else 3
    fails = 0
    for i in range(rounds):
        host = signup()
        guest = signup()
        table = start_game(host, guest)
        print(f"--- round {i + 1}/{rounds} table={table} ---")
        if not drive(table, host, guest):
            fails += 1
    if fails:
        raise SystemExit(f"{fails}/{rounds} rounds stuck")
    print(f"OK — {rounds} rounds")


if __name__ == "__main__":
    main()
