import { useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useLibraryStore } from "../../store/libraryStore";
import { useRows } from "./rows";
import { Thumb } from "./icons";

const CARD_W = 148;
const CARD_H = 168;
const GAP = 12;

/** 网格视图：卡片式，缩略图优先，适合一眼扫图。 */
export function FileGrid() {
  const { rows, searching } = useRows();
  const selectedPath = useLibraryStore((s) => s.selectedPath);
  const select = useLibraryStore((s) => s.select);
  const open = useLibraryStore((s) => s.open);
  const phase = useLibraryStore((s) => s.listingPhase);
  const root = useLibraryStore((s) => s.root);
  const error = useLibraryStore((s) => s.listingError);

  const scrollRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);

  useEffect(() => {
    const node = scrollRef.current;
    if (!node) return;
    const observer = new ResizeObserver(([entry]) => setWidth(entry.contentRect.width));
    observer.observe(node);
    setWidth(node.clientWidth);
    return () => observer.disconnect();
  }, []);

  const cols = Math.max(1, Math.floor((width - GAP) / (CARD_W + GAP)));
  const rowCount = Math.ceil(rows.length / cols);

  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => CARD_H + GAP,
    overscan: 6,
  });

  if (phase === "error") {
    return <div className="list-empty" style={{ color: "var(--error)" }}>{error}</div>;
  }
  if (!root) {
    return <div className="list-empty">打开一个目录开始浏览</div>;
  }
  if (rows.length === 0) {
    return (
      <div className="list-empty">
        {phase === "loading" ? "读取中…" : searching ? "没有匹配的资源" : "此目录没有收录的资源"}
      </div>
    );
  }

  return (
    <div
      className="file-grid"
      ref={scrollRef}
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.key !== "Enter") return;
        const target = selectedPath ?? rows[0]?.path;
        if (target) open(target);
      }}
    >
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((virtualRow) => {
          const start = virtualRow.index * cols;
          const slice = rows.slice(start, start + cols);
          return (
            <div
              key={virtualRow.key}
              className="file-grid-row"
              style={{
                transform: `translateY(${virtualRow.start}px)`,
                height: CARD_H,
                gap: GAP,
                gridTemplateColumns: `repeat(${cols}, ${CARD_W}px)`,
              }}
            >
              {slice.map((row) => (
                <button
                  key={row.path}
                  className="file-card"
                  data-selected={row.path === selectedPath}
                  title={row.path}
                  onClick={(event) => select(row.path, event.ctrlKey || event.metaKey || event.shiftKey)}
                  onDoubleClick={() => open(row.path)}
                >
                  <span className="file-card-thumb">
                    <Thumb path={row.path} kind={row.kind} size={112} hasThumb={row.hasThumb} />
                  </span>
                  <span className="file-card-name">{row.name}</span>
                  <span className="file-card-format mono">{row.label}</span>
                </button>
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}
