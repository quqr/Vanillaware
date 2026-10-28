import { useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { api, type Asm, type FileEntry } from "../../lib/ipc";

export function AsmViewer({ entry }: { entry: FileEntry }) {
  const [data, setData] = useState<Asm | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [mode, setMode] = useState<"asm" | "pseudo">("asm");
  const [pseudo, setPseudo] = useState<string | null>(null);
  const pathRef = useRef(entry.path);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let alive = true;
    pathRef.current = entry.path;
    setData(null);
    setPseudo(null);
    setMode("asm");
    setError(null);
    setBusy(true);

    api
      .docAsm(entry.path, 0)
      .then((next) => {
        if (alive) setData(next);
      })
      .catch((e) => {
        if (alive) setError(String(e));
      })
      .finally(() => {
        if (alive) setBusy(false);
      });

    return () => {
      alive = false;
    };
  }, [entry.path]);

  const lines = data?.lines ?? [];
  const virtualizer = useVirtualizer({
    count: lines.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 18,
    overscan: 24,
  });

  function loadPseudo(index: number) {
    const path = entry.path;
    setPseudo(null);
    setBusy(true);
    setError(null);
    api
      .docPseudo(path, index)
      .then((next) => {
        if (pathRef.current === path) setPseudo(next.text);
      })
      .catch((e) => {
        if (pathRef.current === path) setError(String(e));
      })
      .finally(() => {
        if (pathRef.current === path) setBusy(false);
      });
  }

  function pick(index: number) {
    const path = entry.path;
    setBusy(true);
    setError(null);
    setPseudo(null);
    api
      .docAsm(path, index)
      .then((next) => {
        if (pathRef.current === path) {
          setData(next);
          if (mode === "pseudo") loadPseudo(next.selected);
        }
      })
      .catch((e) => {
        if (pathRef.current === path) setError(String(e));
      })
      .finally(() => {
        if (pathRef.current === path) setBusy(false);
      });
  }

  function showPseudo() {
    setMode("pseudo");
    if (data && pseudo === null) loadPseudo(data.selected);
  }

  if (error) {
    return <div className="flex flex-1 items-center justify-center p-4 text-sm text-error">{error}</div>;
  }
  if (!data) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">
        {busy ? "解析模块…" : "无内容"}
      </div>
    );
  }

  const current = data.functions[data.selected];

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      <div className="w-56 shrink-0 overflow-auto border-r border-base-300 bg-base-200">
        <ul className="menu menu-xs w-full p-0">
          {data.functions.map((fn) => (
            <li key={fn.index}>
              <button
                className={fn.index === data.selected ? "menu-active" : ""}
                title={`${fn.ret} ${fn.name}(${fn.params}) · ${fn.bytecode} 指令`}
                onClick={() => pick(fn.index)}
              >
                <span className="truncate">{fn.name || `func[${fn.index}]`}</span>
                <span className="font-mono text-base-content/50">{fn.bytecode}</span>
              </button>
            </li>
          ))}
        </ul>
      </div>

      <div className="flex min-w-0 flex-1 flex-col overflow-hidden">
        <div className="flex flex-none flex-wrap items-center gap-1.5 border-b border-base-300 bg-base-200 p-1.5 font-mono text-[10px]">
          <span className="badge badge-xs badge-primary">{current?.ret}</span>
          <span className="truncate">{current?.name || "—"}</span>
          <span className="badge badge-xs">params {current?.params ?? 0}</span>
          <span className="badge badge-xs">ops {current?.bytecode ?? 0}</span>
          <div className="join">
            <button
              className={`btn btn-xs join-item ${mode === "asm" ? "btn-active" : ""}`}
              onClick={() => setMode("asm")}
            >
              指令
            </button>
            <button
              className={`btn btn-xs join-item ${mode === "pseudo" ? "btn-active" : ""}`}
              onClick={showPseudo}
            >
              伪代码
            </button>
          </div>
          {busy && <span className="text-base-content/50">读取中…</span>}
        </div>

        {mode === "asm" ? (
          <div className="min-h-0 flex-1 overflow-auto font-mono text-[11px]" ref={scrollRef}>
            <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
              {virtualizer.getVirtualItems().map((item) => {
                const line = lines[item.index];
                return (
                  <div
                    key={line.index}
                    className="absolute inset-x-0 flex gap-1.5 px-1.5 leading-[18px] hover:bg-base-200"
                    style={{ transform: `translateY(${item.start}px)`, height: item.size }}
                  >
                    <span className="w-10 shrink-0 text-right text-base-content/40">{line.index}</span>
                    <span className="w-24 shrink-0 text-primary">{line.op}</span>
                    <span className="min-w-0 flex-1 break-all">
                      {line.args.join(", ")}
                      {line.note && <span className="ml-2 text-base-content/50">; {line.note}</span>}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        ) : (
          <pre className="min-h-0 flex-1 overflow-auto p-2 font-mono text-[11px] leading-snug whitespace-pre-wrap">
            {pseudo ?? (busy ? "生成中…" : "无内容")}
          </pre>
        )}
      </div>
    </div>
  );
}
