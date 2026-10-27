import { useEffect, useState } from "react";
import { getFileThumbUrl, getFileThumbUrlDeferred } from "../../lib/thumbCache";

/** 格式图标。语义名由后端注册表给出，前端只负责画。 */
export function FormatIcon({ icon, size = 16 }: { icon: string; size?: number }) {
  const common = { width: size, height: size, viewBox: "0 0 16 16", fill: "none", stroke: "currentColor" };

  switch (icon) {
    case "texture":
      return (
        <svg {...common}>
          <rect x="2" y="2" width="12" height="12" />
          <path d="M2 10l3.5-3.5L9 10l2-2 3 3" />
          <circle cx="6" cy="5.5" r="1" />
        </svg>
      );
    case "animation":
      return (
        <svg {...common}>
          <rect x="2" y="3" width="12" height="10" />
          <path d="M6.5 6.2 10.5 8l-4 1.8z" />
        </svg>
      );
    case "scene":
      return (
        <svg {...common}>
          <rect x="2" y="2" width="12" height="12" />
          <path d="M2 10h12M4 10V6l3 2 2-3 3 4" />
        </svg>
      );
    case "model":
      return (
        <svg {...common}>
          <path d="M8 2l5 3v6l-5 3-5-3V5z" />
          <path d="M3 5l5 3 5-3M8 8v6" />
        </svg>
      );
    case "script":
      return (
        <svg {...common}>
          <path d="M5 3 2 8l3 5M11 3l3 5-3 5M9 3 7 13" />
        </svg>
      );
    case "text":
      return (
        <svg {...common}>
          <path d="M4 2h5l3 3v9H4z" />
          <path d="M9 2v3h3M6 8h4M6 11h4" />
        </svg>
      );
    case "list":
      return (
        <svg {...common}>
          <path d="M3 4h2M7 4h6M3 8h2M7 8h6M3 12h2M7 12h6" />
        </svg>
      );
    case "grid":
      return (
        <svg {...common}>
          <rect x="2" y="2" width="5" height="5" />
          <rect x="9" y="2" width="5" height="5" />
          <rect x="2" y="9" width="5" height="5" />
          <rect x="9" y="9" width="5" height="5" />
        </svg>
      );
    case "audio":
      return (
        <svg {...common}>
          <path d="M3 6h2l3-3v10L5 10H3z" />
          <path d="M11 5.5a4 4 0 0 1 0 5M13 3.5a7 7 0 0 1 0 9" />
        </svg>
      );
    case "video":
      return (
        <svg {...common}>
          <rect x="2" y="3" width="8" height="10" />
          <path d="M10 6l4-2v8l-4-2z" />
        </svg>
      );
    default:
      return (
        <svg {...common}>
          <path d="M4 2h5l3 3v9H4z" />
          <path d="M9 2v3h3" />
        </svg>
      );
  }
}

/** 缩略图：只有后端声明 `hasThumb` 的条目才会请求，其余直接画格式图标。 */
export function Thumb({
  path,
  icon,
  size = 20,
  hasThumb,
  deferred = false,
}: {
  path: string;
  icon: string;
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
      setFailed(false);
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

  if (url) return <img className="object-contain" style={box} src={url} alt="" />;
  return (
    <span
      className={`flex items-center justify-center ${failed ? "text-error/60" : "text-base-content/50"}`}
      style={box}
    >
      <FormatIcon icon={icon} size={Math.max(12, size - 6)} />
    </span>
  );
}
