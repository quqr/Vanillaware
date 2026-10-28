import type { Scene, SceneAnimation, SceneAttach, SceneFrame, SceneLayer } from "./ipc";

export const FPS = 60;

export interface Camera {
  zoom: number;
  panX: number;
  panY: number;
}

type Point = [number, number];

export function applyMatrix(matrix: number[], x: number, y: number): Point {
  if (matrix.length < 16) return [x, y];
  return [
    matrix[0] * x + matrix[1] * y + matrix[2] + matrix[3],
    matrix[4] * x + matrix[5] * y + matrix[6] + matrix[7],
  ];
}

export function quadPoints(quad: number[], matrix: number[]): Point[] {
  const points: Point[] = [];
  for (let i = 0; i + 1 < quad.length && i < 8; i += 2) {
    points.push(applyMatrix(matrix, quad[i], quad[i + 1]));
  }
  return points;
}

export interface FrameTargets {
  keyframes: number[];
  hitboxes: number[];
}

export function frameTargets(attach: SceneAttach | null): FrameTargets {
  if (!attach || attach.id < 0) return { keyframes: [], hitboxes: [] };
  switch (attach.kind) {
    case "keyframe":
      return { keyframes: [attach.id], hitboxes: [] };
    case "hitbox":
      return { keyframes: [], hitboxes: [attach.id] };
    case "slot":
      return { keyframes: [attach.id], hitboxes: [attach.id] };
    default:
      return { keyframes: [], hitboxes: [] };
  }
}

export interface BoneState {
  frameIndex: number;
  elapsed: number;
  done: boolean;
}

export function newBoneStates(count: number): BoneState[] {
  return Array.from({ length: count }, () => ({ frameIndex: 0, elapsed: 0, done: false }));
}

export function boneAnimation(
  animations: Map<number, SceneAnimation>,
  attach: SceneAttach | null,
): SceneAnimation | null {
  if (!attach || attach.kind !== "animation") return null;
  return animations.get(attach.id) ?? null;
}

export function animationIndex(scene: Scene): Map<number, SceneAnimation> {
  const map = new Map<number, SceneAnimation>();
  for (const animation of scene.animations) map.set(animation.id, animation);
  return map;
}

export function frameDurationMs(frame: SceneFrame, speed: number): number {
  return Math.max(1, (frame.time * 1000) / FPS / speed);
}

export function stepBone(
  animation: SceneAnimation,
  state: BoneState,
  deltaMs: number,
  speed: number,
): boolean {
  if (animation.frames.length === 0 || state.done) return false;
  let changed = false;
  state.elapsed += deltaMs;
  for (let guard = 0; guard < 512; guard += 1) {
    const frame = animation.frames[state.frameIndex];
    if (!frame) break;
    const span = frameDurationMs(frame, speed);
    if (state.elapsed < span) break;
    state.elapsed -= span;

    let next = state.frameIndex + 1;
    if (next >= animation.frames.length) {
      if (animation.loopId >= 0 && animation.loopId < animation.frames.length) {
        next = animation.loopId;
      } else {
        state.done = true;
        state.frameIndex = animation.frames.length - 1;
        break;
      }
    }
    state.frameIndex = next;
    changed = true;
  }
  return changed;
}

export interface BoneLayer {
  layer: SceneLayer;
  matrix: number[];
  color: string | null;
}

export interface BoneHitbox {
  quad: number[];
  matrix: number[];
}

export interface ResolvedScene {
  layers: BoneLayer[];
  hitboxes: BoneHitbox[];

  bones: number;
}

export function resolveBones(
  scene: Scene,
  bones: SceneAttach[],
  states: BoneState[],
  animations: Map<number, SceneAnimation>,
): ResolvedScene {
  const layers: BoneLayer[] = [];
  const hitboxes: BoneHitbox[] = [];
  let drawn = 0;

  for (let index = 0; index < bones.length; index += 1) {
    const animation = boneAnimation(animations, bones[index]);
    if (!animation || !animation.frames.length) continue;
    const state = states[index];
    const frame = animation.frames[Math.min(state?.frameIndex ?? 0, animation.frames.length - 1)];
    if (!frame) continue;
    drawn += 1;

    const targets = frameTargets(frame.attach);
    for (const id of targets.keyframes) {
      const keyframe = scene.keyframe[id];
      if (!keyframe) continue;
      for (const layer of keyframe.layers) {
        if (!layer) continue;
        layers.push({ layer, matrix: frame.matrix, color: frame.color });
      }
    }
    for (const id of targets.hitboxes) {
      const hitbox = scene.hitbox[id];
      if (!hitbox) continue;
      for (const layer of hitbox.layers) {
        if (!layer) continue;
        hitboxes.push({ quad: layer.quad, matrix: frame.matrix });
      }
    }
  }

  return { layers, hitboxes, bones: drawn };
}

export function boundsOf(
  layers: BoneLayer[],
  hitboxes: BoneHitbox[],
): [Point, Point] | null {
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
  for (const item of layers) for (const point of quadPoints(item.layer.dst, item.matrix)) add(point);
  for (const item of hitboxes) for (const point of quadPoints(item.quad, item.matrix)) add(point);
  if (!Number.isFinite(minX)) return null;
  return [
    [minX, minY],
    [maxX, maxY],
  ];
}

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

export function skeletonLength(
  bones: SceneAttach[],
  animations: Map<number, SceneAnimation>,
): number {
  let longest = 0;
  for (const bone of bones) {
    const animation = boneAnimation(animations, bone);
    if (animation) longest = Math.max(longest, animation.frames.length);
  }
  return longest;
}
