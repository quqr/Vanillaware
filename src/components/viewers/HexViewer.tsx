import { useEffect, useState } from "react";
import { api, type FileEntry } from "../../lib/ipc";

const HEAD_BYTES = 4096;
const PER_ROW = 16;

/** 二进制查看器：只读头部 4KB，够看清魔数与版本号，不做整文件加载。 */
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
    return <div className="pending-hint" style={{ color: "var(--error)" }}>{error}</div>;
  }
  if (!bytes) {
    return <div className="pending-hint">读取中…</div>;
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
    <div className="hex-viewer">
      <div className="hex-note dim mono">
        头部 {bytes.length} 字节（上限 {HEAD_BYTES}）· 文件共 {entry.size} 字节
      </div>
      <pre className="hex-dump mono">{rows.join("\n")}</pre>
    </div>
  );
}
