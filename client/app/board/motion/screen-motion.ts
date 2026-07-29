import { handFlightScale } from "./flights";

export type DragGhostZone = "hand" | "command" | "graveyard" | "exile";

export type DragGhost = {
  print: string;
  name: string;
  x: number;
  y: number;
  scale: number;
  zone: DragGhostZone;
};

export type HandDragPose = {
  name: string;
  print: string;
  x: number;
  y: number;
  zone?: DragGhostZone;
};

export function dragGhostFromHandDrag(drag: HandDragPose, zoom: number, faceW?: number): DragGhost {
  return {
    print: drag.print,
    name: drag.name,
    x: drag.x,
    y: drag.y,
    scale: handFlightScale(zoom, faceW),
    zone: drag.zone ?? "hand",
  };
}
