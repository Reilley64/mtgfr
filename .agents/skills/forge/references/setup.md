# Forge checkout setup

Required when `./.repos/forge` is missing from the repository checkout.

## Preferred shape

**Vendored tree** (same posture as `./.repos/effect`): sparse Forge script snapshot committed into mtgfr — no nested `.git`, not gitignored.

```sh
just forge    # scripts/sync-forge.sh — replace .repos/forge from upstream tip
```

- Repo path: `./.repos/forge`
- Source: `https://github.com/Card-Forge/forge.git`
- Sparse paths (then pruned of cone junk):
  - `forge-gui/res/cardsfolder`
  - `forge-gui/res/tokenscripts`
- Keeps root `LICENSE`, `README.md`, and writes `VENDOR_REVISION` (upstream SHA)
- After clone, the script **deletes** `.repos/forge/.git` so files are part of the mtgfr index
- Replaces via `.new` / `.old` swap so a crash mid-move does not leave an empty tree without a retry path

## Notes

- Always re-fetches upstream (`--depth 1 --filter=blob:none --sparse`) and replaces the tree.
- Do **not** vendor `forge-gui/res/effects` — that directory is UI GIFs, not scripts.
- Expanding to `forge-game` / `forge-core` Java is out of scope unless the user asks.
- After `just forge`, commit the `.repos/forge` diff so the vendor stays shared.
- `.dockerignore` excludes `.repos` so image build context stays lean.
