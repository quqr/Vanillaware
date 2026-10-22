import { useEffect, useState } from "react";
import { api, type FileEntry, type FrameList } from "../../lib/ipc";
import { getFrameUrl } from "../../lib/thumbCache";

/** 纹理查看器：FTEX 容器内的帧列表 + 单帧预览。 */
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
    return <div className="pending-hint" style={{ color: "var(--error)" }}>{error}</div>;
  }

  const frame = frames?.frames[index] ?? null;

  return (
    <div className="atlas-viewer">
      <div className="atlas-frames">
        <div className="section-title">帧（{frames?.count ?? "…"}）</div>
        {frames?.frames.map((item) => (
          <button
            key={item.index}
            className="tree-item"
            data-active={item.index === index}
            onClick={() => setIndex(item.index)}
          >
            <span>#{item.index}</span>
            <span className="mono dim">
              {item.width}×{item.height}
            </span>
          </button>
        ))}
      </div>

      <div className="atlas-stage">
        {frame && (
          <div className="atlas-meta mono dim">
            {frame.width}×{frame.height} · {frame.formatName} · {frame.channels}ch
          </div>
        )}
        {url ? <img src={url} alt={entry.name} /> : <span className="dim">解码中…</span>}
      </div>
    </div>
  );
}
