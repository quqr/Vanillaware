import { useEffect, useState } from "react";
import type { Kind } from "../../lib/ipc";
import { getFileThumbUrl, getFileThumbUrlDeferred } from "../../lib/thumbCache";

/** 类别图标。类别由后端注册表给出，前端只管画。 */
export function KindIcon({ kind, size = 16 }: { kind: Kind; size?: number }) {
  const common = { width: size, height: size, viewBox: "0 0 16 16", fill: "none", stroke: "currentColor" };
  if (kind === "texture") {
    return (
      <svg {...common}>
        <rect x="2" y="2" width="12" height="12" />
        <path d="M2 10l3.5-3.5L9 10l2-2 3 3" />
        <circle cx="6" cy="5.5" r="1" />
      </svg>
    );
  }
  if (kind === "animation") {
    return (
      <svg {...common}>
        <rect x="2" y="3" width="12" height="10" />
        <path d="M6.5 6.2 10.5 8l-4 1.8z" />
      </svg>
    );
  }
  return (
    <svg {...common}>
      <path d="M4 2h5l3 3v9H4z" />
      <path d="M9 2v3h3" />
    </svg>
  );
}

/** 缩略图：只有后端声明 `hasThumb` 的条目才会请求，其余直接画图标。 */
export function Thumb({
  path,
  kind,
  size = 20,
  hasThumb,
  deferred = false,
}: {
  path: string;
  kind: Kind;
  size?: number;
  hasThumb: boolean;
  /** 列表里用：滚动停止后才解码。 */
  deferred?: boolean;
}) {
  const [url, setUrl] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (!hasThumb) {
      setUrl(null);
      return;
    }

    let alive = true;
    setUrl(null);
    setFailed(false);

    (deferred ? getFileThumbUrlDeferred(path) : getFileThumbUrl(path))
      .then((next) => {
        if (alive) setUrl(next);
      })
      .catch(() => {
        if (alive) setFailed(true);
      });

    return () => {
      alive = false;
    };
  }, [path, hasThumb, deferred]);

  const box = { width: size, height: size };

  if (url) return <img className="thumb" style={box} src={url} alt="" />;
  return (
    <span className="icon-slot" style={box} data-failed={failed}>
      <KindIcon kind={kind} size={Math.max(12, size - 6)} />
    </span>
  );
}
