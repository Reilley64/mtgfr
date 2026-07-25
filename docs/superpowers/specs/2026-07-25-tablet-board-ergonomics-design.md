# Tablet Board Ergonomics Design

**Status:** Accepted
**Date:** 2026-07-25
**Surface:** Board camera and short-landscape HUD ergonomics

## Goal

Make short-landscape tablet play less cramped without introducing a portrait board layout or persisted camera state.

## Design

- Add one board message, `BoardCameraZoomed({ x, y, factor })`, for both wheel and two-finger pinch zoom.
- Keep camera math in `client/app/board/geometry/camera.ts`; the message handler reuses `zoomAt()` and sets `cameraUserMoved`.
- Add `client/app/board/html/camera-gesture-mount.ts` as a pure browser-event translator. It listens for wheel and two-finger touch gestures, converts client coordinates into the board-internal screen space used by canvas pointer handlers, and prevents native scroll/zoom only when the gesture is over the board.
- Mount the gesture host from the live board view with `data-testid="board-camera-gesture-mount"`.
- Pass the hand bar height into `fitCamera()` during board sync so cold-fit framing reserves the bottom hand lane.
- Add `.hit-quiet` to tiny board HUD close controls that need coarse-pointer hit targets.

## Non-goals

- No portrait reflow.
- No sessionStorage or persisted camera.
- No new zoom controls, minimap, or camera reset UI.

## Tests

- Unit tests cover wheel factor, pinch factor, pinch center, and client-to-board coordinate conversion.
- Board story tests cover zoom setting `cameraUserMoved` and preventing later sync refits.
- Scene tests cover the live gesture mount and the visible `.hit-quiet` chrome classes.
