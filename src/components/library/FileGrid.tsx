import { t } from "../../i18n";
import { useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useLibraryStore } from "../../store/libraryStore";
import { useRows } from "./rows";
import { Thumb } from "./icons";

const CARD_W = 148;
const CARD_H = 168;
const GAP = 12;

export function FileGrid() {
  const { rows, searching } = useRows();
  const selectedPath = useLibraryStore((s) => s.selectedPath);
  const select = useLibraryStore((s) => s.select);
  const open = useLibraryStore((s) => s.open);
  const phase = useLibraryStore((s) => s.listingPhase);
  const root = useLibraryStore((s) => s.root);
  const error = useLibraryStore((s) => s.listingError);
  const formatFilter = useLibraryStore((s) => s.formatFilter);
  const query = useLibraryStore((s) => s.query);
  const hiddenFiles = useLibraryStore((s) => s.hiddenFiles);

  const scrollRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);

  useEffect(() => {
    const node = scrollRef.current;
    if (!node) return;
    const observer = new ResizeObserver(([entry]) => setWidth(entry.contentRect.width));
    observer.observe(node);
    setWidth(node.clientWidth);
    return () => observer.disconnect();
  }, [rows.length]);

  const cols = Math.max(1, Math.floor((width - GAP) / (CARD_W + GAP)));
  const rowCount = Math.ceil(rows.length / cols);

  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => CARD_H + GAP,
    overscan: 6,
  });

  if (phase === "error") {
    return <div className="p-4 text-sm text-error">{error}</div>;
  }
  if (!root) {
    return <div className="p-4 text-sm text-base-content/60">{t("library.openHint")}</div>;
  }
  if (rows.length === 0) {
    return (
      <div className="p-4 text-sm text-base-content/60">
        {phase === "loading"
          ? t("common.loading")
          : searching || query.trim().length > 0
            ? t("library.noMatch")
            : formatFilter.length > 0
              ? t("library.filterEmpty")
              : t("library.emptyDirectory")}
        {!searching && hiddenFiles > 0 && phase === "ready" && (
          <span className="mt-1.5 block font-mono text-xs text-base-content/40">
            {t("library.hiddenFiles", { count: hiddenFiles })}
          </span>
        )}
      </div>
    );
  }

  return (
    <div
      className="min-h-0 flex-1 overflow-auto p-2.5 outline-none"
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
              className="absolute left-0 grid"
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
                  className={`card card-sm flex h-full flex-col items-center gap-1 overflow-hidden border bg-base-100 p-2 text-center ${
                    row.path === selectedPath ? "border-primary bg-primary/10" : "border-base-300"
                  }`}
                  title={row.path}
                  onClick={(event) => select(row.path, event.ctrlKey || event.metaKey || event.shiftKey)}
                  onDoubleClick={() => open(row.path)}
                >
                  <span className="flex h-28 items-center justify-center bg-base-200">
                    <Thumb path={row.path} icon={row.icon} size={112} hasThumb={row.hasThumb} />
                  </span>
                  <span className="w-full truncate px-1 text-[11px]">{row.name}</span>
                  <span className="font-mono text-[10px] text-base-content/50">{row.label}</span>
                </button>
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}
