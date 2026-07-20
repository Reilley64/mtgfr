# 0036 — Sparse synthesized table audio

Status: **Accepted**

## Context

Arena layers music, card SFX, UI alerts, and VO. This client’s north star is “Arena, Unplugged” — game-client polish without storefront spectacle. We still want audio that pulls attention when you owe a decision and light table feel for shared actions.

## Decision

- Ship **attention cues** (priority, your turn) and **table-feel cues** (land, stack enter, resolve, combat damage) only — no music, VO, per-card unique SFX, or shipped audio files.
- Synthesize all cues with Web Audio; unlock the shared context on lobby **Ready up** (user gesture).
- One local **Sound preference** (on/off, default on). Attention outranks table feel in the mix (quieter table-feel peaks).
- Table feel for everyone on the stream; attention only for seats that can hold priority (not **Watcher** / not eliminated **Spectator**).
- When your turn and priority arrive in the same update, play only the your-turn attention cue.

## Consequences

- Mute is a first-class escape hatch; competitive tables can silence all audio without leaving the client.
- Expanding to files or music later would be a deliberate departure from Unplugged, not an accidental creep from this ADR.
