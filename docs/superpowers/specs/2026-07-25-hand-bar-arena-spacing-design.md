# Hand bar Arena-forward spacing

**Status:** approved design  
**Module:** `client/app/board/html/hand.ts`, `client/app/board/geometry/handBarHit.ts`, `client/app/board/motion/flights.ts` (`HAND_FACE_W`), `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md`

## Goal

Make the bottom hand/zone bar feel less cramped and closer to MTG Arena’s resting hand: larger faces, taller visible strip, wider peeks. Spacing is hand-tuned (not a single global scale factor).

## Non-goals

- Restyling the priority context bar (button sizes, companion gaps, rocker chrome)
- Responsive peek/height clamps for narrow viewports
- Mulligan overlay face sizes (separate surface)
- Changing fan tilt math, left-peek hit policy, playable borders, or zone section order
- Moving the hand bar onto canvas

## Geometry targets

Replace today’s dense constants with these Arena-forward values:

| Constant | Today | Target | Location |
|----------|------:|-------:|----------|
| `HAND_FACE_W` | 180 | **208** | `flights.ts` |
| `HAND_BAR_PEEK` | 64 | **92** | `handBarHit.ts` |
| `HAND_VISIBLE_H` | 130 | **178** | `hand.ts` |
| pip row height | 20 | **24** | `hand.ts` (`HAND_PIP_ROW_H`) |
| bar bottom padding | 12 | **16** | `hand.ts` (term in `HAND_BAR_H`) |
| `HAND_BAR_H` | 162 | **218** | derived: `178 + 24 + 16` |
| section gap | `gap-xl` | unchanged | horizontal air comes from wider peeks |

`HAND_CARD_H` stays `Math.round(HAND_FACE_W / 0.716)`. Hit height, raise translate, sticky inspect band, and drag play threshold keep deriving from these constants.

`HAND_PLAY_SLACK_PX` (96) stays unless a Scene drag test shows the play/cancel boundary feels wrong after the taller bar — then adjust in the same change.

## Behavior

- Fan, hover raise, rightmost full-face hit, and buried left-peek hit are unchanged in policy; only the numbers that feed them change.
- Anything already positioned with `HAND_BAR_H` (priority bar `bottom`, docked prompt aims, drag commit threshold, Alt-inspect sticky band) moves up with the taller bar. Those controls are not restyled in this change.
- Seven-card + commander overflow keeps today’s wrap/scroll behavior (no new clamp).

## Spec truth

Update `2026-07-20-hand-and-zone-bar.md` in the same implementation change so Behavior / Implementation Decisions cite the target geometry and note Arena-forward spacing. Cross-link this design; do not duplicate drag or playable-border rules already there.

## Testing

- Update `handBarHit.test.ts` fixtures that hardcode visible height `130` (and any peek/face assumptions that break) to use the shared constants or the new targets.
- Keep existing raise / hit-band assertions (bottom-anchored raise; rightmost full face).
- Hand Scene/unit suites stay green; add or extend one assertion that bar height / peek match the table so a silent regress to 162/64 fails.
- Drag play threshold remains expression-based on `HAND_BAR_H`; fix only if a Scene drag test fails.

## Approach note

Hand-tuned targets were chosen over a single ~1.15 scale factor so peek width and visible height can track Arena’s resting hand feel independently of face width.
