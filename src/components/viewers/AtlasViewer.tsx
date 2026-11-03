import { useEffect, useState } from "react";

import { t } from "../../i18n";
import { api, type FileEntry, type FrameList } from "../../lib/ipc";
import { getFrameUrl } from "../../lib/thumbCache";
import { ImageViewport } from "../ImageViewport";

interface Probe {
  transparent: boolean;
  uniformRgb: string | null;
}

function inspect(image: HTMLImageElement): Probe {
  const edge = 16;
  const canvas = document.createElement("canvas");
  canvas.width = edge;
  canvas.height = edge;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) return { transparent: false, uniformRgb: null };

  context.drawImage(image, 0, 0, edge, edge);
  const data = context.getImageData(0, 0, edge, edge).data;

  let opaque = false;
  let uniform = true;
  let red = data[0];
  let green = data[1];
  let blue = data[2];

  for (let at = 0; at < data.length; at += 4) {
    if (data[at + 3] > 0) opaque = true;
    if (data[at] !== red || data[at + 1] !== green || data[at + 2] !== blue) uniform = false;
  }

  const hex = (value: number) => value.toString(16).padStart(2, "0");
  return {
    transparent: !opaque,
    uniformRgb: uniform ? `#${hex(red)}${hex(green)}${hex(blue)}` : null,
  };
}

export function AtlasViewer({ entry }: { entry: FileEntry }) {
  const [frames, setFrames] = useState<FrameList | null>(null);
  const [index, setIndex] = useState(0);
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [probe, setProbe] = useState<Probe | null>(null);

  useEffect(() => {
    let alive = true;
    setFrames(null);
    setIndex(0);
    setError(null);

    api
      .listFrames(entry.path)
      .then((list) => {
        if (alive) setFrames(list);
      })
      .catch((e) => {
        if (alive) setError(String(e));
      });

    return () => {
      alive = false;
    };
  }, [entry.path]);

  useEffect(() => {
    let alive = true;
    setUrl(null);
    setProbe(null);

    getFrameUrl(entry.path, index)
      .then((next) => {
        if (alive) setUrl(next);
      })
      .catch((e) => {
        if (alive) setError(String(e));
      });

    return () => {
      alive = false;
    };
  }, [entry.path, index]);

  useEffect(() => {
    if (!url) return;
    let alive = true;
    const image = new Image();
    image.onload = () => {
      if (alive) setProbe(inspect(image));
    };
    image.src = url;
    return () => {
      alive = false;
    };
  }, [url]);

  if (error) {
    return <div className="flex flex-1 items-center justify-center p-4 text-sm text-error">{error}</div>;
  }

  const frame = frames?.frames[index] ?? null;

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      <div className="w-36 shrink-0 overflow-auto border-r border-base-300 bg-base-200">
        <div className="p-2 text-[10px] uppercase tracking-wider text-base-content/60">
          {t("atlas.frames", { count: frames?.count ?? "…" })}
        </div>
        <ul className="menu menu-xs w-full p-0">
          {frames?.frames.map((item) => (
            <li key={item.index}>
              <button className={item.index === index ? "menu-active" : ""} onClick={() => setIndex(item.index)}>
                <span>#{item.index}</span>
                <span className="font-mono text-base-content/50">
                  {item.width}×{item.height}
                </span>
              </button>
            </li>
          ))}
        </ul>
      </div>

      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-1.5 overflow-hidden p-2">
        {frame && (
          <div className="flex flex-wrap items-center gap-1.5 font-mono text-[10px] text-base-content/50">
            <span>
              {frame.width}×{frame.height} · {frame.formatName} · {frame.channels}ch
            </span>
            {probe?.transparent && (
              <span
                className="badge badge-xs badge-warning font-sans"
                title={t("atlas.transparentHint")}
              >
                {t("atlas.transparent")}
              </span>
            )}
            {probe?.transparent && probe.uniformRgb && (
              <span className="font-mono">RGB {probe.uniformRgb}</span>
            )}
          </div>
        )}
        {url ? (
          <ImageViewport
            key={`${entry.path}#${index}`}
            className="min-h-0 flex-1 rounded border border-base-300"
            src={url}
            alt={entry.name}
          />
        ) : (
          <span className="p-2 text-base-content/50">{t("atlas.decoding")}</span>
        )}
      </div>
    </div>
  );
}
