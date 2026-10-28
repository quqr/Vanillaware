import { useCallback, useEffect, useRef, useState } from "react";
import { Texture } from "pixi.js";
import { api, type Scene, type SceneAnimation, type SceneAttach, type SceneLayer } from "../../lib/ipc";
import {
  NO_TINT,
  PixiRenderer,
  multiplyTint,
  parseColor,
  textureFromPng,
  type RenderFrame,
  type RenderLayer,
  type Tint,
} from "../../lib/pixiScene";
import {
  animationIndex,
  boundsOf,
  boneAnimation,
  fitCamera,
  newBoneStates,
  quadPoints,
  resolveBones,
  skeletonLength,
  srcUvs,
  stepBone,
  type BoneState,
} from "../../lib/sceneDraw";

/** 播放位置：全部放在 ref 里，由 Pixi ticker 推进，避免 60Hz 触发 React 重渲染。
 *
 * 播放单位是 **skeleton**：一根骨一个游标，各骨按自己的 loop 点循环
 * （权威播放器 `draw_skeleton` + `anim_current` 的行为）。
 */
interface Playhead {
  skeletonIndex: number;
  bones: BoneState[];
  playing: boolean;
  speed: number;
  dirty: boolean;
}

interface Hud {
  frame: number;
  frames: number;
  bones: number;
  layerCount: number;
  time: number;
  attach: SceneAttach | null;
}

/** 动画播放查看器：骨架列表 + PixiJS 合成 + 图层面板 + 时间轴（ADR-0007 / 0011 / 0024）。
 *
 * 受控组件：目标 `.mbs` 与平台 `tag` 都由查看器面板传入，不再读全局选中态。
 */
export function ScenePlayer({ path, tag }: { path: string; tag: string }) {
  const [scene, setScene] = useState<Scene | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [textureError, setTextureError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const [skeletonIndex, setSkeletonIndex] = useState(0);
  const [playing, setPlaying] = useState(true);
  const [speed, setSpeed] = useState(1);
  const [showHitbox, setShowHitbox] = useState(false);
  const [showOutline, setShowOutline] = useState(false);
  const [applyTint, setApplyTint] = useState(true);
  const [hud, setHud] = useState<Hud>({
    frame: 0,
    frames: 0,
    bones: 0,
    layerCount: 0,
    time: 0,
    attach: null,
  });
  const [zoomLabel, setZoomLabel] = useState(1);

  const wrapRef = useRef<HTMLDivElement>(null);
  const rendererRef = useRef<PixiRenderer | null>(null);
  const sceneRef = useRef<Scene | null>(null);
  const animationsRef = useRef<Map<number, SceneAnimation>>(new Map());
  const texturesRef = useRef(new Map<number, Texture>());
  const inflightTexRef = useRef(new Map<number, Promise<void>>());
  const playheadRef = useRef<Playhead>({
    skeletonIndex: 0,
    bones: [],
    playing: true,
    speed: 1,
    dirty: true,
  });
  const optionsRef = useRef({ showHitbox: false, showOutline: false, applyTint: true });
  const hudAtRef = useRef(0);

  // 把 React 控件状态镜像到 ref，ticker 闭包读 ref 不读 state。
  useEffect(() => {
    playheadRef.current.playing = playing;
    playheadRef.current.speed = speed;
  }, [playing, speed]);

  useEffect(() => {
    optionsRef.current = { showHitbox, showOutline, applyTint };
    playheadRef.current.dirty = true;
  }, [showHitbox, showOutline, applyTint]);

  /** 场景加载。 */
  useEffect(() => {
    if (!path) {
      setScene(null);
      sceneRef.current = null;
      return;
    }

    let alive = true;
    setLoading(true);
    setError(null);
    setTextureError(null);
    setSkeletonIndex(0);

    api
      .previewScene(path, tag)
      .then((next) => {
        if (!alive) return;
        sceneRef.current = next;
        animationsRef.current = animationIndex(next);
        const first = next.skeletons[0];
        playheadRef.current = {
          skeletonIndex: 0,
          bones: newBoneStates(first?.bones.length ?? 0),
          playing: true,
          speed: 1,
          dirty: true,
        };
        setScene(next);
        setPlaying(true);
        setHud({
          frame: 0,
          frames: skeletonLength(first?.bones ?? [], animationsRef.current),
          bones: first?.bones.length ?? 0,
          layerCount: 0,
          time: 0,
          attach: null,
        });
      })
      .catch((e) => {
        if (alive) setError(String(e));
      })
      .finally(() => {
        if (alive) setLoading(false);
      });

    return () => {
      alive = false;
    };
  }, [path, tag]);

  /** 图集纹理：同一 texId 的并发请求合并，返回可等待的任务。 */
  const ensureTextureAsync = useCallback((mbsPath: string, texId: number): Promise<void> => {
    if (texturesRef.current.has(texId)) return Promise.resolve();

    const running = inflightTexRef.current.get(texId);
    if (running) return running;

    const task = api
      .getAtlasPng(mbsPath, texId)
      .then(textureFromPng)
      .then((texture) => {
        texturesRef.current.set(texId, texture);
        playheadRef.current.dirty = true;
      })
      .catch((error: unknown) => {
        // 纹理取不到时画面直接空白；必须把原因告诉用户，不能静默吞掉。
        setTextureError((previous) => previous ?? `纹理 ${texId} 加载失败：${String(error)}`);
      })
      .finally(() => inflightTexRef.current.delete(texId));

    inflightTexRef.current.set(texId, task);
    return task;
  }, []);

  const ensureTexture = useCallback(
    (mbsPath: string, texId: number) => {
      void ensureTextureAsync(mbsPath, texId);
    },
    [ensureTextureAsync],
  );

  /** 进查看器即把场景用到的图集全部取回：拖动时间轴不再突现空白。 */
  useEffect(() => {
    if (!scene) return;
    let cancelled = false;

    // 只预取真正被图层引用到的纹理：图集里的空帧不值得占用显存。
    const referenced = new Set<number>();
    for (const keyframe of scene.keyframe) {
      for (const layer of keyframe?.layers ?? []) {
        if (layer?.texId !== null && layer?.texId !== undefined) referenced.add(layer.texId);
      }
    }
    const ids = referenced.size > 0 ? [...referenced] : scene.atlas.map((frame) => frame.texId);
    void (async () => {
      for (const texId of ids) {
        if (cancelled) return;
        await ensureTextureAsync(scene.mbsPath, texId);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [scene, ensureTextureAsync]);

  /** 某副骨架的骨列表。 */
  const bonesOf = useCallback((index: number): SceneAttach[] => {
    return sceneRef.current?.skeletons[index]?.bones ?? [];
  }, []);

  /** 把当前骨架各骨的帧合成成渲染器输入。 */
  const drawFrame = useCallback(() => {
    const renderer = rendererRef.current;
    const scene = sceneRef.current;
    if (!renderer || !scene) return;

    const playhead = playheadRef.current;
    const animations = animationsRef.current;
    const bones = scene.skeletons[playhead.skeletonIndex]?.bones ?? [];
    const resolved = resolveBones(scene, bones, playhead.bones, animations);

    const renderLayers: RenderLayer[] = [];
    for (const item of resolved.layers) {
      const layer = item.layer;
      const texture = layer.texId === null ? null : texturesRef.current.get(layer.texId);
      if (!texture || layer.src === null) {
        if (layer.texId !== null) ensureTexture(scene.mbsPath, layer.texId);
        continue;
      }
      // 顶点色 = 时间线 color × 每图层 fogquad，再与纹理逐分量相乘。
      const frameTint = optionsRef.current.applyTint ? parseColor(item.color) : NO_TINT;
      const tint: Tint = layer.fog?.[0] ? multiplyTint(frameTint, parseColor(layer.fog[0])) : frameTint;
      renderLayers.push({
        positions: positionsOf(layer.dst, item.matrix),
        uvs: srcUvs(layer.src, texture.width, texture.height),
        texture,
        tint: tint.tint,
        alpha: tint.alpha,
      });
    }

    const hitboxQuads = resolved.hitboxes.map((item) => positionsOf(item.quad, item.matrix));
    const input: RenderFrame = { layers: renderLayers, hitboxes: hitboxQuads };
    renderer.submit(input, optionsRef.current);

    const now = performance.now();
    if (now - hudAtRef.current > 100) {
      hudAtRef.current = now;
      // HUD 以「跑得最远的那根骨」为准；各骨长度不同，取最大值才有意义。
      let frame = 0;
      let frames = 0;
      let time = 0;
      let attach: SceneAttach | null = null;
      for (let index = 0; index < bones.length; index += 1) {
        const animation = boneAnimation(animations, bones[index]);
        if (!animation) continue;
        const at = Math.min(playhead.bones[index]?.frameIndex ?? 0, animation.frames.length - 1);
        if (at >= frame) {
          frame = at;
          time = animation.frames[at]?.time ?? 0;
          attach = animation.frames[at]?.attach ?? null;
        }
        frames = Math.max(frames, animation.frames.length);
      }
      setHud({ frame, frames, bones: bones.length, layerCount: renderLayers.length, time, attach });
    }
  }, [ensureTexture]);

  /** 按首个骨架的前几十帧取景（各骨都算进去，缺骨的模型也能框住）。 */
  const fitScene = useCallback(() => {
    const renderer = rendererRef.current;
    const scene = sceneRef.current;
    const wrap = wrapRef.current;
    if (!renderer || !scene || !wrap) return;

    const bones = scene.skeletons[0]?.bones ?? [];
    const animations = animationsRef.current;
    const probes = newBoneStates(bones.length);
    let bounds: ReturnType<typeof boundsOf> = null;
    for (let frame = 0; frame < 60 && !bounds; frame += 1) {
      for (const probe of probes) probe.frameIndex = frame;
      const resolved = resolveBones(scene, bones, probes, animations);
      bounds = boundsOf(resolved.layers, resolved.hitboxes);
    }
    if (bounds) renderer.setCamera(fitCamera(bounds, wrap.clientWidth, wrap.clientHeight));
    setZoomLabel(renderer.getCamera().zoom);
  }, []);

  /** 画布与渲染器生命周期：场景首次到达后才创建（此前组件走 loading/empty 分支，
   *  wrapRef 还是 null）。之后切换文件复用同一个 WebGL 上下文。 */
  const hasScene = scene !== null;
  useEffect(() => {
    if (!hasScene) return;
    const wrap = wrapRef.current;
    if (!wrap) return;

    let disposed = false;
    let created: PixiRenderer | null = null;
    let detach: () => void = () => undefined;

    const onTick = (deltaMs: number) => {
      const playhead = playheadRef.current;
      const scene = sceneRef.current;
      if (!scene) return;
      const bones = scene.skeletons[playhead.skeletonIndex]?.bones ?? [];
      let changed = playhead.dirty;

      if (playhead.playing && bones.length > 0) {
        let running = 0;
        for (let index = 0; index < bones.length; index += 1) {
          const animation = boneAnimation(animationsRef.current, bones[index]);
          const state = playhead.bones[index];
          if (!animation || !state) continue;
          if (stepBone(animation, state, deltaMs, playhead.speed)) changed = true;
          if (!state.done) running += 1;
        }
        // 所有骨都不再循环（loopId < 0 且播到末帧）才停下。
        if (running === 0) {
          playhead.playing = false;
          setPlaying(false);
        }
      }

      if (!changed) return;
      playhead.dirty = false;
      drawFrame();
    };

    PixiRenderer.create(wrap, wrap.clientWidth, wrap.clientHeight, onTick)
      .then((renderer) => {
        if (disposed) {
          renderer.destroy();
          return;
        }
        created = renderer;
        rendererRef.current = renderer;

        const canvas = renderer.canvas;
        const onWheel = (event: WheelEvent) => {
          event.preventDefault();
          renderer.zoomAt(event.deltaY < 0 ? 1.12 : 1 / 1.12);
          setZoomLabel(renderer.getCamera().zoom);
        };
        let lastX = 0;
        let lastY = 0;
        let dragging = false;
        const onDown = (event: PointerEvent) => {
          dragging = true;
          lastX = event.clientX;
          lastY = event.clientY;
          canvas.dataset.panning = "true";
          canvas.setPointerCapture(event.pointerId);
        };
        const onMove = (event: PointerEvent) => {
          if (!dragging) return;
          renderer.panBy(event.clientX - lastX, event.clientY - lastY);
          lastX = event.clientX;
          lastY = event.clientY;
        };
        const onUp = () => {
          dragging = false;
          canvas.dataset.panning = "false";
        };
        canvas.addEventListener("wheel", onWheel, { passive: false });
        canvas.addEventListener("pointerdown", onDown);
        canvas.addEventListener("pointermove", onMove);
        canvas.addEventListener("pointerup", onUp);
        canvas.addEventListener("pointercancel", onUp);

        const detachListeners = () => {
          canvas.removeEventListener("wheel", onWheel);
          canvas.removeEventListener("pointerdown", onDown);
          canvas.removeEventListener("pointermove", onMove);
          canvas.removeEventListener("pointerup", onUp);
          canvas.removeEventListener("pointercancel", onUp);
        };
        detach = detachListeners;

        if (sceneRef.current) fitScene();
        else setZoomLabel(renderer.getCamera().zoom);

        playheadRef.current.dirty = true;
        drawFrame();
      })
      .catch((e) => setError(String(e)));

    return () => {
      disposed = true;
      rendererRef.current = null;
      detach();
      created?.destroy();
    };
  }, [drawFrame, fitScene, hasScene]);

  /** 画布尺寸变化 → 渲染器 resize + 重画。 */
  useEffect(() => {
    const wrap = wrapRef.current;
    if (!wrap) return;
    const observer = new ResizeObserver(([entry]) => {
      const renderer = rendererRef.current;
      if (!renderer) return;
      renderer.resize(entry.contentRect.width, entry.contentRect.height);
      playheadRef.current.dirty = true;
    });
    observer.observe(wrap);
    return () => observer.disconnect();
  }, []);

  /** 场景切换：重新取景，并释放上一份纹理。 */
  useEffect(() => {
    if (!scene) return;
    fitScene();
    playheadRef.current.dirty = true;

    return () => {
      for (const texture of texturesRef.current.values()) texture.destroy(true);
      texturesRef.current.clear();
      inflightTexRef.current.clear();
    };
  }, [scene, fitScene]);

  function selectSkeleton(index: number) {
    const playhead = playheadRef.current;
    playhead.skeletonIndex = index;
    playhead.bones = newBoneStates(bonesOf(index).length);
    playhead.playing = false;
    playhead.dirty = true;
    setSkeletonIndex(index);
    setPlaying(false);
    setHud((prev) => ({ ...prev, frame: 0, time: 0, attach: null }));
  }

  /** 跳到第 `frame` 帧：各骨按自己的长度夹取（骨长不一致是常态）。 */
  function seek(frame: number) {
    const playhead = playheadRef.current;
    const bones = bonesOf(playhead.skeletonIndex);
    for (let index = 0; index < bones.length; index += 1) {
      const animation = boneAnimation(animationsRef.current, bones[index]);
      const state = playhead.bones[index];
      if (!animation || !state) continue;
      state.frameIndex = Math.max(0, Math.min(frame, animation.frames.length - 1));
      state.elapsed = 0;
      state.done = false;
    }
    playhead.playing = false;
    playhead.dirty = true;
    setPlaying(false);
  }

  function togglePlay() {
    const playhead = playheadRef.current;
    playhead.playing = !playhead.playing;
    // 重新播放时把已结束的骨复位，否则按一下再也不动。
    if (playhead.playing) {
      for (const state of playhead.bones) {
        state.done = false;
        state.elapsed = 0;
      }
    }
    setPlaying(playhead.playing);
  }

  if (!path) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">
        选择一个 .mbs 文件
      </div>
    );
  }

  if (loading && !scene) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">
        解析中…
      </div>
    );
  }

  if (error) {
    return <div className="flex flex-1 items-center justify-center p-4 text-sm text-error">{error}</div>;
  }

  if (!scene)
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">无内容</div>
    );

  const total = hud.frames;
  const currentLayers: SceneLayer[] = resolveBones(
    scene,
    scene.skeletons[skeletonIndex]?.bones ?? [],
    playheadRef.current.bones,
    animationsRef.current,
  ).layers.map((item) => item.layer);

  const loopId = (() => {
    for (const bone of scene.skeletons[skeletonIndex]?.bones ?? []) {
      const animation = boneAnimation(animationsRef.current, bone);
      if (animation) return animation.loopId;
    }
    return -1;
  })();

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex flex-none items-center gap-1.5 border-b border-base-300 bg-base-200 px-2 py-1.5">
        <select
          className="select select-xs"
          value={skeletonIndex}
          onChange={(event) => selectSkeleton(Number(event.target.value))}
          title={`骨架（${scene.skeletons.length}）`}
        >
          {scene.skeletons.map((item, index) => (
            <option key={index} value={index}>
              {item.name || `骨架 ${index}`} · {item.bones.length} 骨
            </option>
          ))}
        </select>

        <button className="btn btn-xs" onClick={togglePlay} disabled={total === 0}>
          {playing ? "暂停" : "播放"}
        </button>
        <button className="btn btn-xs" onClick={() => seek(hud.frame - 1)} title="上一帧">
          ◀
        </button>
        <button
          className="btn btn-xs"
          onClick={() => seek(Math.min(total - 1, hud.frame + 1))}
          title="下一帧"
        >
          ▶
        </button>

        <select
          className="select select-xs"
          value={speed}
          onChange={(event) => setSpeed(Number(event.target.value))}
          title="播放速度"
        >
          {[0.25, 0.5, 1, 2, 4].map((value) => (
            <option key={value} value={value}>
              {value}×
            </option>
          ))}
        </select>

        <div className="flex-1" />
        <span className="font-mono text-[11px] text-base-content/60">
          {hud.frame + 1}/{total} · t{hud.time} · {hud.bones} 骨 · loop {loopId}
        </span>
      </div>

      <div
        className="relative min-h-0 flex-1 cursor-grab overflow-hidden bg-base-200 active:cursor-grabbing"
        ref={wrapRef}
        style={{
          backgroundImage:
            "repeating-conic-gradient(color-mix(in oklab, currentColor 12%, transparent) 0% 25%, transparent 0% 50%)",
          backgroundSize: "16px 16px",
        }}
      >
        <div className="pointer-events-none absolute left-2 top-2 flex gap-1.5">
          <span className="badge badge-sm bg-base-100/90">{Math.round(zoomLabel * 100)}%</span>
          <span className="badge badge-sm bg-base-100/90">{hud.layerCount} layers</span>
          {hud.attach && (
            <span className="badge badge-sm badge-success">
              {hud.attach.kind} {hud.attach.id}
            </span>
          )}
          {textureError && (
            <span className="badge badge-sm badge-error" title={textureError}>
              纹理失败
            </span>
          )}
        </div>
      </div>

      <div className="flex flex-none flex-col gap-1.5 border-t border-base-300 bg-base-200 px-2 py-1.5">
        <div
          className="relative h-6 cursor-pointer border border-base-300 bg-base-100"
          onPointerDown={(event) => {
            const rect = event.currentTarget.getBoundingClientRect();
            const ratio = Math.min(0.999, Math.max(0, (event.clientX - rect.left) / rect.width));
            seek(Math.floor(ratio * total));
          }}
        >
          {total > 0 &&
            total <= 240 &&
            Array.from({ length: total }, (_, index) => (
              <div
                key={index}
                className="absolute inset-y-0 w-px bg-base-300"
                style={{ left: `${(index / total) * 100}%` }}
              />
            ))}
          {loopId >= 0 && total > 0 && (
            <div
              className="absolute inset-y-0 w-px bg-warning"
              style={{ left: `${(loopId / total) * 100}%` }}
              title="循环起点"
            />
          )}
          <div
            className="absolute -top-0.5 -bottom-0.5 w-0.5 bg-primary shadow-md"
            style={{ left: `${total > 0 ? (hud.frame / total) * 100 : 0}%` }}
          />
        </div>
      </div>

      <details className="collapse collapse-arrow shrink-0 border-t border-base-300 bg-base-200">
        <summary className="collapse-title font-mono text-xs text-base-content/60">
          图层 {currentLayers.length} · 调试
        </summary>
        <div className="collapse-content">
          {currentLayers.length === 0 && (
            <div className="font-mono text-xs text-base-content/50">本帧没有贴图图层</div>
          )}
          {currentLayers.map((layer, index) => (
            <LayerRow key={index} index={index} layer={layer} />
          ))}

          <div className="mt-2 border-t border-base-300 pt-1.5">
            <ToggleRow label="命中框" checked={showHitbox} onChange={setShowHitbox} />
            <ToggleRow label="四边形描边" checked={showOutline} onChange={setShowOutline} />
            <ToggleRow label="应用雾色" checked={applyTint} onChange={setApplyTint} />
          </div>
        </div>
      </details>
    </div>
  );
}

/** dstquad 四角经矩阵映射后的 8 个世界坐标（与 `quad_multi4` 同口径）。 */
function positionsOf(quad: number[], matrix: number[]): Float32Array {
  const out = new Float32Array(8);
  const points = quadPoints(quad, matrix);
  for (let index = 0; index < points.length; index += 1) {
    out[index * 2] = points[index][0];
    out[index * 2 + 1] = points[index][1];
  }
  return out;
}

function LayerRow({ index, layer }: { index: number; layer: SceneLayer }) {
  const fog = layer.fog?.[0] ?? null;
  return (
    <div
      className={`flex items-center gap-1.5 border-b border-base-300 px-1 py-0.5 font-mono text-[11px] text-base-content/80 ${
        layer.texId === null ? "opacity-40" : ""
      }`}
    >
      <span className="size-2 shrink-0 border border-base-300" style={{ background: fog ?? "transparent" }} />
      <span className="w-6 shrink-0 text-base-content/50">{index}</span>
      <span className="min-w-0 truncate" title={`${layer.flags} · blend ${layer.blendId}`}>
        tex {layer.texId ?? "-"} · b{layer.blendId} · {layer.flags}
        {layer.colorize ? ` · ${layer.colorize}` : ""}
      </span>
    </div>
  );
}

function ToggleRow({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (next: boolean) => void;
}) {
  return (
    <label className="flex cursor-pointer items-center gap-1.5 py-0.5 font-mono text-[11px]">
      <input
        type="checkbox"
        className="toggle toggle-xs"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
      />
      <span className={checked ? "" : "text-base-content/50"}>{label}</span>
    </label>
  );
}
