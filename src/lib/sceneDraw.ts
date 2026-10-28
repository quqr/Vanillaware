/** 预览合成：把 MBS 的 Quad 时间线摊成「这一帧要画哪些图层」。
 *
 * 口径对照权威播放器 `quad_player_mobile/src/quad-draw.js`：
 * - **播放单位是 skeleton，不是 animation**。`draw_skeleton` 遍历 `skeleton.bone`，
 *   每根骨各自求当前帧再递归画 attach；只播 `animation[0]` 会漏掉副骨/叠加层
 *   （语料 678 个可播模型里，354 个的首骨图层数不足总量的 90%，最差为 0）。
 * - **attach 要看 `type`**：`keyframe` 取 keyframe，`hitbox` 取 hitbox，
 *   `slot` 是 (keyframe, hitbox) 的同号对。原来只按 id 去查 keyframe，
 *   88k 条 hitbox attach 会画出一张无关的贴图。
 * - **顶点变换是 `quad_multi4`**：`x' = m0*x + m1*y + m2 + m3`，
 *   `y' = m4*x + m5*y + m6 + m7`。`m2`/`m6` 是 z 列投影（绕 x/y 轴旋转时非零），
 *   漏掉会把侧身姿态平移错。
 * - 顶点色 = 时间线 `color` × 该图层 `fogquad`，逐分量相乘（含 alpha）。
 *
 * 有意保留的差异（见 `docs/adr/0024`）：`blend_id` 1..3 的表项 PHP 未解出，
 * 这里一律按普通混合画（权威播放器会直接跳过；宁可混合不精确也别丢部件）。
 */

import type { Scene, SceneAnimation, SceneAttach, SceneFrame, SceneLayer } from "./ipc";

export const FPS = 60;

export interface Camera {
  zoom: number;
  panX: number;
  panY: number;
}

type Point = [number, number];

/** 行主序 16 项矩阵作用于 xy（含 z 列 2/6 与平移列 3/7）；空数组视为单位矩阵。 */
export function applyMatrix(matrix: number[], x: number, y: number): Point {
  if (matrix.length < 16) return [x, y];
  return [
    matrix[0] * x + matrix[1] * y + matrix[2] + matrix[3],
    matrix[4] * x + matrix[5] * y + matrix[6] + matrix[7],
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

/** 一条 attach 指向的 keyframe / hitbox 下标。 */
export interface FrameTargets {
  keyframes: number[];
  hitboxes: number[];
}

/** 对应权威播放器 `draw_attach` 的分派。`slot` 的两半共用同一个 s6 下标。 */
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
      // `skeleton` / `list` / `__MIX` 这类跨 mbs 的挂载在预览器里不展开。
      return { keyframes: [], hitboxes: [] };
  }
}

/** 一根骨的播放游标：时间线帧号 + 该帧内已过毫秒。 */
export interface BoneState {
  frameIndex: number;
  elapsed: number;
  done: boolean;
}

export function newBoneStates(count: number): BoneState[] {
  return Array.from({ length: count }, () => ({ frameIndex: 0, elapsed: 0, done: false }));
}

/** 骨（attach）→ 它对应的时间线；非 animation 的骨返回 null。 */
export function boneAnimation(
  animations: Map<number, SceneAnimation>,
  attach: SceneAttach | null,
): SceneAnimation | null {
  if (!attach || attach.kind !== "animation") return null;
  return animations.get(attach.id) ?? null;
}

/** `scene.animations` 建按 id 取的下标（`Pad::Empty` 已被后端滤掉，id 不连续）。 */
export function animationIndex(scene: Scene): Map<number, SceneAnimation> {
  const map = new Map<number, SceneAnimation>();
  for (const animation of scene.animations) map.set(animation.id, animation);
  return map;
}

/** 一帧的时长（毫秒），`time` 的单位是 60Hz 帧。 */
export function frameDurationMs(frame: SceneFrame, speed: number): number {
  return Math.max(1, (frame.time * 1000) / FPS / speed);
}

/** 推进一根骨；返回是否换了帧。每根骨有自己的 loop 点（权威播放器 `anim_current`）。 */
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

/** 要画的一个图层：图层本体 + 它所属骨这一帧的矩阵与颜色。 */
export interface BoneLayer {
  layer: SceneLayer;
  matrix: number[];
  color: string | null;
}

/** 要画的一个命中框。 */
export interface BoneHitbox {
  quad: number[];
  matrix: number[];
}

export interface ResolvedScene {
  layers: BoneLayer[];
  hitboxes: BoneHitbox[];
  /** 真正出了图层的骨数（HUD 用）。 */
  bones: number;
}

/** 按骨架各骨的当前帧解析出整帧要画的图层。 */
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

/** 一组图层的世界包围盒（未过相机）。 */
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

/** 骨架里最长的骨有多少帧（时间轴与 HUD 用）。 */
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
