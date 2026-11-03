import { t } from "../../i18n";
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

  useEffect(() => {
    playheadRef.current.playing = playing;
    playheadRef.current.speed = speed;
  }, [playing, speed]);

  useEffect(() => {
    optionsRef.current = { showHitbox, showOutline, applyTint };
    playheadRef.current.dirty = true;
  }, [showHitbox, showOutline, applyTint]);

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
        setTextureError((previous) => previous ?? t("scene.textureLoad", { texId, error: String(error) }));
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

  useEffect(() => {
    if (!scene) return;
    let cancelled = false;

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

  const bonesOf = useCallback((index: number): SceneAttach[] => {
    return sceneRef.current?.skeletons[index]?.bones ?? [];
  }, []);

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

  function seek(frame: number) {
    const playhead = playheadRef.current;
    const bones = bonesOf(playhead.skeletonIndex);
    for (let index = 0; index < bones.length; index += 1) {
      const animation = boneAnimation(animationsRef.current, bones[index]);
      const state = playhead.bones[index];
      if (!animation || !state) continue;
      state.frameIndex = Math.max(0, Math.min(frame, animation.frames.length - 1));
      state.remaining = -1;
      state.done = false;
    }
    playhead.playing = false;
    playhead.dirty = true;
    setPlaying(false);
  }

  function togglePlay() {
    const playhead = playheadRef.current;
    playhead.playing = !playhead.playing;
    if (playhead.playing) {
      for (const state of playhead.bones) {
        state.done = false;
        state.remaining = -1;
      }
    }
    setPlaying(playhead.playing);
  }

  if (!path) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">
        {t("scene.pickMbs")}
      </div>
    );
  }

  if (loading && !scene) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">
        {t("scene.parsing")}
      </div>
    );
  }

  if (error) {
    return <div className="flex flex-1 items-center justify-center p-4 text-sm text-error">{error}</div>;
  }

  if (!scene)
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">{t("common.empty")}</div>
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
          title={t("scene.skeletonGroup", { count: scene.skeletons.length })}
        >
          {scene.skeletons.map((item, index) => (
            <option key={index} value={index}>
              {item.name || t("scene.skeletonName", { index })} · {t("scene.bones", { count: item.bones.length })}
            </option>
          ))}
        </select>

        <button className="btn btn-xs" onClick={togglePlay} disabled={total === 0}>
          {playing ? t("common.pause") : t("common.play")}
        </button>
        <button className="btn btn-xs" onClick={() => seek(hud.frame - 1)} title={t("video.prevFrame")}>
          ◀
        </button>
        <button
          className="btn btn-xs"
          onClick={() => seek(Math.min(total - 1, hud.frame + 1))}
          title={t("video.nextFrame")}
        >
          ▶
        </button>

        <select
          className="select select-xs"
          value={speed}
          onChange={(event) => setSpeed(Number(event.target.value))}
          title={t("scene.speed")}
        >
          {[0.25, 0.5, 1, 2, 4].map((value) => (
            <option key={value} value={value}>
              {value}×
            </option>
          ))}
        </select>

        <div className="flex-1" />
        <span className="font-mono text-[11px] text-base-content/60">
          {t("scene.hud", { frame: hud.frame + 1, total, time: hud.time, bones: hud.bones, loop: loopId })}
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
              {t("scene.textureFailed")}
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
              title={t("scene.loopStart")}
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
          {t("scene.layers", { count: currentLayers.length })}
        </summary>
        <div className="collapse-content">
          {currentLayers.length === 0 && (
            <div className="font-mono text-xs text-base-content/50">{t("scene.noLayers")}</div>
          )}
          {currentLayers.map((layer, index) => (
            <LayerRow key={index} index={index} layer={layer} />
          ))}

          <div className="mt-2 border-t border-base-300 pt-1.5">
            <ToggleRow label={t("scene.hitbox")} checked={showHitbox} onChange={setShowHitbox} />
            <ToggleRow label={t("scene.outline")} checked={showOutline} onChange={setShowOutline} />
            <ToggleRow label={t("scene.tint")} checked={applyTint} onChange={setApplyTint} />
          </div>
        </div>
      </details>
    </div>
  );
}

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
