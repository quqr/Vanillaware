import type { Scene, SceneAnimation, SceneAttach, SceneFrame, SceneLayer } from "./ipc";

export const FPS = 60;

export interface Camera {
  zoom: number;
  panX: number;
  panY: number;
}

type Point = [number, number];

/**
 * Applies an engine matrix. `f_700058d708` builds it row major with the
 * translation in `m[12..15]` and `FUN_700057e970` evaluates `p' = p * m`, so the
 * corner is a row vector: `x' = m0*x + m4*y + m12`.
 */
export function applyMatrix(matrix: number[], x: number, y: number): Point {
  if (matrix.length < 16) return [x, y];
  return [
    matrix[0] * x + matrix[4] * y + matrix[12],
    matrix[1] * x + matrix[5] * y + matrix[13],
  ];
}

/**
 * The `s2` / `s1` records are `[centre(2), corner(8), first corner again(2)]`,
 * so the drawable quad is `[2..10]`. An eight float value is taken as-is.
 */
export function quadCorners(quad: number[]): number[] {
  if (quad.length >= 12) return quad.slice(2, 10);
  return quad.slice(0, 8);
}

export function quadPoints(quad: number[], matrix: number[]): Point[] {
  const corners = quadCorners(quad);
  const points: Point[] = [];
  for (let i = 0; i + 1 < corners.length; i += 2) {
    points.push(applyMatrix(matrix, corners[i], corners[i + 1]));
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
  /** Remaining time inside the current frame, in engine units (1000 = one 60 Hz tick). */
  remaining: number;
  done: boolean;
}

/** One engine tick is 1/60 s and advances the animation clock by 1000 units. */
export const TICK_UNITS = 1000;
/** Engine units per wall-clock millisecond (1000 units per 16.666 ms). */
export const UNITS_PER_MS = (TICK_UNITS * FPS) / 1000;

export function newBoneStates(count: number): BoneState[] {
  return Array.from({ length: count }, () => ({ frameIndex: 0, remaining: -1, done: false }));
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

export function frameUnits(frame: SceneFrame): number {
  return Math.max(0, frame.time) * TICK_UNITS;
}

export function frameFlags(frame: SceneFrame): number {
  const value = Number(frame.flags);
  return Number.isFinite(value) ? value : 0;
}

function clampIndex(index: number, length: number): number {
  if (length <= 0) return 0;
  return Math.max(0, Math.min(index, length - 1));
}

/**
 * Advances one track by `deltaMs`, mirroring `f_700058e970`: the countdown of
 * the current frame is consumed first, the leftover time keeps advancing so a
 * long stall skips the frames that would have been missed.
 */
export function stepBone(
  animation: SceneAnimation,
  state: BoneState,
  deltaMs: number,
  speed: number,
): boolean {
  const frames = animation.frames;
  if (frames.length === 0 || state.done) return false;
  if (state.remaining < 0) {
    state.remaining = frameUnits(frames[clampIndex(state.frameIndex, frames.length)]);
  }

  let units = deltaMs * UNITS_PER_MS * speed;
  let changed = false;

  for (let guard = 0; guard < 1024 && units > 0; guard += 1) {
    if (state.remaining > units) {
      state.remaining -= units;
      return changed;
    }
    units -= Math.max(0, state.remaining);

    const frame = frames[clampIndex(state.frameIndex, frames.length)];
    if (!frame) break;
    const flags = frameFlags(frame);
    let next: number;

    if (flags & 0x100) {
      // Global rewind: every track restarts from its first frame.
      next = 0;
    } else if (flags & 0x4) {
      next = animation.loopId >= 0 ? animation.loopId : state.frameIndex + 1;
    } else if (flags & 0x8 || flags & 0x1800) {
      // Frame flags that stop the track on this frame.
      state.done = true;
      break;
    } else {
      next = state.frameIndex + 1;
      if (next >= frames.length) {
        if (animation.loopId >= 0 && animation.loopId < frames.length) {
          next = animation.loopId;
        } else {
          state.done = true;
          state.frameIndex = frames.length - 1;
          break;
        }
      }
    }

    if (next >= frames.length) {
      state.done = true;
      state.frameIndex = frames.length - 1;
      break;
    }
    state.frameIndex = next;
    state.remaining = frameUnits(frames[next]);
    changed = true;
  }
  return changed;
}

/**
 * Interpolation coefficient of the current frame, mirroring `f_700058e80c`.
 *
 * `in_rate == 1` uses the continuous ramp `1 - remaining/duration`; every other
 * value quantises the elapsed time to `1/rate` steps, which is why some clips
 * look noticeably steppier than others.
 */
export function boneT(animation: SceneAnimation, state: BoneState): number {
  const frames = animation.frames;
  const frame = frames[clampIndex(state.frameIndex, frames.length)];
  if (!frame) return 0;
  const duration = Math.max(1, frame.time);
  const total = duration * TICK_UNITS;
  const remaining = state.remaining < 0 ? total : Math.max(0, Math.min(state.remaining, total));
  const rate = frame.inRate ?? 1;

  let t: number;
  if (rate <= 1) {
    t = 1 - remaining / total;
  } else {
    const elapsedWhole = duration - Math.ceil(remaining / TICK_UNITS);
    t = (Math.trunc(elapsedWhole / rate) * rate) / duration;
  }
  return Math.max(0, Math.min(1, t));
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

/** Catmull-Rom weights used by `f_700058c1c8`, ordered as prev / cur / next / next+1. */
export function splineWeights(t: number): [number, number, number, number] {
  const c0 = t * t;
  const c1 = c0 * t;
  return [
    -0.5 * c1 + c0 - 0.5 * t,
    1.5 * c1 - 2.5 * c0 + 1,
    -1.5 * c1 + 2 * c0 + 0.5 * t,
    0.5 * c1 - 0.5 * c0,
  ];
}

export interface S7Values {
  move: number[];
  rotate: number[];
  scale: number[];
}

function pick(values: number[] | undefined, index: number): number {
  return values?.[index] ?? 0;
}

/**
 * `f_700058c0a0` mixes the eight `s7` floats (move 3, rotate 3, scale 2) and the
 * four fog bytes; the rotation matrix is rebuilt from the mixed angles
 * afterwards. Interpolating matrix elements instead would drift off the Euler
 * path the engine actually takes.
 */
export function mixS7(
  prev: SceneFrame | undefined,
  cur: SceneFrame,
  next: SceneFrame | undefined,
  next2: SceneFrame | undefined,
  t: number,
  mixId: number,
): S7Values {
  const out: S7Values = { move: [], rotate: [], scale: [] };
  const lengths: Record<keyof S7Values, number> = { move: 3, rotate: 3, scale: 2 };
  const [w0, w1, w2, w3] = splineWeights(t);

  for (const key of Object.keys(lengths) as (keyof S7Values)[]) {
    const count = lengths[key];
    const values: number[] = [];
    for (let i = 0; i < count; i += 1) {
      const a = pick(cur[key], i);
      const b = next ? pick(next[key], i) : a;
      if (mixId === 1) {
        values.push(a + (b - a) * t);
      } else if (mixId === 2) {
        const p = prev ? pick(prev[key], i) : a;
        const c = next2 ? pick(next2[key], i) : b;
        values.push(p * w0 + a * w1 + b * w2 + c * w3);
      } else {
        values.push(a);
      }
    }
    out[key] = values;
  }
  return out;
}

const ENGINE_IDENTITY: number[] = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1];

function mul44(a: number[], b: number[]): number[] {
  const out = new Array<number>(16).fill(0);
  for (let row = 0; row < 4; row += 1) {
    for (let col = 0; col < 4; col += 1) {
      let sum = 0;
      for (let k = 0; k < 4; k += 1) sum += a[row * 4 + k] * b[k * 4 + col];
      out[row * 4 + col] = sum;
    }
  }
  return out;
}

/** Row-vector rotation matrices, the transposes of the usual column-vector forms. */
function rotationMatrix(axis: "x" | "y" | "z", radian: number): number[] {
  const sin = Math.sin(radian);
  const cos = Math.cos(radian);
  const m = ENGINE_IDENTITY.slice();
  if (axis === "z") {
    m[0] = cos;
    m[1] = sin;
    m[4] = -sin;
    m[5] = cos;
  } else if (axis === "x") {
    m[5] = cos;
    m[6] = sin;
    m[9] = -sin;
    m[10] = cos;
  } else {
    m[0] = cos;
    m[2] = -sin;
    m[8] = sin;
    m[10] = cos;
  }
  return m;
}

/**
 * `f_700058d708` + `f_700058e014`: scale, then Z, X and Y rotations
 * right-multiplied in that order, translation written into the last row, and the
 * flip bits negating a whole column afterwards.
 */
export function engineMatrix(values: S7Values, flags: number): number[] {
  let m = ENGINE_IDENTITY.slice();
  m[0] = pick(values.scale, 0);
  m[5] = pick(values.scale, 1);
  m = mul44(m, rotationMatrix("z", pick(values.rotate, 2)));
  m = mul44(m, rotationMatrix("x", pick(values.rotate, 0)));
  m = mul44(m, rotationMatrix("y", pick(values.rotate, 1)));
  m[12] = pick(values.move, 0);
  m[13] = pick(values.move, 1);
  m[14] = pick(values.move, 2);
  if (flags & 0x1) {
    for (let row = 0; row < 4; row += 1) m[row * 4] = -m[row * 4];
  }
  if (flags & 0x2) {
    for (let row = 0; row < 4; row += 1) m[row * 4 + 1] = -m[row * 4 + 1];
  }
  return m;
}

function hexBytes(color: string): [number, number, number, number] {
  const hex = color.replace("#", "");
  const parts: [number, number, number, number] = [255, 255, 255, 255];
  for (let i = 0; i < 4; i += 1) {
    const slice = hex.slice(i * 2, i * 2 + 2);
    if (slice.length === 2) parts[i] = Number.parseInt(slice, 16);
  }
  return parts;
}

function byteHex(value: number): string {
  return Math.max(0, Math.min(255, Math.round(value)))
    .toString(16)
    .padStart(2, "0");
}

export function mixColor(
  prev: string | null | undefined,
  cur: string | null | undefined,
  next: string | null | undefined,
  next2: string | null | undefined,
  t: number,
  mixId: number,
): string | null {
  if (!cur) return cur ?? null;
  if (mixId !== 1 && mixId !== 2) return cur;
  const a = hexBytes(cur);
  const n = hexBytes(next ?? cur);
  const p = hexBytes(prev ?? cur);
  const q = hexBytes(next2 ?? next ?? cur);
  const [w0, w1, w2, w3] = splineWeights(t);
  const out: string[] = [];
  for (let i = 0; i < 4; i += 1) {
    const value =
      mixId === 1 ? a[i] + (n[i] - a[i]) * t : p[i] * w0 + a[i] * w1 + n[i] * w2 + q[i] * w3;
    out.push(byteHex(value));
  }
  return `#${out.join("")}`;
}

/** `s2` / `s1` interpolation, gated by `in_s6` and the same weights as the rect. */
export function mixQuad(
  prev: number[] | null | undefined,
  cur: number[] | null | undefined,
  next: number[] | null | undefined,
  next2: number[] | null | undefined,
  t: number,
  mixId: number,
): number[] {
  const base = cur ?? [];
  if (mixId !== 1 && mixId !== 2) return base;
  const length = Math.max(base.length, (next ?? []).length);
  const out = new Array<number>(length);
  if (mixId === 1) {
    for (let i = 0; i < length; i += 1) {
      const a = base[i] ?? 0;
      const b = next?.[i] ?? a;
      out[i] = a + (b - a) * t;
    }
    return out;
  }
  const [w0, w1, w2, w3] = splineWeights(t);
  for (let i = 0; i < length; i += 1) {
    const a = base[i] ?? 0;
    const b = prev?.[i] ?? a;
    const c = next?.[i] ?? a;
    const d = next2?.[i] ?? c;
    out[i] = b * w0 + a * w1 + c * w2 + d * w3;
  }
  return out;
}

function keyframeOf(scene: Scene, frame: SceneFrame | undefined) {
  if (!frame) return null;
  const targets = frameTargets(frame.attach);
  for (const id of targets.keyframes) {
    const keyframe = scene.keyframe[id];
    if (keyframe) return keyframe;
  }
  return null;
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
    const frames = animation.frames;
    const at = clampIndex(state?.frameIndex ?? 0, frames.length);
    const cur = frames[at];
    if (!cur) continue;
    drawn += 1;

    const prev = frames[Math.max(0, at - 1)];
    const next = frames[Math.min(at + 1, frames.length - 1)];
    const next2 = frames[Math.min(at + 2, frames.length - 1)];
    const t = state ? boneT(animation, state) : 0;

    const values = mixS7(prev, cur, next, next2, t, cur.matrixMixId);
    const matrix = engineMatrix(values, frameFlags(cur));
    const color = mixColor(prev.color, cur.color, next.color, next2.color, t, cur.matrixMixId);

    const targets = frameTargets(cur.attach);
    for (const id of targets.keyframes) {
      const keyframe = scene.keyframe[id];
      if (!keyframe) continue;
      const prevKeyframe = keyframeOf(scene, prev);
      const nextKeyframe = keyframeOf(scene, next);
      const next2Keyframe = keyframeOf(scene, next2);

      for (let layerIndex = 0; layerIndex < keyframe.layers.length; layerIndex += 1) {
        const layer = keyframe.layers[layerIndex];
        if (!layer) continue;
        const prevLayer = prevKeyframe?.layers[layerIndex] ?? null;
        const nextLayer = nextKeyframe?.layers[layerIndex] ?? null;
        const next2Layer = next2Keyframe?.layers[layerIndex] ?? null;

        // `in_s6` drives the same three-state mix for the `s6` rect and for the
        // per-layer `s2`/`s1`/`s0` records (the rect equals their union bbox in
        // 98.6% of the corpus), so the quad follows the same weights.
        const dst = mixQuad(prevLayer?.dst, layer.dst, nextLayer?.dst, next2Layer?.dst, t, cur.dstQuadMixId);
        const src = layer.src
          ? mixQuad(prevLayer?.src, layer.src, nextLayer?.src, next2Layer?.src, t, cur.dstQuadMixId)
          : null;
        const fog = layer.fog
          ? layer.fog.map(
              (corner, cornerIndex) =>
                mixColor(
                  prevLayer?.fog?.[cornerIndex],
                  corner,
                  nextLayer?.fog?.[cornerIndex],
                  next2Layer?.fog?.[cornerIndex],
                  t,
                  cur.fogQuadMixId,
                ) ?? corner,
            )
          : null;

        layers.push({ layer: { ...layer, dst, src, fog }, matrix, color });
      }
    }

    for (const id of targets.hitboxes) {
      const hitbox = scene.hitbox[id];
      if (!hitbox) continue;
      for (const layer of hitbox.layers) {
        if (!layer) continue;
        hitboxes.push({ quad: layer.quad, matrix });
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
  const values = src ? quadCorners(src) : [];
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
