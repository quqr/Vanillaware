import { t } from "../../i18n";
import { useEffect, useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { api, type Dialogue, type DialogueEntry, type FileEntry } from "../../lib/ipc";

function hex(value: number): string {
  return `0x${value.toString(16)}`;
}

export function DialogueViewer({ entry }: { entry: FileEntry }) {
  const [data, setData] = useState<Dialogue | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [picked, setPicked] = useState<number | null>(null);

  const sceneRef = useRef<HTMLDivElement>(null);
  const lineRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let alive = true;
    setData(null);
    setError(null);
    setQuery("");
    setPicked(null);
    api
      .dialogueOpen(entry.path)
      .then((next) => {
        if (!alive) return;
        setData(next);
        const first = next.entries.find((item) => item.lines.length > 0) ?? next.entries[0];
        setPicked(first ? next.entries.indexOf(first) : null);
      })
      .catch((e) => {
        if (alive) setError(String(e));
      });
    return () => {
      alive = false;
    };
  }, [entry.path]);

  const scenes = useMemo(() => {
    if (!data) return [];
    const needle = query.trim().toLowerCase();
    return data.entries.filter((item) => {
      if (!needle) return true;
      return (
        item.name.toLowerCase().includes(needle) ||
        String(item.group).includes(needle) ||
        item.lines.some((line) => line.text?.toLowerCase().includes(needle))
      );
    });
  }, [data, query]);

  const current: DialogueEntry | null =
    data && picked !== null ? (data.entries[picked] ?? null) : null;

  const sceneList = useVirtualizer({
    count: scenes.length,
    getScrollElement: () => sceneRef.current,
    estimateSize: () => 34,
    overscan: 16,
  });

  const lines = current?.lines ?? [];
  const lineList = useVirtualizer({
    count: lines.length,
    getScrollElement: () => lineRef.current,
    estimateSize: () => 58,
    overscan: 12,
  });

  if (error) {
    return <div className="flex flex-1 items-center justify-center p-4 text-sm text-error">{error}</div>;
  }
  if (!data) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">
        {t("dialogue.reading")}
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex flex-none flex-wrap items-center gap-1.5 border-b border-base-300 bg-base-200 p-1.5">
        <span className="badge badge-xs badge-primary">{t("dialogue.scenes", { count: data.stats.entries })}</span>
        <span className="badge badge-xs">{t("dialogue.commands", { count: data.stats.commands })}</span>
        <span className="badge badge-xs">{t("dialogue.lines", { count: data.stats.textCommands })}</span>
        <span className="badge badge-xs badge-ghost">{t("dialogue.ids", { count: data.stats.uniqueIds })}</span>
        {data.pool.path ? (
          <span className="badge badge-xs badge-ghost" title={data.pool.path}>
            {t("dialogue.pool", { count: data.pool.strings })}
          </span>
        ) : (
          <span className="badge badge-xs badge-warning">{t("dialogue.noPool")}</span>
        )}
        {data.stats.missingText > 0 && (
          <span className="badge badge-xs badge-warning">
            {t("dialogue.missing", { count: data.stats.missingText })}
          </span>
        )}
        {data.warnings.length > 0 && (
          <span className="badge badge-xs badge-error" title={data.warnings.join("\n")}>
            {t("dialogue.warnings", { count: data.warnings.length })}
          </span>
        )}
        <input
          className="input input-xs ml-auto w-48"
          placeholder={t("dialogue.filter")}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      </div>

      <div className="flex min-h-0 flex-1 overflow-hidden">
        <div ref={sceneRef} className="w-64 flex-none overflow-auto border-r border-base-300">
          <div className="relative w-full" style={{ height: sceneList.getTotalSize() }}>
            {sceneList.getVirtualItems().map((row) => {
              const scene = scenes[row.index];
              const position = data.entries.indexOf(scene);
              const active = position === picked;
              return (
                <button
                  key={`${scene.index}-${scene.name}-${row.index}`}
                  className={`absolute left-0 top-0 flex w-full items-baseline gap-1.5 px-2 py-1.5 text-left text-xs ${
                    active ? "bg-primary/15 text-primary" : "hover:bg-base-200"
                  }`}
                  style={{ height: row.size, transform: `translateY(${row.start}px)` }}
                  onClick={() => setPicked(position)}
                >
                  <span className="truncate font-mono">{scene.name}</span>
                  <span className="ml-auto flex-none text-base-content/40">{scene.lines.length}</span>
                </button>
              );
            })}
          </div>
        </div>

        <div className="flex min-w-0 flex-1 flex-col">
          {current ? (
            <>
              <div className="flex flex-none flex-wrap items-center gap-1.5 border-b border-base-300 px-2 py-1 text-xs">
                <span className="font-mono font-semibold">{current.name}</span>
                <span className="badge badge-xs badge-ghost">group {current.group}</span>
                <span className="badge badge-xs badge-ghost">kind {current.kind}</span>
                <span className="badge badge-xs badge-ghost">@ {hex(current.offset)}</span>
                <span className="badge badge-xs">
                  {t("dialogue.commands", { count: current.commands })}
                </span>
                <span className="badge badge-xs">
                  {t("dialogue.lines", { count: current.lines.length })}
                </span>
              </div>

              {lines.length === 0 ? (
                <div className="flex flex-1 items-center justify-center p-4 text-sm text-base-content/50">
                  {t("dialogue.empty")}
                </div>
              ) : (
                <div ref={lineRef} className="min-h-0 flex-1 overflow-auto">
                  <div className="relative w-full" style={{ height: lineList.getTotalSize() }}>
                    {lineList.getVirtualItems().map((row) => {
                      const line = lines[row.index];
                      return (
                        <div
                          key={`${line.at}-${row.index}`}
                          className="absolute left-0 top-0 w-full border-b border-base-300/40 px-2 py-1.5"
                          style={{ height: row.size, transform: `translateY(${row.start}px)` }}
                        >
                          <div className="flex items-baseline gap-2 text-xs">
                            <span className="flex-none font-semibold text-primary">
                              {line.speaker ?? t("dialogue.noSpeaker")}
                            </span>
                            {line.textId !== null && (
                              <span className="flex-none font-mono text-base-content/40">
                                #{line.textId}
                              </span>
                            )}
                            {line.voice !== null && line.voice !== undefined && (
                              <span className="flex-none text-base-content/40">
                                {t("dialogue.voice", { id: line.voice })}
                              </span>
                            )}
                            <span className="ml-auto flex-none font-mono text-base-content/30">
                              {hex(line.opcode)} · {hex(line.at)}
                            </span>
                          </div>
                          <div className="mt-0.5 whitespace-pre-wrap break-words text-sm">
                            {line.text ?? (
                              <span className="text-base-content/40">{t("dialogue.noText")}</span>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}
            </>
          ) : (
            <div className="flex flex-1 items-center justify-center p-4 text-sm text-base-content/50">
              {t("dialogue.empty")}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
