import { t } from "../../i18n";
import { useEffect, useMemo, useRef, useState } from "react";
import { api, type Content, type FieldMap, type FileEntry } from "../../lib/ipc";
import { ImageViewport } from "../ImageViewport";

const WINDOW_WIDTH = 1920;

export function FieldViewer({ entry }: { entry: FileEntry }) {
  const isUcb = entry.ext?.toLowerCase() === "ucb";
  const [maps, setMaps] = useState<FieldMap[]>([]);
  const [layer, setLayer] = useState<string | null>(null);
  const [x0, setX0] = useState<number | null>(null);
  const [mode, setMode] = useState<"area" | "class">("area");
  const [content, setContent] = useState<Content | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let alive = true;
    setMaps([]);
    setLayer(null);
    setX0(null);
    setContent(null);
    setError(null);
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

  const range = useMemo(() => {
    if (!selected) return null;
    const low = selected.window[0];
    const high = selected.window[2] - WINDOW_WIDTH;
    if (high <= low) return null;
    return { low, high };
  }, [selected]);

  const request = useRef(0);
  useEffect(() => {
    if (isUcb) return;
    if (!layer) return;
    const id = ++request.current;
    setBusy(true);
    const timer = window.setTimeout(() => {
      api
        .fieldRender(entry.path, layer, x0, WINDOW_WIDTH)
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
  }, [entry.path, layer, x0, isUcb]);

  useEffect(() => {
    if (!isUcb) return;
    let alive = true;
    setBusy(true);
    api
      .fieldRender(entry.path, null, null, WINDOW_WIDTH, mode)
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

  const groups = useMemo(() => {
    const out: { group: string; entries: FieldMap[] }[] = [];
    for (const map of maps) {
      const bucket = out.find((item) => item.group === map.group);
      if (bucket) bucket.entries.push(map);
      else out.push({ group: map.group, entries: [map] });
    }
    return out;
  }, [maps]);

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
              className="select select-xs max-w-[16rem] bg-base-100"
              value={layer ?? ""}
              onChange={(event) => {
                const next = maps.find((map) => map.name === event.target.value);
                setLayer(event.target.value);
                if (next) {
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
            {range && (
              <input
                className="range range-xs min-w-[8rem] flex-1"
                type="range"
                min={Math.round(range.low)}
                max={Math.round(range.high)}
                step={32}
                value={Math.round(x0 ?? range.low)}
                onChange={(event) => setX0(Number(event.target.value))}
              />
            )}
            {x0 !== null && (
              <span className="font-mono text-base-content/60">x {Math.round(x0)}</span>
            )}
          </>
        )}
        {busy && <span className="text-base-content/50">{t("field.rendering")}</span>}
      </div>

      <div className="relative min-h-0 flex-1 overflow-hidden bg-base-300/40 p-2">
        {error && <div className="p-2 text-xs text-error">{error}</div>}
        {!error && !content && (
          <div className="p-2 font-mono text-xs text-base-content/50">{t("field.rendering")}</div>
        )}
        {content?.image && (
          <ImageViewport
            className="h-full w-full rounded border border-base-300"
            src={content.image}
            alt={content.title}
            checker={false}
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
