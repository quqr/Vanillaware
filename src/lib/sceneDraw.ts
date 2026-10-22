/** 预览合成：把 Quad 的时间线一帧画到 Canvas 2D。
 *
 * 只做「几何 + 贴图 + 透明度」：dstquad 的四个角经 s7 矩阵映射到画布，
 * srcquad 是 0..1 的 UV；任意四边形用两个三角形做仿射近似（单三角形内是精确仿射）。
 */

import type { Scene, SceneAnimation, SceneFrame, SceneLayer } from "./ipc";

export const FPS = 60;

export interface Camera {
  zoom: number;
  panX: number;
  panY: number;
}

type Point = [number, number];

/** 行主序 16 项矩阵作用于 xy（含平移列 3/7）；空数组视为单位矩阵。 */
export function applyMatrix(matrix: number[], x: number, y: number): Point {
  if (matrix.length < 16) return [x, y];
  return [matrix[0] * x + matrix[1] * y + matrix[3], matrix[4] * x + matrix[5] * y + matrix[7]];
}

/** 帧的解析结果：要画的图层 + 要叠的命中框。 */
export function resolveFrame(scene: Scene, animation: SceneAnimation, index: number) {
  const frame: SceneFrame | undefined = animation.frames[index];
  if (!frame) return { frame: null, layers: [] as SceneLayer[], hitboxes: [] as number[][] };

  const id = frame.attach?.id ?? -1;
  const layers = scene.keyframe[id]?.layers.filter((layer): layer is SceneLayer => layer !== null) ?? [];
  const hitboxes =
    scene.hitbox[id]?.layers
      .filter((layer): layer is { flags: string; quad: number[] } => layer !== null)
      .map((layer) => layer.quad) ?? [];

  return { frame, layers, hitboxes };
}

/** 一帧里所有图层/命中框的外接框（世界坐标，未过相机）。 */
export function frameBounds(scene: Scene, animation: SceneAnimation, index: number): [Point, Point] | null {
  const { frame, layers, hitboxes } = resolveFrame(scene, animation, index);
  if (!frame) return null;

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  const add = ([x, y]: Point) => {
    minX = Math.min(minX, x);
    minY = Math.min(minY, y);
    maxX = Math.max(maxX, x);
    maxY = Math.max(maxY, y);
  };

  for (const layer of layers) {
    for (const point of quadPoints(layer.dst, frame.matrix)) add(point);
  }
  for (const quad of hitboxes) {
    for (const point of quadPoints(quad, frame.matrix)) add(point);
  }

  if (!Number.isFinite(minX)) return null;
  return [
    [minX, minY],
    [maxX, maxY],
  ];
}

/** dstquad 的 4 个角经矩阵映射后的点。 */
export function quadPoints(quad: number[], matrix: number[]): Point[] {
  const points: Point[] = [];
  for (let i = 0; i + 1 < quad.length && i < 8; i += 2) {
    points.push(applyMatrix(matrix, quad[i], quad[i + 1]));
  }
  return points;
}

/** srcquad → 渲染器用的 8 项归一化 UV。
 *
 * 对应权威播放器 `vram_srcquad` 的口径：srcquad 可能是 0..1 的归一化值，也可能是
 * 纹理像素坐标（任一分量 > 1 即视为像素）。前者原样使用，后者除以纹理尺寸归一化。
 *
 * 归一化是必需的：Pixi 内置 mesh 着色器 `uv = (uTextureMatrix * vec3(uv,1)).xy`，
 * 4 顶点的 mesh 走 batch 时 UV 被原样写进顶点缓冲。两条路径都不接受像素坐标，
 * 传像素会让采样越界被 clamp 到贴图边缘。
 */
export function srcUvs(src: number[] | null, width: number, height: number): Float32Array {
  const out = new Float32Array(8);
  const values = src ?? [];
  let normalized = true;
  for (let i = 0; i < 8; i += 1) {
    if ((values[i] ?? 0) > 1) normalized = false;
  }
  for (let i = 0; i < 8; i += 1) {
    const value = values[i] ?? 0;
    out[i] = normalized ? value : i % 2 === 0 ? value / width : value / height;
  }
  return out;
}


/** 一帧的时长（毫秒），`time` 的单位是 60Hz 帧。 */
export function frameDurationMs(frame: SceneFrame, speed: number): number {
  return Math.max(1, (frame.time * 1000) / FPS / speed);
}

/** 把外接框缩放到画布中央（留 10% 边距）。 */
export function fitCamera(
  [[minX, minY], [maxX, maxY]]: [Point, Point],
  width: number,
  height: number,
): Camera {
  const boxWidth = Math.max(1, maxX - minX);
  const boxHeight = Math.max(1, maxY - minY);
  const zoom = Math.min((width * 0.9) / boxWidth, (height * 0.9) / boxHeight, 4);
  const centerX = (minX + maxX) / 2;
  const centerY = (minY + maxY) / 2;
  return { zoom, panX: -centerX * zoom, panY: -centerY * zoom };
}
