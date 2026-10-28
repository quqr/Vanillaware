import { useEffect, useState } from "react";
import { api, type BbsLayout, type FileEntry } from "../../lib/ipc";

const W = 800;
const H = 600;

export function BbsLayoutViewer({ entry }: { entry: FileEntry }) {
  const [data, setData] = useState<BbsLayout | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    setData(null);
    setError(null);
    api
      .docLayout(entry.path)
      .then((next) => {
        if (alive) setData(next);
      })
      .catch((e) => {
        if (alive) setError(String(e));
      });
    return () => {
      alive = false;
    };
  }, [entry.path]);

  if (error) {
    return <div className="flex flex-1 items-center justify-center p-4 text-sm text-error">{error}</div>;
  }
  if (!data) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">
        读取布局…
      </div>
    );
  }

  const [rawX0, rawY0, rawX1, rawY1] = data.bbox;
  const xs = [rawX0, rawX1, ...data.objects.map((object) => object.pos[0])];
  const ys = [rawY0, rawY1, ...data.objects.map((object) => object.pos[1])];
  const x0 = Math.min(...xs);
  const x1 = Math.max(...xs);
  const y0 = Math.min(...ys);
  const y1 = Math.max(...ys);
  const spanX = Math.max(1, x1 - x0);
  const spanY = Math.max(1, y1 - y0);
  const scale = Math.min(W / spanX, H / spanY) * 0.9;
  const offsetX = (W - spanX * scale) / 2;
  const offsetY = (H - spanY * scale) / 2;

  const px = (x: number) => offsetX + (x - x0) * scale;
  const py = (y: number) => offsetY + (y1 - y) * scale;

  const routes = new Map<number, typeof data.routes>();
  for (const route of data.routes) {
    const list = routes.get(route.slot) ?? [];
    list.push(route);
    routes.set(route.slot, list);
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex flex-none flex-wrap items-center gap-1.5 border-b border-base-300 bg-base-200 p-1.5 font-mono text-[10px]">
        <span className="badge badge-xs badge-primary">bbox</span>
        <span className="text-base-content/60">
          {rawX0.toFixed(1)}, {rawY0.toFixed(1)} → {rawX1.toFixed(1)}, {rawY1.toFixed(1)}
        </span>
        <span className="badge badge-xs">obj {data.objects.length}</span>
        <span className="badge badge-xs">fog {data.zones.length}</span>
        <span className="badge badge-xs">route {data.routes.length}</span>
        <span className="badge badge-xs">mbs {data.mbsNames.length}</span>
        <span className="badge badge-xs">ftx {data.ftxNames.length}</span>
      </div>

      <div className="min-h-0 flex-1 overflow-hidden bg-base-100">
        <svg viewBox={`0 0 ${W} ${H}`} className="h-full w-full" role="img" aria-label="BBS layout">
          <rect width={W} height={H} className="fill-base-100" />

          {data.zones.map((zone) => (
            <polygon
              key={`z${zone.id}`}
              points={zone.quad.map(([x, y]) => `${px(x)},${py(y)}`).join(" ")}
              fill="currentColor"
              fillOpacity={0.08}
              stroke="currentColor"
              strokeOpacity={0.25}
            />
          ))}

          <rect
            x={px(rawX0)}
            y={py(rawY1)}
            width={Math.max(0, (rawX1 - rawX0) * scale)}
            height={Math.max(0, (rawY1 - rawY0) * scale)}
            fill="none"
            stroke="currentColor"
            strokeOpacity={0.45}
            strokeDasharray="4 3"
          />

          {Array.from(routes.entries()).map(([slot, list]) => {
            const points = [...list]
              .sort((left, right) => left.local - right.local)
              .map((route) => `${px(route.pos[0])},${py(route.pos[1])}`)
              .join(" ");
            return (
              <polyline
                key={`route${slot}`}
                points={points}
                fill="none"
                stroke="hsl(175 70% 55%)"
                strokeWidth={1}
                strokeOpacity={0.8}
              />
            );
          })}

          {data.objects.map((object) => (
            <rect
              key={`o${object.id}`}
              x={px(object.pos[0]) - 2}
              y={py(object.pos[1]) - 2}
              width={4}
              height={4}
              fill={`hsl(${(object.ftx * 47) % 360} 70% 58%)`}
              fillOpacity={0.9}
            />
          ))}
        </svg>
      </div>
    </div>
  );
}
