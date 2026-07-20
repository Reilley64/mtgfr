# 0031 — Card id is Scryfall oracle id; Printing is art preference

Card identity is Scryfall’s oracle id (`CardDef.id`); a **Printing** is a Scryfall card UUID used only for art. Decks store `(id, count, print)` with `print` required; visible game objects carry `print` so every client shows the same art. The engine stays print-agnostic. Images resolve CDN-only by Printing UUID (missing art is a broken image). Card `default_print` is Scryfall’s preferred print (`/cards/named`); precon fixtures stamp explicit SoC/Archidekt prints. No live server yet — wipe user decks and break `/v1` deck DTOs in place rather than expand-then-contract or `/v2`.

Supersedes the name→Scryfall-id map role of [0015](0015-card-imagery-via-self-hosted-cdn-and-name-id-map.md) for identity; CDN layout by UUID remains.
