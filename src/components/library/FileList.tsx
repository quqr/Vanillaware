import { useEffect, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useLibraryStore } from "../../store/libraryStore";
import { formatBytes } from "../../lib/format";
import { useRows } from "./rows";
import { Thumb } from "./icons";
import { noteListScroll } from "../../lib/thumbCache";

/** 列表视图：虚拟化行，键盘可选、双击打开。 */
export function FileList() {
  const { rows, searching } = useRows();
  const selectedPath = useLibraryStore((s) => s.selectedPath);
  const select = useLibraryStore((s) => s.select);
  const open = useLibraryStore((s) => s.open);
  const phase = useLibraryStore((s) => s.listingPhase);
  const root = useLibraryStore((s) => s.root);
  const error = useLibraryStore((s) => s.listingError);
  const hiddenFiles = useLibraryStore((s) => s.hiddenFiles);

  const scrollRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 30,
    overscan: 16,
  });

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: 0 });
  }, [rows.length]);

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
        {!searching && hiddenFiles > 0 && phase === "ready" && (
          <span className="dim mono" style={{ display: "block", marginTop: 6 }}>
            另有 {hiddenFiles} 个文件不在格式注册表内，未收录
          </span>
        )}
      </div>
    );
  }

  return (
    <div
      className="file-list"
      ref={scrollRef}
      onScroll={noteListScroll}
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.key !== "Enter") return;
        const target = selectedPath ?? rows[0]?.path;
        if (target) open(target);
      }}
    >
      <div className="file-list-head">
        <span />
        <span>名称</span>
        <span>格式</span>
        <span>{searching ? "目录" : "大小"}</span>
        <span>修改时间</span>
      </div>

      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((item) => {
          const row = rows[item.index];
          return (
            <div
              key={row.path}
              className="file-row"
              data-selected={row.path === selectedPath}
              style={{ transform: `translateY(${item.start}px)`, height: item.size }}
              title={row.path}
              onClick={(event) => select(row.path, event.ctrlKey || event.metaKey || event.shiftKey)}
              onDoubleClick={() => open(row.path)}
            >
              <span className="cell-icon">
                <Thumb path={row.path} kind={row.kind} size={20} hasThumb={row.hasThumb} deferred />
              </span>
              <span className="cell-name">{row.name}</span>
              <span className="cell-format">{row.label}</span>
              <span className="cell-meta mono">
                {searching ? shorten(row.dir ?? "") : row.size === null ? "—" : formatBytes(row.size)}
              </span>
              <span className="cell-meta mono">
                {row.mtimeMs ? formatDate(row.mtimeMs) : "—"}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function shorten(path: string): string {
  if (path.length <= 28) return path;
  return `…${path.slice(path.length - 27)}`;
}

function formatDate(ms: number): string {
  const date = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}
