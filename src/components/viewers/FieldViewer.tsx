import { t } from "../../i18n";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  api,
  type Content,
  type FieldMap,
  type FieldOverview,
  type FileEntry,
} from "../../lib/ipc";

const MIN_WIDTH = 800;
const MAX_WIDTH = 12000;
const DEFAULT_WIDTH = 1920;
/** Engine clock units in one 60 Hz tick; a second is 60 of them. */
const TICK = 1000;
const CLOCK_MAX = TICK * 60 * 10;

export function FieldViewer({ entry }: { entry: FileEntry }) {
  const isUcb = entry.ext?.toLowerCase() === "ucb";
  const [maps, setMaps] = useState<FieldMap[]>([]);
  const [layer, setLayer] = useState<string | null>(null);
  const [overview, setOverview] = useState<FieldOverview | null>(null);
  const [x0, setX0] = useState<number | null>(null);
  const [width, setWidth] = useState(DEFAULT_WIDTH);
  const [mode, setMode] = useState<"area" | "class">("area");
  const [content, setContent] = useState<Content | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [clock, setClock] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [terrain, setTerrain] = useState(false);

  useEffect(() => {
    let alive = true;
    setMaps([]);
    setLayer(null);
    setX0(null);
    setContent(null);
    setOverview(null);
    setError(null);
    setWidth(DEFAULT_WIDTH);
    setClock(0);
    setPlaying(false);
    if (isUcb) return;
    api
      .fieldMaps(entry.path)
      .then((next) => {
        if (!alive) return;
        setMaps(next);
        const first = next[0];
        if (first) {
          setLayer(first.name);
          setX0(Math.round(first.window[0] + (first.window[2] - first.window[0]) * 0.5));
        }
      })
      .catch((e) => alive && setError(String(e)));
    return () => {
      alive = false;
    };
  }, [entry.path, isUcb]);

  const selected = useMemo(
    () => maps.find((map) => map.name === layer) ?? null,
    [maps, layer],
  );

  useEffect(() => {
    if (isUcb || !layer) return;
    let alive = true;
    api
      .fieldOverview(entry.path, layer)
      .then((next) => alive && setOverview(next))
      .catch(() => alive && setOverview(null));
    return () => {
      alive = false;
    };
  }, [entry.path, isUcb, layer]);

  const span = overview && overview.span[2] > overview.span[0] ? overview.span : null;

  const clamp = useCallback(
    (value: number, total: [number, number, number, number] | null, size: number) => {
      if (!total) return value;
      const low = total[0];
      const high = Math.max(low, total[2] - size);
      return Math.min(high, Math.max(low, value));
    },
    [],
  );

  const request = useRef(0);
  useEffect(() => {
    if (isUcb || !layer) return;
    const id = ++request.current;
    setBusy(true);
    const timer = window.setTimeout(() => {
      api
        .fieldRender(entry.path, layer, x0, width, undefined, clock, terrain)
        .then((next) => {
          if (request.current !== id) return;
          setContent(next);
          setError(null);
        })
        .catch((e) => {
          if (request.current !== id) return;
          setError(String(e));
        })
        .finally(() => {
          if (request.current === id) setBusy(false);
        });
    }, 160);
    return () => window.clearTimeout(timer);
  }, [entry.path, layer, x0, width, clock, terrain, isUcb]);

  useEffect(() => {
    if (!isUcb) return;
    let alive = true;
    setBusy(true);
    api
      .fieldRender(entry.path, null, null, DEFAULT_WIDTH, mode)
      .then((next) => {
        if (!alive) return;
        setContent(next);
        setError(null);
      })
      .catch((e) => alive && setError(String(e)))
      .finally(() => alive && setBusy(false));
    return () => {
      alive = false;
    };
  }, [entry.path, isUcb, mode]);

  useEffect(() => {
    if (!playing || isUcb) return;
    const step = window.setInterval(() => {
      setClock((value) => (value >= CLOCK_MAX ? 0 : value + TICK * 3));
    }, 200);
    return () => window.clearInterval(step);
  }, [playing, isUcb]);

  const groups = useMemo(() => {
    const out: { group: string; entries: FieldMap[] }[] = [];
    for (const map of maps) {
      const bucket = out.find((item) => item.group === map.group);
      if (bucket) bucket.entries.push(map);
      else out.push({ group: map.group, entries: [map] });
    }
    return out;
  }, [maps]);

  const zoom = useCallback(
    (factor: number, anchor: number) => {
      setWidth((current) => {
        const next = Math.round(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, current * factor)));
        if (next === current) return current;
        setX0((left) =>
          left === null ? left : clamp(left + (current - next) * anchor, span, next),
        );
        return next;
      });
    },
    [clamp, span],
  );

  const imageRef = useRef<HTMLImageElement | null>(null);
  const drag = useRef<{ startX: number; startX0: number } | null>(null);

  const onWheel = (event: React.WheelEvent) => {
    if (isUcb || x0 === null) return;
    event.preventDefault();
    const box = imageRef.current?.getBoundingClientRect();
    const anchor = box && box.width > 0 ? (event.clientX - box.left) / box.width : 0.5;
    zoom(event.deltaY > 0 ? 1.25 : 0.8, Math.min(1, Math.max(0, anchor)));
  };

  const onPointerDown = (event: React.PointerEvent) => {
    if (isUcb || x0 === null) return;
    const box = imageRef.current?.getBoundingClientRect();
    if (!box || box.width <= 0) return;
    drag.current = { startX: event.clientX, startX0: x0 };
    (event.target as HTMLElement).setPointerCapture?.(event.pointerId);
  };

  const onPointerMove = (event: React.PointerEvent) => {
    const state = drag.current;
    const box = imageRef.current?.getBoundingClientRect();
    if (!state || !box || box.width <= 0) return;
    const units = ((event.clientX - state.startX) * width) / box.width;
    setX0(clamp(state.startX0 - units, span, width));
  };

  const onPointerUp = () => {
    drag.current = null;
  };

  const jump = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!span || x0 === null) return;
    const box = event.currentTarget.getBoundingClientRect();
    if (box.width <= 0) return;
    const at = (event.clientX - box.left) / box.width;
    const world = span[0] + at * (span[2] - span[0]);
    setX0(clamp(world - width * 0.5, span, width));
  };

  const view = useMemo(() => {
    if (!span || x0 === null) return null;
    const total = span[2] - span[0];
    if (total <= 0) return null;
    return {
      left: ((x0 - span[0]) / total) * 100,
      width: Math.min(100, (width / total) * 100),
    };
  }, [span, x0, width]);

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex flex-none flex-wrap items-center gap-2 border-b border-base-300 bg-base-200 px-2 py-1.5 text-[11px]">
        {isUcb ? (
          <div className="join">
            {(["area", "class"] as const).map((value) => (
              <button
                key={value}
                className={`btn btn-xs join-item ${mode === value ? "btn-primary" : "btn-ghost"}`}
                onClick={() => setMode(value)}
              >
                {t(value === "area" ? "field.area" : "field.class")}
              </button>
            ))}
          </div>
        ) : (
          <>
            <select
              className="select select-xs max-w-[14rem] bg-base-100"
              value={layer ?? ""}
              onChange={(event) => {
                const next = maps.find((map) => map.name === event.target.value);
                setLayer(event.target.value);
                setOverview(null);
                setClock(0);
                setPlaying(false);
                if (next) {
                  setWidth(Math.min(DEFAULT_WIDTH, Math.max(MIN_WIDTH, next.window[2] - next.window[0])));
                  setX0(Math.round(next.window[0] + (next.window[2] - next.window[0]) * 0.5));
                }
              }}
            >
              {groups.map((group) => (
                <optgroup key={group.group} label={group.group}>
                  {group.entries.map((map) => (
                    <option key={map.name} value={map.name}>
                      {map.name} · {map.layers} {t("field.layers")}
                    </option>
                  ))}
                </optgroup>
              ))}
            </select>
            <div className="join">
              <button className="btn btn-xs join-item" onClick={() => zoom(0.8, 0.5)}>
                {t("field.zoomIn")}
              </button>
              <button className="btn btn-xs join-item" onClick={() => zoom(1.25, 0.5)}>
                {t("field.zoomOut")}
              </button>
            </div>
            <span className="font-mono text-base-content/60">
              {Math.round(width)} u · x {Math.round(x0 ?? 0)}
            </span>
            <label className="flex cursor-pointer items-center gap-1 text-base-content/70">
              <input
                type="checkbox"
                className="checkbox checkbox-xs"
                checked={terrain}
                onChange={(event) => setTerrain(event.target.checked)}
              />
              {t("field.terrain")}
            </label>
            <div className="join">
              <button
                className={`btn btn-xs join-item ${playing ? "btn-primary" : "btn-ghost"}`}
                onClick={() => setPlaying((value) => !value)}
              >
                {t(playing ? "field.pause" : "field.play")}
              </button>
              <button
                className="btn btn-xs join-item"
                onClick={() => {
                  setPlaying(false);
                  setClock(0);
                }}
              >
                {t("field.reset")}
              </button>
            </div>
            <input
              className="range range-xs min-w-[6rem] flex-1"
              type="range"
              min={0}
              max={CLOCK_MAX}
              step={TICK}
              value={clock}
              onChange={(event) => {
                setPlaying(false);
                setClock(Number(event.target.value));
              }}
            />
            <span className="font-mono text-base-content/60">
              {(clock / (TICK * 60)).toFixed(1)}s
            </span>
          </>
        )}
        {busy && <span className="text-base-content/50">{t("field.rendering")}</span>}
      </div>

      {!isUcb && overview?.image && span && (
        <div className="flex-none border-b border-base-300 bg-base-300/50 px-2 py-1">
          <div
            className="relative h-8 w-full cursor-crosshair overflow-hidden rounded border border-base-300"
            onPointerDown={jump}
            title={t("field.minimapHint")}
          >
            <img
              src={overview.image}
              alt=""
              className="pointer-events-none h-full w-full object-fill opacity-80"
              draggable={false}
            />
            {view && (
              <div
                className="pointer-events-none absolute inset-y-0 border border-primary bg-primary/20"
                style={{ left: `${view.left}%`, width: `${view.width}%` }}
              />
            )}
          </div>
        </div>
      )}

      {!isUcb && selected && selected.models > 0 && (
        <div
          className="flex-none border-b border-base-300 bg-base-200/60 px-2 py-1 text-[10px] leading-relaxed text-base-content/60"
          title={t("field.propsHint")}
        >
          <span className="text-base-content/70">
            {selected.models} {t("field.models")}
          </span>{" "}
          {selected.props.length > 0 ? (
            <>
              <span className="text-base-content/70">· {t("field.props")}</span>{" "}
              <span className="font-mono">
                {selected.props.map((prop) => `${prop.name}×${prop.count}`).join("  ")}
              </span>
            </>
          ) : (
            <span className="opacity-70">{t("field.propsNone")}</span>
          )}
        </div>
      )}

      <div className="relative min-h-0 flex-1 overflow-auto bg-base-300/40 p-2">
        {error && <div className="p-2 text-xs text-error">{error}</div>}
        {!error && !content && (
          <div className="p-2 font-mono text-xs text-base-content/50">{t("field.rendering")}</div>
        )}
        {content?.image && (
          <img
            ref={imageRef}
            className={`w-full rounded border border-base-300 ${
              isUcb ? "" : "cursor-grab active:cursor-grabbing"
            }`}
            src={content.image}
            alt={content.title}
            draggable={false}
            onWheel={onWheel}
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerUp}
            onPointerCancel={onPointerUp}
          />
        )}
        {content && !content.image && (
          <div className="p-2 text-xs text-base-content/60">{t("field.noImage")}</div>
        )}
      </div>

      {content && (content.imageNote || content.note) && (
        <div className="flex-none border-t border-base-300 bg-base-200 px-2 py-1.5 text-[10px] leading-relaxed text-base-content/60">
          {content.imageNote ?? content.note}
        </div>
      )}
    </div>
  );
}
