# Tasks

- [x] Carry `is_token`, `legendary`, and `colors` on `ObjectView` (proto → prost → schema projection), with the face-down redaction split covered by projection tests
- [x] Add the frame/slot geometry: `slotRects(variant, face)` with a `permanent` (square) variant, the token arch and missing title, and the legend crown
- [x] Render a card face offscreen from frame assets, art, and name; cache it by the drawn facts (`CardFaceCache`, `sharedFaceCache`) and clear it once card fonts load
- [x] Make the resting permanent square (`CARD_W`/`CARD_H`) while drag ghosts and flights keep the card-shaped footprint
- [x] Paint the rendered face in `paintCard`/`paintFaceUp`, falling back to the printed image until the face lands and repainting when it does; keep face-down on the card back
- [x] Verify live at four seats: square art-first tiles, tap rotation, P/T badge and commander gold painting over, Alt-hold inspect still showing the printed card
