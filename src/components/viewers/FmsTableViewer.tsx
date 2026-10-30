import { t } from "../../i18n";
import { useEffect, useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { api, type FileEntry, type FmsText } from "../../lib/ipc";

export function FmsTableViewer({ entry }: { entry: FileEntry }) {
  const [data, setData] = useState<FmsText | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let alive = true;
    setData(null);
    setError(null);
    setQuery("");
    api
      .docText(entry.path)
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

  const strings = useMemo(() => {
    if (!data) return [];
    const needle = query.trim().toLowerCase();
    if (!needle) return data.strings;
    return data.strings.filter((text) => text.toLowerCase().includes(needle));
  }, [data, query]);

  const virtualizer = useVirtualizer({
    count: strings.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 22,
    overscan: 24,
  });

  if (error) {
    return <div className="flex flex-1 items-center justify-center p-4 text-sm text-error">{error}</div>;
  }
  if (!data) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">
        {t("fms.reading")}
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex flex-none flex-wrap items-center gap-1.5 border-b border-base-300 bg-base-200 p-1.5">
        <span className="badge badge-xs badge-primary">encoding {data.encoding}</span>
        <span className="badge badge-xs">count {data.count}</span>
        <span className="badge badge-xs">match {strings.length}</span>
        <input
          className="input input-xs ml-auto w-56"
          placeholder={t("fms.filter")}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      </div>

      <div className="min-h-0 flex-1 overflow-auto font-mono text-[11px]" ref={scrollRef}>
        <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
          {virtualizer.getVirtualItems().map((item) => (
            <div
              key={item.key}
              className="absolute inset-x-0 flex gap-2 border-b border-base-300/30 px-2 leading-[22px] hover:bg-base-200"
              style={{ transform: `translateY(${item.start}px)`, height: item.size }}
            >
              <span className="w-10 shrink-0 text-right text-base-content/40">{item.index}</span>
              <span className="min-w-0 flex-1 truncate" title={strings[item.index]}>
                {strings[item.index]}
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
