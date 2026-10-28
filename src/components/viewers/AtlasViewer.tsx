import { useEffect, useState } from "react";
import { api, type FileEntry, type FrameList } from "../../lib/ipc";
import { getFrameUrl } from "../../lib/thumbCache";

export function AtlasViewer({ entry }: { entry: FileEntry }) {
  const [frames, setFrames] = useState<FrameList | null>(null);
  const [index, setIndex] = useState(0);
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

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

  if (error) {
    return <div className="flex flex-1 items-center justify-center p-4 text-sm text-error">{error}</div>;
  }

  const frame = frames?.frames[index] ?? null;

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      <div className="w-36 shrink-0 overflow-auto border-r border-base-300 bg-base-200">
        <div className="p-2 text-[10px] uppercase tracking-wider text-base-content/60">
          帧（{frames?.count ?? "…"}）
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

      <div className="flex min-w-0 flex-1 flex-col gap-1.5 overflow-auto p-2">
        {frame && (
          <div className="font-mono text-[10px] text-base-content/50">
            {frame.width}×{frame.height} · {frame.formatName} · {frame.channels}ch
          </div>
        )}
        {url ? (
          <img className="max-w-full object-contain" src={url} alt={entry.name} />
        ) : (
          <span className="text-base-content/50">解码中…</span>
        )}
      </div>
    </div>
  );
}
