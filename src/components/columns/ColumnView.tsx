import { useEffect, useRef } from "react";

import { t } from "../../i18n";
import { useLibraryStore, COLUMN_WIDTH_DEFAULT } from "../../store/libraryStore";
import { ColumnViewColumn } from "./Column";
import { firstByLetter } from "./model";

const MIN_WIDTH = 160;
const MAX_WIDTH = 520;
const TYPE_AHEAD_RESET = 700;

export function ColumnView() {
  const columns = useLibraryStore((state) => state.columns);
  const activeColumn = useLibraryStore((state) => state.activeColumn);
  const widths = useLibraryStore((state) => state.columnWidths);
  const filter = useLibraryStore((state) => state.formatFilter);
  const selectRow = useLibraryStore((state) => state.selectRow);
  const enterRow = useLibraryStore((state) => state.enterRow);
  const selectSibling = useLibraryStore((state) => state.selectSibling);
  const setActiveColumn = useLibraryStore((state) => state.setActiveColumn);
  const setColumnWidth = useLibraryStore((state) => state.setColumnWidth);
  const up = useLibraryStore((state) => state.up);
  const unpin = useLibraryStore((state) => state.unpin);

  const scroller = useRef<HTMLDivElement | null>(null);
  const typed = useRef({ buffer: "", at: 0 });

  useEffect(() => {
    const node = scroller.current;
    if (!node) return;
    const target = node.querySelector<HTMLElement>(`[data-column="${activeColumn}"]`);
    target?.scrollIntoView({ inline: "nearest", block: "nearest" });
  }, [activeColumn, columns.length]);

  function visibleRows(column: (typeof columns)[number]) {
    return column.entries.filter(
      (row) => row.isDir || filter.length === 0 || filter.includes(row.ext),
    );
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    const column = columns[activeColumn];
    if (!column) return;

    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        selectSibling(1);
        return;
      case "ArrowUp":
        event.preventDefault();
        selectSibling(-1);
        return;
      case "ArrowRight":
        event.preventDefault();
        enterRow(activeColumn);
        return;
      case "ArrowLeft":
        event.preventDefault();
        setActiveColumn(activeColumn - 1);
        return;
      case "Enter":
        event.preventDefault();
        enterRow(activeColumn);
        return;
      case "Backspace":
        event.preventDefault();
        void up();
        return;
      case "Escape":
        unpin();
        return;
      case "Home":
      case "End": {
        const visible = visibleRows(column);
        if (visible.length === 0) return;
        event.preventDefault();
        selectRow(activeColumn, visible[event.key === "Home" ? 0 : visible.length - 1].path);
        return;
      }
      default:
        break;
    }

    if (event.key.length === 1 && /[a-zA-Z0-9]/.test(event.key) && !event.ctrlKey && !event.metaKey) {
      const now = Date.now();
      const state = typed.current;
      state.buffer = now - state.at > TYPE_AHEAD_RESET ? event.key : `${state.buffer}${event.key}`;
      state.at = now;
      const hit = firstByLetter(visibleRows(column), state.buffer);
      if (hit) selectRow(activeColumn, hit);
    }
  }

  if (columns.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center p-6 text-sm text-base-content/40">
        <span>{t("library.pickRoot")}</span>
      </div>
    );
  }

  return (
    <div
      ref={scroller}
      className="flex min-h-0 flex-1 flex-row items-stretch overflow-x-auto outline-none"
      tabIndex={0}
      onKeyDown={onKeyDown}
    >
      {columns.map((column, index) => (
        <div key={`${column.dir}#${index}`} className="flex h-full flex-row" data-column={index}>
          <ColumnViewColumn
            column={column}
            index={index}
            width={widths[index] ?? COLUMN_WIDTH_DEFAULT}
            active={index === activeColumn}
            filter={filter}
            onActivate={setActiveColumn}
            onSelect={selectRow}
            onEnter={enterRow}
          />
          <Resizer
            width={widths[index] ?? COLUMN_WIDTH_DEFAULT}
            onResize={(next) =>
              setColumnWidth(index, Math.min(Math.max(next, MIN_WIDTH), MAX_WIDTH))
            }
          />
        </div>
      ))}
    </div>
  );
}

function Resizer({ width, onResize }: { width: number; onResize(next: number): void }) {
  const origin = useRef({ x: 0, width: 0 });
  return (
    <div
      className="w-[3px] flex-none cursor-col-resize bg-base-300/40 transition-colors hover:bg-primary/50"
      onPointerDown={(event) => {
        origin.current = { x: event.clientX, width };
        event.currentTarget.setPointerCapture(event.pointerId);
      }}
      onPointerMove={(event) => {
        if (!event.currentTarget.hasPointerCapture(event.pointerId)) return;
        onResize(origin.current.width + (event.clientX - origin.current.x));
      }}
      onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
    />
  );
}
