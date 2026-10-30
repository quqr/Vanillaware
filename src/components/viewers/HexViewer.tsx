import { t } from "../../i18n";
import { useEffect, useState } from "react";
import { api, type FileEntry } from "../../lib/ipc";

const HEAD_BYTES = 4096;
const PER_ROW = 16;

export function HexViewer({ entry }: { entry: FileEntry }) {
  const [bytes, setBytes] = useState<Uint8Array | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    setBytes(null);
    setError(null);

    api
      .readFileHead(entry.path, HEAD_BYTES)
      .then((raw) => {
        if (!alive) return;
        const view =
          raw instanceof ArrayBuffer
            ? new Uint8Array(raw)
            : new Uint8Array(raw as unknown as Uint8Array);
        setBytes(view);
      })
      .catch((e) => {
        if (alive) setError(String(e));
      });

    return () => {
      alive = false;
    };
  }, [entry.path]);

  if (error) {
    return <div className="flex flex-1 items-center justify-center p-4 text-sm text-error">{error}</div>;
  }
  if (!bytes) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">
        {t("common.loading")}
      </div>
    );
  }

  const rows: string[] = [];
  for (let offset = 0; offset < bytes.length; offset += PER_ROW) {
    const slice = bytes.subarray(offset, offset + PER_ROW);
    const hex = [...slice].map((b) => b.toString(16).padStart(2, "0")).join(" ");
    const ascii = [...slice]
      .map((b) => (b >= 0x20 && b < 0x7f ? String.fromCharCode(b) : "."))
      .join("");
    rows.push(`${offset.toString(16).padStart(6, "0")}  ${hex.padEnd(PER_ROW * 3 - 1)}  ${ascii}`);
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex-none border-b border-base-300 p-1.5 font-mono text-[10px] text-base-content/50">
        {t("hex.head", { bytes: bytes.length, limit: HEAD_BYTES, size: entry.size })}
      </div>
      <pre className="min-h-0 flex-1 overflow-auto bg-base-100 p-2 font-mono text-[11px] leading-snug">
        {rows.join("\n")}
      </pre>
    </div>
  );
}
