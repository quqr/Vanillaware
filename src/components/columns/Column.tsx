import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useMemo, useRef } from "react";

import { formatBytes } from "../../lib/format";
import type { Column, Row } from "./model";

interface Props {
  column: Column;
  index: number;
  width: number;
  active: boolean;
  filter: string[];
  onActivate(index: number): void;
  onSelect(index: number, path: string): void;
  onEnter(index: number): void;
}

const ROW_HEIGHT = 26;

export function ColumnViewColumn({
  column,
  index,
  width,
  active,
  filter,
  onActivate,
  onSelect,
  onEnter,
}: Props) {
  const scrollRef = useRef<HTMLDivElement | null>(null);

  const visible = useMemo(
    () => column.entries.filter((row) => row.isDir || filter.length === 0 || filter.includes(row.ext)),
    [column.entries, filter],
  );

  const virtualizer = useVirtualizer({
    count: visible.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 12,
  });

  const selected = column.selected;
  useEffect(() => {
    if (!active || !selected) return;
    const at = visible.findIndex((row) => row.path === selected);
    if (at < 0) return;
    virtualizer.scrollToIndex(at, { align: "auto" });
  }, [active, selected, visible, virtualizer]);

  const title = column.dir.split(/[\\/]/).filter(Boolean).pop() ?? column.dir;

  return (
    <div className="flex h-full flex-col" style={{ width }}>
      <div
        className="flex h-7 flex-none items-center gap-1 border-b border-base-300 px-2 text-[10px] uppercase tracking-wide text-base-content/50"
        title={column.dir}
      >
        <span className="truncate">{title}</span>
        {column.hiddenFiles > 0 && (
          <span className="ml-auto font-mono normal-case">+{column.hiddenFiles}</span>
        )}
      </div>

      {column.phase === "loading" && (
        <div className="p-3 font-mono text-[11px] text-base-content/40">...</div>
      )}

      {column.phase === "error" && (
        <div className="m-2 alert alert-error alert-soft text-[11px] break-all">{column.error}</div>
      )}

      {column.phase === "ready" && (
        <div
          ref={scrollRef}
          className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden outline-none"
          onMouseDown={() => onActivate(index)}
        >
          <div className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
            {virtualizer.getVirtualItems().map((item) => {
              const row = visible[item.index];
              return (
                <RowView
                  key={row.path}
                  row={row}
                  top={item.start}
                  selected={column.selected === row.path}
                  active={active}
                  onClick={() => {
                    onActivate(index);
                    onSelect(index, row.path);
                  }}
                  onDoubleClick={() => {
                    onActivate(index);
                    onSelect(index, row.path);
                    onEnter(index);
                  }}
                />
              );
            })}
          </div>
        </div>
      )}

      {column.phase === "ready" && visible.length === 0 && (
        <div className="p-3 text-[11px] text-base-content/40">—</div>
      )}
    </div>
  );
}

function RowView({
  row,
  top,
  selected,
  active,
  onClick,
  onDoubleClick,
}: {
  row: Row;
  top: number;
  selected: boolean;
  active: boolean;
  onClick(): void;
  onDoubleClick(): void;
}) {
  const tone = selected
    ? active
      ? "bg-primary text-primary-content"
      : "bg-base-content/15"
    : "hover:bg-base-content/5";

  return (
    <div
      className={`absolute left-0 flex w-full cursor-default select-none items-center gap-1.5 px-2 text-[12px] ${tone}`}
      style={{ top, height: ROW_HEIGHT }}
      onClick={onClick}
      onDoubleClick={onDoubleClick}
      title={row.path}
    >
      <Glyph row={row} />
      <span className="truncate">{row.name}</span>
      {!row.isDir && (
        <span className="ml-auto flex-none font-mono text-[10px] opacity-50">
          {formatBytes(row.size)}
        </span>
      )}
    </div>
  );
}

function Glyph({ row }: { row: Row }) {
  if (row.isDir) {
    return (
      <svg viewBox="0 0 16 16" className="size-3.5 flex-none opacity-60" aria-hidden>
        {row.kind === "archive" ? (
          <path
            d="M2 4.5A1.5 1.5 0 0 1 3.5 3h3l1.4 1.5h4.6A1.5 1.5 0 0 1 14 6v5.5A1.5 1.5 0 0 1 12.5 13h-9A1.5 1.5 0 0 1 2 11.5v-7Z"
            fill="currentColor"
          />
        ) : (
          <path
            d="M2 4.5A1.5 1.5 0 0 1 3.5 3h3l1.4 1.5h4.6A1.5 1.5 0 0 1 14 6v5.5A1.5 1.5 0 0 1 12.5 13h-9A1.5 1.5 0 0 1 2 11.5v-7Z"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.2"
          />
        )}
      </svg>
    );
  }
  return (
    <svg viewBox="0 0 16 16" className="size-3.5 flex-none opacity-40" aria-hidden>
      <path d="M4 2.5h5l3 3V13a.5.5 0 0 1-.5.5h-7A.5.5 0 0 1 4 13V3a.5.5 0 0 1 .5-.5Z" fill="currentColor" />
    </svg>
  );
}
