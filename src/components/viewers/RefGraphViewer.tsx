import { useEffect, useMemo, useState } from "react";
import { api, type FileEntry, type GraphNode, type FileGraph } from "../../lib/ipc";
import { useLibraryStore } from "../../store/libraryStore";

const WIDTH = 760;
const ROW = 30;

/** 外部引用边标签 → 中文。 */
const EXTERNAL_LABEL: Record<string, string> = {
  text: "文本表",
  loads: "资源",
};

/** 跨文件引用图：RSC 条目/载荷、OTM 清单、BBS 名表共用（ADR-0019）。 */
export function RefGraphViewer({ entry }: { entry: FileEntry }) {
  const [data, setData] = useState<FileGraph | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<GraphNode | null>(null);
  const select = useLibraryStore((s) => s.select);

  useEffect(() => {
    let alive = true;
    setData(null);
    setError(null);
    setSelected(null);
    api
      .docGraph(entry.path)
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

  // 外部引用（配对 .fms 与载荷内联资源串）：按边标签分组并统计引用次数。
  const externals = useMemo(() => {
    if (!data) return [];
    const byNode = new Map<number, { label: string; count: number }>();
    for (const edge of data.edges) {
      const target = data.nodes.find((node) => node.id === edge.to);
      if (!target || target.kind !== "external") continue;
      const current = byNode.get(edge.to);
      byNode.set(edge.to, { label: edge.label, count: (current?.count ?? 0) + 1 });
    }
    return data.nodes
      .filter((node) => node.kind === "external")
      .map((node) => ({ node, ...(byNode.get(node.id) ?? { label: "", count: 1 }) }));
  }, [data]);

  if (error) {
    return <div className="flex flex-1 items-center justify-center p-4 text-sm text-error">{error}</div>;
  }
  if (!data) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">
        读取引用图…
      </div>
    );
  }

  const entries = data.nodes.filter((node) => node.kind === "entry");
  const scripts = data.nodes.filter((node) => node.kind === "script").length;
  const height = Math.max(340, entries.length * ROW + 80);
  const entryY = new Map<number, number>();
  entries.forEach((node, index) => entryY.set(node.id, 40 + index * ROW));
  const payloadY = new Map<number, number>();
  for (const edge of data.edges) {
    if (edge.label === "payload" && entryY.has(edge.from)) {
      payloadY.set(edge.to, entryY.get(edge.from) ?? 40);
    }
  }

  const pos = (node: GraphNode): { x: number; y: number } | null => {
    if (node.kind === "file") return { x: 30, y: height / 2 };
    if (node.kind === "entry") {
      const y = entryY.get(node.id);
      return y === undefined ? null : { x: 260, y };
    }
    if (node.kind === "script") {
      const y = payloadY.get(node.id);
      return y === undefined ? null : { x: 540, y };
    }
    return null;
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex flex-none flex-wrap items-center gap-1.5 border-b border-base-300 bg-base-200 p-1.5 font-mono text-[10px]">
        <span className="badge badge-xs badge-primary">RSC</span>
        <span className="badge badge-xs">entry {entries.length}</span>
        <span className="badge badge-xs">script {scripts}</span>
        <span className="badge badge-xs">edge {data.edges.length}</span>
        {externals.length > 0 && <span className="badge badge-xs">ref {externals.length}</span>}
        {data.truncated && <span className="badge badge-xs badge-warning">已截断</span>}
      </div>

      {externals.length > 0 && (
        <div className="flex flex-none flex-wrap items-center gap-1 border-b border-base-300 bg-base-100 px-1.5 py-1 font-mono text-[10px]">
          <span className="text-base-content/50">跨文件</span>
          {externals.map(({ node, label, count }) => {
            const resolved = node.status === "resolved";
            return (
              <button
                key={node.id}
                className={`btn btn-xs ${resolved ? "btn-outline" : "btn-disabled"}`}
                title={resolved ? node.path ?? node.label : `缺失：${node.path ?? node.label}`}
                disabled={!resolved}
                onClick={() => node.path && select(node.path)}
              >
                <span className="text-base-content/50">{EXTERNAL_LABEL[label] ?? label}</span>
                <span className="max-w-48 truncate">{node.label}</span>
                {count > 1 && <span className="opacity-60">×{count}</span>}
              </button>
            );
          })}
        </div>
      )}

      {selected && (
        <div className="flex flex-none items-center gap-2 border-b border-base-300 bg-base-100 px-2 py-1 font-mono text-[10px]">
          <span className="badge badge-xs badge-primary">{selected.kind}</span>
          <span className="truncate">{selected.label}</span>
          {selected.detail && <span className="text-base-content/50">{selected.detail}</span>}
          {selected.offset !== null && (
            <span className="text-base-content/50">@0x{selected.offset.toString(16)}</span>
          )}
        </div>
      )}

      <div className="min-h-0 flex-1 overflow-auto bg-base-100 p-2">
        <svg width={WIDTH} height={height} role="img" aria-label="RSC reference graph">
          {data.edges.map((edge, index) => {
            const from = data.nodes.find((node) => node.id === edge.from);
            const to = data.nodes.find((node) => node.id === edge.to);
            if (!from || !to) return null;
            const a = pos(from);
            const b = pos(to);
            if (!a || !b) return null;
            const mid = (a.x + b.x) / 2;
            const fromWidth = from.kind === "entry" ? 230 : 190;
            return (
              <path
                key={index}
                d={`M ${a.x + fromWidth} ${a.y + 10} C ${mid} ${a.y + 10}, ${mid} ${b.y + 10}, ${b.x} ${b.y + 10}`}
                fill="none"
                stroke="currentColor"
                strokeOpacity={edge.label === "payload" ? 0.45 : 0.2}
                strokeWidth={1}
              />
            );
          })}

          {data.nodes.map((node) => {
            const point = pos(node);
            if (!point) return null;
            const width = node.kind === "file" ? 190 : node.kind === "script" ? 190 : 230;
            return (
              <g
                key={node.id}
                transform={`translate(${point.x}, ${point.y})`}
                className="cursor-pointer"
                onClick={() => setSelected(node)}
              >
                <rect
                  width={width}
                  height={22}
                  rx={5}
                  className={
                    node.kind === "file"
                      ? "fill-primary/15 stroke-primary"
                      : node.kind === "script"
                        ? "fill-secondary/15 stroke-secondary"
                        : "fill-base-200 stroke-base-300"
                  }
                  strokeWidth={1}
                />
                <text x={6} y={15} className="fill-base-content font-mono text-[10px]">
                  {node.kind === "file" ? "文件 " : ""}
                  {truncate(node.label, node.kind === "file" ? 22 : 28)}
                </text>
              </g>
            );
          })}
        </svg>
      </div>
    </div>
  );
}

function truncate(text: string, max: number): string {
  return text.length <= max ? text : `${text.slice(0, max - 1)}…`;
}
