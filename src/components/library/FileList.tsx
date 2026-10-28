import { t } from "../../i18n";
import { useEffect, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useLibraryStore } from "../../store/libraryStore";
import { formatBytes } from "../../lib/format";
import { useRows } from "./rows";
import { Thumb } from "./icons";
import { noteListScroll } from "../../lib/thumbCache";

const GRID = "grid grid-cols-[28px_minmax(0,1fr)_76px_110px_130px] items-center gap-2 px-2.5";

export function FileList() {
  const { rows, searching } = useRows();
  const selectedPath = useLibraryStore((s) => s.selectedPath);
  const select = useLibraryStore((s) => s.select);
  const open = useLibraryStore((s) => s.open);
  const phase = useLibraryStore((s) => s.listingPhase);
  const root = useLibraryStore((s) => s.root);
  const error = useLibraryStore((s) => s.listingError);
  const hiddenFiles = useLibraryStore((s) => s.hiddenFiles);
  const formatFilter = useLibraryStore((s) => s.formatFilter);
  const query = useLibraryStore((s) => s.query);

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
      className="min-h-0 flex-1 overflow-auto outline-none"
      ref={scrollRef}
      onScroll={noteListScroll}
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.key !== "Enter") return;
        const target = selectedPath ?? rows[0]?.path;
        if (target) open(target);
      }}
    >
      <div
        className={`${GRID} sticky top-0 z-10 h-7 border-b border-base-300 bg-base-200 font-mono text-[10px] uppercase tracking-wider text-base-content/60`}
      >
        <span />
        <span>{t("library.columnName")}</span>
        <span>{t("library.columnFormat")}</span>
        <span>{searching ? t("library.directory") : t("library.columnSize")}</span>
        <span>{t("library.columnModified")}</span>
      </div>

      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((item) => {
          const row = rows[item.index];
          return (
            <div
              key={row.path}
              className={`${GRID} absolute inset-x-0 border-b border-base-300/40 hover:bg-base-200 ${
                row.path === selectedPath ? "bg-primary/10" : ""
              }`}
              style={{ transform: `translateY(${item.start}px)`, height: item.size }}
              title={row.path}
              onClick={(event) => select(row.path, event.ctrlKey || event.metaKey || event.shiftKey)}
              onDoubleClick={() => open(row.path)}
            >
              <span className="flex items-center justify-center">
                <Thumb path={row.path} icon={row.icon} size={20} hasThumb={row.hasThumb} deferred />
              </span>
              <span className="truncate">{row.name}</span>
              <span className="font-mono text-[10px] text-base-content/50">{row.label}</span>
              <span className="truncate font-mono text-[10px] text-base-content/50">
                {searching ? shorten(row.dir ?? "") : row.size === null ? "—" : formatBytes(row.size)}
              </span>
              <span className="truncate font-mono text-[10px] text-base-content/50">
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
