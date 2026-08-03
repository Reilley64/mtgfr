# Forge checkout setup

Required when `./.repos/forge` is missing from the repository checkout.

## Preferred shape

**Vendored tree** (same posture as `./.repos/effect`): sparse Forge script snapshot committed into mtgfr — no nested `.git`, not gitignored.

```sh
just forge    # scripts/sync-forge.sh — replace .repos/forge from upstream tip
```

- Repo path: `./.repos/forge`
- Source: `https://github.com/Card-Forge/forge.git`
- Sparse paths:
  - `forge-gui/res/cardsfolder`
  - `forge-gui/res/tokenscripts`
  - `forge-gui/res/effects`
- After clone, the script **deletes** `.repos/forge/.git` so files are part of the mtgfr index

## Notes

- Always re-fetches upstream (`--depth 1 --filter=blob:none --sparse`) and replaces the tree.
- Expanding to `forge-game` / `forge-core` Java is out of scope unless the user asks.
- After `just forge`, commit the `.repos/forge` diff so the vendor stays shared.
