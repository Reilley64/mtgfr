# Card frame assets

These M15 frame layers and the typefaces in `../card-fonts/` were sourced from
[Investigamer/cardconjurer](https://github.com/Investigamer/cardconjurer), whose M15 assets live
under `img/frames/m15/regular/`, `img/frames/m15/crowns/`, and `fonts/`.

The frame art and fonts are Wizards of the Coast assets. They are vendored for this private game
among friends, not presented as freely licensed assets. The public site remains non-crawlable via
`client/public/robots.txt`; revisit the asset rights before using these files on a public marketing
surface or redistributing them independently.

## Upstream mapping

| Local key | Frame | P/T plate | Legend crown |
| --- | --- | --- | --- |
| `w` | `m15FrameW` | `m15PTW` | `m15CrownW` |
| `u` | `m15FrameU` | `m15PTU` | `m15CrownU` |
| `b` | `m15FrameB` | `m15PTB` | `m15CrownB` |
| `r` | `m15FrameR` | `m15PTR` | `m15CrownR` |
| `g` | `m15FrameG` | `m15PTG` | `m15CrownG` |
| `m` | `m15FrameM` | `m15PTM` | `m15CrownM` |
| `c` | `m15FrameA` | `m15PTC` | `m15CrownC` |
| `land` | `m15FrameL` | — | `m15CrownL` |

The regular set has no separate colourless frame, so `c` deliberately uses the artifact frame
(`m15FrameA`). Card Conjurer's `eldrazi.png` is the available alternative if a real card makes that
choice visibly wrong.

## Refresh procedure

Every output is a 750×1050 transparent WebP. Full frames are resized to that canvas. Upstream P/T
and crown files are bare sprites, so place them on the same canvas using the bounds from Card
Conjurer's frame pack:

- P/T: `{ x: 0.7573, y: 0.8848, w: 0.1880, h: 0.0733 }` → approximately
  `141×77+568+929` on a 750×1050 canvas.
- Crown: `{ x: 0.0274, y: 0.0191, w: 0.9454, h: 0.1667 }` → approximately
  `709×175+21+20`.

ImageMagick equivalents, with the upstream PNG in `$source` and the local file in `$output`, are:

```sh
# Full frame
magick "$source" -resize '750x1050!' -quality 82 "$output"

# Bare P/T sprite
magick -size 750x1050 canvas:none \
  \( "$source" -resize '141x77!' \) -geometry +568+929 -composite -quality 82 "$output"

# Bare crown sprite
magick -size 750x1050 canvas:none \
  \( "$source" -resize '709x175!' \) -geometry +21+20 -composite -quality 82 "$output"
```

Re-derive an opaque layer's bounds rather than nudging the renderer by eye:

```sh
magick "$output" -alpha extract -threshold 50% -format '%@' info:
```

After refreshing, run `cd client && bun run test card-render/assets card-render/frame
card-render/render`. For visual calibration against a real printing, start the dev server and use
`node client/scripts/card-render-diff.mjs <scryfall-print-id> --images` as documented in that script.
