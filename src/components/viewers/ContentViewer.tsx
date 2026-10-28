import { useEffect, useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { api, type Content, type ContentSection, type FileEntry } from "../../lib/ipc";

export function ContentViewer({ entry }: { entry: FileEntry }) {
  const [content, setContent] = useState<Content | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState(0);

  useEffect(() => {
    let alive = true;
    setContent(null);
    setError(null);
    setTab(0);
    api
      .contentOpen(entry.path)
      .then((next) => {
        if (alive) setContent(next);
      })
      .catch((e) => {
        if (alive) setError(String(e));
      });
    return () => {
      alive = false;
    };
  }, [entry.path]);

  if (error) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 text-sm text-error">{error}</div>
    );
  }
  if (!content) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">
        解析内容…
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex flex-none flex-wrap items-center gap-1.5 border-b border-base-300 bg-base-200 p-1.5 text-[10px]">
        <span className="badge badge-xs badge-primary">{content.title}</span>
        {content.note && <span className="text-base-content/60">{content.note}</span>}
      </div>

      <div className="min-h-0 flex-1 overflow-auto">
        {content.image && (
          <div className="border-b border-base-300 p-2">
            <div className="mb-1 text-[10px] text-base-content/50">
              {content.imageNote ?? "位图（白 = 1）"}
            </div>
            <img
              src={content.image}
              alt="内容位图"
              className="max-w-full bg-base-100 [image-rendering:pixelated]"
              style={{ imageRendering: "pixelated" }}
            />
          </div>
        )}

        {content.series && <SeriesView series={content.series} />}

        {content.text && <SourceView lines={content.text} />}

        {content.sections.length > 0 && (
          <SectionView
            sections={content.sections}
            active={Math.min(tab, content.sections.length - 1)}
            onSelect={setTab}
          />
        )}
      </div>
    </div>
  );
}

function SeriesView({ series }: { series: NonNullable<Content["series"]> }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const height = 120;

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const paint = () => {
      const context = canvas.getContext("2d");
      if (!context) return;
      const rect = canvas.getBoundingClientRect();
      const width = Math.max(1, Math.floor(rect.width));
      const dpr = window.devicePixelRatio || 1;
      canvas.width = Math.floor(width * dpr);
      canvas.height = Math.floor(height * dpr);
      context.setTransform(dpr, 0, 0, dpr, 0, 0);
      context.clearRect(0, 0, width, height);

      const styles = getComputedStyle(canvas);
      const accent = styles.getPropertyValue("--color-primary").trim() || "#3b82f6";
      const points = series.points;
      if (!points.length) return;

      let min = Number.POSITIVE_INFINITY;
      let max = Number.NEGATIVE_INFINITY;
      for (const value of points) {
        if (!Number.isFinite(value)) continue;
        if (value < min) min = value;
        if (value > max) max = value;
      }
      if (!Number.isFinite(min) || !Number.isFinite(max) || min === max) {
        min = 0;
        max = 1;
      }
      const span = max - min;
      const columns = Math.min(points.length, width);
      const step = points.length / columns;
      context.fillStyle = accent;
      for (let column = 0; column < columns; column += 1) {
        const from = Math.floor(column * step);
        const to = Math.max(from + 1, Math.floor((column + 1) * step));
        let low = Number.POSITIVE_INFINITY;
        let high = Number.NEGATIVE_INFINITY;
        for (let index = from; index < to && index < points.length; index += 1) {
          const value = points[index];
          if (!Number.isFinite(value)) continue;
          if (value < low) low = value;
          if (value > high) high = value;
        }
        if (low > high) continue;
        const top = height - ((high - min) / span) * (height - 8) - 4;
        const bottom = height - ((low - min) / span) * (height - 8) - 4;
        context.fillRect(column, top, 1, Math.max(1, bottom - top));
      }
    };
    paint();
    const observer = new ResizeObserver(paint);
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [series, height]);

  return (
    <div className="border-b border-base-300 p-2">
      <div className="mb-1 flex items-center gap-1 text-[10px]">
        <span className="badge badge-xs">{series.label}</span>
        <span className="text-base-content/50">{series.points.length} 点</span>
        {series.note && <span className="text-base-content/40">{series.note}</span>}
      </div>
      <canvas ref={ref} className="block h-[120px] w-full" />
    </div>
  );
}

function SourceView({ lines }: { lines: string[] }) {
  const ref = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: lines.length,
    getScrollElement: () => ref.current,
    estimateSize: () => 18,
    overscan: 30,
  });

  return (
    <div className="border-b border-base-300">
      <div className="flex items-center gap-1 p-1.5 text-[10px]">
        <span className="badge badge-xs">文本</span>
        <span className="text-base-content/50">{lines.length} 行</span>
      </div>
      <div ref={ref} className="h-96 overflow-auto bg-base-100">
        <div className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
          {virtualizer.getVirtualItems().map((row) => (
            <div
              key={row.index}
              className="absolute inset-x-0 flex gap-2 px-2 font-mono text-[11px] leading-[18px]"
              style={{ height: row.size, transform: `translateY(${row.start}px)` }}
            >
              <span className="w-10 flex-none text-right text-base-content/30">
                {row.index + 1}
              </span>
              <span className="whitespace-pre">{lines[row.index]}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function SectionView({
  sections,
  active,
  onSelect,
}: {
  sections: ContentSection[];
  active: number;
  onSelect: (index: number) => void;
}) {
  const section = sections[active];
  const ref = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: section.rows.length,
    getScrollElement: () => ref.current,
    estimateSize: () => 24,
    overscan: 24,
  });

  const columns = useMemo(() => section.columns, [section]);

  return (
    <div className="flex min-h-0 flex-col">
      <div className="flex flex-none flex-wrap items-center gap-1 border-b border-base-300 p-1.5">
        {sections.map((item, index) => (
          <button
            key={item.title}
            className={`btn btn-xs ${index === active ? "btn-primary" : ""}`}
            onClick={() => onSelect(index)}
          >
            {item.title}
            <span className="ml-1 text-[9px] opacity-60">{item.rows.length}</span>
          </button>
        ))}
        <span className="ml-auto text-[10px] text-base-content/40">
          共 {section.total} 行{section.rows.length < section.total ? "（已截断）" : ""}
        </span>
      </div>

      <div className="grid flex-none gap-2 border-b border-base-300 bg-base-200/60 px-2 py-1 font-mono text-[10px] text-base-content/60"
        style={{ gridTemplateColumns: `repeat(${columns.length}, minmax(0, 1fr))` }}
      >
        {columns.map((column) => (
          <span key={column} className="truncate">
            {column}
          </span>
        ))}
      </div>

      <div ref={ref} className="h-96 overflow-auto">
        <div className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
          {virtualizer.getVirtualItems().map((row) => {
            const values = section.rows[row.index] ?? [];
            return (
              <div
                key={row.index}
                className="absolute inset-x-0 grid items-center gap-2 border-b border-base-300/40 px-2 font-mono text-[11px]"
                style={{
                  height: row.size,
                  transform: `translateY(${row.start}px)`,
                  gridTemplateColumns: `repeat(${columns.length}, minmax(0, 1fr))`,
                }}
              >
                {columns.map((_, index) => (
                  <span key={index} className="truncate" title={values[index] ?? ""}>
                    {values[index] ?? ""}
                  </span>
                ))}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
