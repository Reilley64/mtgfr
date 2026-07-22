import { Canvas } from "foldkit";

export function feltShapes(width: number, height: number): Canvas.Shape[] {
  const speckles = Array.from({ length: 24 }, (_, i) => {
    const x = (((i * 73) % 97) / 97) * width;
    const y = (((i * 41) % 89) / 89) * height;
    return Canvas.Rect({ x, y, width: 2, height: 2, fill: "#1a2a22" });
  });

  return [
    Canvas.Rect({ x: 0, y: 0, width, height, fill: "#0B1310" }),
    ...speckles,
    Canvas.Rect({ x: 0, y: 0, width, height, fill: "rgba(0,0,0,0.18)" }),
  ];
}
