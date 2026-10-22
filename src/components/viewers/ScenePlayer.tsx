import { useCallback, useEffect, useRef, useState } from "react";
import { Texture } from "pixi.js";
import { api, type Scene, type SceneLayer } from "../../lib/ipc";
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
import { fitCamera, frameBounds, frameDurationMs, resolveFrame, srcUvs } from "../../lib/sceneDraw";

/** 播放位置：全部放在 ref 里，由 Pixi ticker 推进，避免 60Hz 触发 React 重渲染。 */
interface Playhead {
  animationIndex: number;
  frameIndex: number;
  elapsed: number;
  playing: boolean;
  speed: number;
  dirty: boolean;
}

interface Hud {
  frameIndex: number;
  frameCount: number;
  time: number;
  loopId: number;
  layerCount: number;
}

/** 动画播放查看器：动画列表 + PixiJS 合成 + 图层面板 + 时间轴（ADR-0007 / ADR-0011）。
 *
 * 受控组件：目标 `.mbs` 与平台 `tag` 都由查看器面板传入，不再读全局选中态。
 */
export function ScenePlayer({ path, tag }: { path: string; tag: string }) {

  const [scene, setScene] = useState<Scene | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const [animationIndex, setAnimationIndex] = useState(0);
  const [playing, setPlaying] = useState(true);
  const [speed, setSpeed] = useState(1);
  const [showHitbox, setShowHitbox] = useState(false);
  const [showOutline, setShowOutline] = useState(false);
  const [applyTint, setApplyTint] = useState(true);
  const [hud, setHud] = useState<Hud>({ frameIndex: 0, frameCount: 0, time: 0, loopId: -1, layerCount: 0 });
  const [zoomLabel, setZoomLabel] = useState(1);

  const wrapRef = useRef<HTMLDivElement>(null);
  const rendererRef = useRef<PixiRenderer | null>(null);
  const sceneRef = useRef<Scene | null>(null);
  const texturesRef = useRef(new Map<number, Texture>());
  const inflightTexRef = useRef(new Map<number, Promise<void>>());
  const playheadRef = useRef<Playhead>({ animationIndex: 0, frameIndex: 0, elapsed: 0, playing: true, speed: 1, dirty: true });
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
    setAnimationIndex(0);

    api
      .previewScene(path, tag)
      .then((next) => {
        if (!alive) return;
        sceneRef.current = next;
        playheadRef.current = { animationIndex: 0, frameIndex: 0, elapsed: 0, playing: true, speed: 1, dirty: true };
        setScene(next);
        setPlaying(true);
        setHud({ frameIndex: 0, frameCount: next.animations[0]?.frames.length ?? 0, time: 0, loopId: next.animations[0]?.loopId ?? -1, layerCount: 0 });
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
      .catch(() => undefined)
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

  /** 把当前帧的图层与命中框组装成渲染器输入。 */
  const drawFrame = useCallback(() => {
    const renderer = rendererRef.current;
    const scene = sceneRef.current;
    if (!renderer || !scene) return;

    const playhead = playheadRef.current;
    const animation = scene.animations[playhead.animationIndex];
    if (!animation) {
      renderer.submit({ layers: [], hitboxes: [] }, { showHitbox: false, showOutline: false });
      return;
    }

    const { frame, layers, hitboxes } = resolveFrame(scene, animation, playhead.frameIndex);
    if (!frame) {
      renderer.submit({ layers: [], hitboxes: [] }, optionsRef.current);
      return;
    }

    // 权威播放器：顶点色 = 时间线 color × 每图层 fogquad，再与纹理逐分量相乘。
    const frameTint = optionsRef.current.applyTint ? parseColor(frame.color) : NO_TINT;
    const renderLayers: RenderLayer[] = [];

    for (const layer of layers) {
      const texture = layer.texId === null ? null : texturesRef.current.get(layer.texId);
      if (!texture || layer.src === null) {
        if (layer.texId !== null) ensureTexture(scene.mbsPath, layer.texId);
        continue;
      }
      const tint: Tint = layer.fog?.[0] ? multiplyTint(frameTint, parseColor(layer.fog[0])) : frameTint;
      renderLayers.push({
        positions: positions(layer, frame.matrix),
        uvs: srcUvs(layer.src, texture.width, texture.height),
        texture,
        tint: tint.tint,
        alpha: tint.alpha,
      });
    }

    const hitboxQuads = hitboxes.map((quad) => positionsOf(quad, frame.matrix));
    const input: RenderFrame = { layers: renderLayers, hitboxes: hitboxQuads };
    renderer.submit(input, optionsRef.current);

    const now = performance.now();
    if (now - hudAtRef.current > 100) {
      hudAtRef.current = now;
      setHud({
        frameIndex: playhead.frameIndex,
        frameCount: animation.frames.length,
        time: frame.time,
        loopId: animation.loopId,
        layerCount: renderLayers.length,
      });
    }
  }, [ensureTexture]);

  /** 按首个有图层的帧取景。 */
  const fitScene = useCallback(() => {
    const renderer = rendererRef.current;
    const scene = sceneRef.current;
    const wrap = wrapRef.current;
    if (!renderer || !scene || !wrap) return;

    const animation = scene.animations[0];
    let bounds: ReturnType<typeof frameBounds> = null;
    for (let index = 0; animation && index < animation.frames.length && index < 60 && !bounds; index += 1) {
      bounds = frameBounds(scene, animation, index);
    }
    if (bounds) renderer.setCamera(fitCamera(bounds, wrap.clientWidth, wrap.clientHeight));
    setZoomLabel(renderer.getCamera().zoom);
  }, []);

  /** 画布与渲染器生命周期：只在挂载时创建一次，切换文件不重建 WebGL 上下文。 */
  useEffect(() => {
    const wrap = wrapRef.current;
    if (!wrap) return;

    let disposed = false;
    let created: PixiRenderer | null = null;
    let detach: () => void = () => undefined;

    const onTick = (deltaMs: number) => {
      const playhead = playheadRef.current;
      const animation = sceneRef.current?.animations[playhead.animationIndex];
      if (!animation) return;

      let changed = playhead.dirty;

      if (playhead.playing && animation.frames.length > 0) {
        playhead.elapsed += deltaMs;
        for (let guard = 0; guard < 512; guard += 1) {
          const frame = animation.frames[playhead.frameIndex];
          if (!frame) break;
          const span = frameDurationMs(frame, playhead.speed);
          if (playhead.elapsed < span) break;

          playhead.elapsed -= span;
          let next = playhead.frameIndex + 1;
          if (next >= animation.frames.length) {
            if (animation.loopId >= 0 && animation.loopId < animation.frames.length) {
              next = animation.loopId;
            } else {
              next = animation.frames.length - 1;
              playhead.playing = false;
              setPlaying(false);
            }
          }
          playhead.frameIndex = next;
          changed = true;
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
  }, [drawFrame, fitScene]);

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

  function selectAnimation(index: number) {
    const playhead = playheadRef.current;
    playhead.animationIndex = index;
    playhead.frameIndex = 0;
    playhead.elapsed = 0;
    playhead.playing = false;
    playhead.dirty = true;
    setAnimationIndex(index);
    setPlaying(false);
    setHud((prev) => ({ ...prev, frameIndex: 0, time: 0 }));
  }

  function seek(index: number) {
    const playhead = playheadRef.current;
    playhead.frameIndex = Math.max(0, index);
    playhead.elapsed = 0;
    playhead.playing = false;
    playhead.dirty = true;
    setPlaying(false);
  }

  function togglePlay() {
    const playhead = playheadRef.current;
    playhead.playing = !playhead.playing;
    playhead.elapsed = 0;
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

  const animation = scene.animations[animationIndex] ?? null;
  const total = animation?.frames.length ?? 0;
  const currentFrame = animation?.frames[hud.frameIndex] ?? null;
  const loopId = animation?.loopId ?? -1;
  const keyframeId = currentFrame?.attach?.id ?? -1;
  const layers = scene.keyframe[keyframeId]?.layers.filter((layer): layer is SceneLayer => layer !== null) ?? [];

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex flex-none items-center gap-1.5 border-b border-base-300 bg-base-200 px-2 py-1.5">
        <select
          className="select select-xs"
          value={animationIndex}
          onChange={(event) => selectAnimation(Number(event.target.value))}
          title={`动画（${scene.animations.length}）`}
        >
          {scene.animations.map((item, index) => (
            <option key={item.id} value={index}>
              {item.name || `动画 ${item.id}`} · {item.frames.length} 帧
            </option>
          ))}
        </select>

        <button className="btn btn-xs" onClick={togglePlay} disabled={total === 0}>
          {playing ? "暂停" : "播放"}
        </button>
        <button className="btn btn-xs" onClick={() => seek(hud.frameIndex - 1)} title="上一帧">
          ◀
        </button>
        <button
          className="btn btn-xs"
          onClick={() => seek(Math.min(total - 1, hud.frameIndex + 1))}
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
          {hud.frameIndex + 1}/{total} · t{hud.time} · loop {loopId}
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
          {currentFrame?.attach && (
            <span className="badge badge-sm badge-success">{currentFrame.attach.kind}</span>
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
            animation?.frames.map((_, index) => (
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
            style={{ left: `${total > 0 ? (hud.frameIndex / total) * 100 : 0}%` }}
          />
        </div>
      </div>

      <details className="collapse collapse-arrow shrink-0 border-t border-base-300 bg-base-200">
        <summary className="collapse-title font-mono text-xs text-base-content/60">
          图层 {layers.length} · 调试
        </summary>
        <div className="collapse-content">
          {layers.length === 0 && (
            <div className="font-mono text-xs text-base-content/50">本帧没有贴图图层</div>
          )}
          {layers.map((layer, index) => (
            <LayerRow key={index} index={index} layer={layer} color={currentFrame?.color ?? null} />
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

/** dstquad 四角经矩阵映射后的 8 个世界坐标。 */
function positions(layer: SceneLayer, matrix: number[]): Float32Array {
  return positionsOf(layer.dst, matrix);
}

function positionsOf(quad: number[], matrix: number[]): Float32Array {
  const out = new Float32Array(8);
  for (let i = 0; i < 4; i += 1) {
    const x = quad[i * 2] ?? 0;
    const y = quad[i * 2 + 1] ?? 0;
    if (matrix.length < 16) {
      out[i * 2] = x;
      out[i * 2 + 1] = y;
    } else {
      out[i * 2] = matrix[0] * x + matrix[1] * y + matrix[3];
      out[i * 2 + 1] = matrix[4] * x + matrix[5] * y + matrix[7];
    }
  }
  return out;
}

function LayerRow({ index, layer, color }: { index: number; layer: SceneLayer; color: string | null }) {
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
        {color ? ` · ${color}` : ""}
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
