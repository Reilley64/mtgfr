# Attachment Hover Rise Design

## Goal

Make an attached permanent easier to inspect by sliding it farther out from under its host while the pointer hovers it. The interaction should feel like the hand action bar's short card rise, while preserving the battlefield's authoritative attachment layering.

## Scope

This behavior applies to every battlefield permanent with a valid `attached_to` host, including Auras, Equipment, and future attachable permanent kinds. It does not apply to unattached permanents or attachments whose host cannot be resolved into a valid attachment stack.

Only the topmost attached permanent under the pointer responds. Hovering one attachment moves only that card. Descendants attached to that card and its parent, sibling, and root-host cards remain at their resting positions.

## Interaction

An attached permanent rises one existing attachment-peek step: 20% of its current rendered side length. The direction is relative to the attachment stack's root host controller and follows the stack's existing exposure direction. An attachment controlled by another player still follows its root host's controller/avatar so it always slides farther out from under that host:

- a card controlled from the viewer-side seat moves upward on screen, toward the table center;
- a card controlled from the mirrored seat moves downward on screen, toward the table center;
- other supported seat layouts use the same away-from-root-host-controller-avatar, toward-table-center rule.

The rise and return each ease over 120 milliseconds. When reduced motion is requested, the card snaps to the destination without interpolation.

The card remains at its existing position in attachment-subtree paint order throughout the interaction. Hover does not bring it above its parent attachment, sibling attachments, root host, avatars, arrows, HTML overlays, or screen-motion layers. Tap state and crowded-row scale are unchanged.

Hover ends when the pointer leaves the card's stable hover footprint, a drag or camera pan begins, the card is no longer a valid attachment, or the visible board state disappears. The return uses the same short transition.

## Stable Hit Testing

Attachment hover uses a pure, presentation-aware hit helper over the existing `RenderCard[]` paint order. It walks cards from topmost to bottommost, so a parent or root host still wins wherever its own footprint covers an attachment.

For the currently hovered attachment, the helper accepts the union of:

- its original resting footprint; and
- its fully shifted destination footprint.

This union prevents the hover from flickering off while the card moves away from the small exposed strip where the interaction began. Other cards retain their ordinary resting footprints. A newly hovered attachment must still be the topmost hittable card at the pointer position.

Logical battlefield layout remains authoritative and unchanged. The transient shift does not affect camera fitting, table or seat bounds, combat or stack-arrow endpoints, flight destinations, clustering, attachment depth allocation, or game actions.

## State and Rendering

`BoardModel` owns one semantic value, `hoveredAttachmentId`, updated through ordinary Foldkit pointer messages. Pointer movement derives it from the current logical cards and the attachment-aware hit helper. Pointer down into drag or pan behavior clears it.

`BitmapFrame` publishes the hovered attachment ID. The resting-paint snapshot includes that value so entering or leaving hover invalidates the resting layer.

The bitmap Mount owns the transient animation progress because it is renderer timing, not game or interaction state. When the published hover ID changes, the Mount interpolates the affected attachment's controller-relative offset and repaints the resting layer until the 120 millisecond transition settles. It paints the translated card at the same array index as before; it never extracts or re-paints the card on a higher layer.

If the ID is missing, stale, unattached, or otherwise invalid, rendering safely falls back to the resting pose. The next board fold clears stale semantic hover state.

## Tests

Focused pure tests shall prove:

- the rise is exactly 20% of the card's current side;
- normal and mirrored root-host controllers move away from their avatar toward the table center;
- a cross-controller attachment follows its root host controller rather than its own controller;
- only the hovered attachment moves, including when it hosts another attachment;
- card size, tap state, and `RenderCard[]` order do not change;
- the stable hit union covers the original exposed strip and shifted destination;
- the existing host and attachment-subtree topmost rules still win in overlap;
- leaving the union clears hover; and
- missing IDs and unattached cards remain at rest.

Bitmap Mount tests shall prove enter and return interpolation, resting-layer repaint invalidation, reduced-motion snapping, and unchanged paint order.

A board Scene interaction test shall exercise pointer hover and leave and assert the user-visible published attachment hover state. Existing mirrored attachment hit coverage shall be extended while touching this area.

## Living Documentation

At implementation completion, update `openspec/specs/game-board/spec.md` and `docs/CLIENT_CANVAS_MAP.md` to state that attachment hover is controller-relative, presentation-only, stable-hit-tested, and cannot change attachment subtree layering.
