import { t } from "../../i18n";
import { useEffect, useState } from "react";
import { JsonTree } from "../JsonTree";
import { api, type ConvertResult } from "../../lib/ipc";

export function StructurePanel({ path, tag }: { path: string; tag: string }) {
  const [result, setResult] = useState<ConvertResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!path) {
      setResult(null);
      return;
    }

    let alive = true;
    setBusy(true);
    setError(null);

    api
      .convertMbs(path, tag)
      .then((next) => {
        if (alive) setResult(next);
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
  }, [path, tag]);

  async function exportQuad() {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const suggested = path.replace(/\.mbs$/i, "") + ".quad.json";
    const picked = await save({
      defaultPath: suggested,
      filters: [{ name: "Quad JSON", extensions: ["json"] }],
    });
    if (typeof picked !== "string") return;
    try {
      await api.exportQuad(path, tag, picked);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex flex-none items-center gap-2 border-b border-base-300 bg-base-200 px-2 py-1.5">
        <button className="btn btn-xs" disabled={busy} onClick={() => void exportQuad()}>
          {t("structure.exportQuad")}
        </button>
        {busy && <span className="font-mono text-xs text-base-content/50">{t("structure.converting")}</span>}
        <div className="flex-1" />
        {result && (
          <span className={`badge badge-sm ${result.selfCheck.length > 0 ? "badge-error" : "badge-success"}`}>
            self-check {result.selfCheck.length}
          </span>
        )}
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-2 font-mono text-[11px]">
        {error && <div className="alert alert-error alert-soft mb-1.5 p-2 text-xs break-all">{error}</div>}
        {!result && !busy && <div className="text-base-content/50">{t("structure.waiting")}</div>}
        {result && <JsonTree value={result.quad} />}
      </div>

      <div className="max-h-48 shrink-0 overflow-auto border-t border-base-300 bg-base-200 p-2">
        <div className="mb-1.5 text-[10px] uppercase tracking-wider text-base-content/60">{t("structure.selfCheck")}</div>
        {result?.selfCheck.length === 0 && (
          <div className="font-mono text-xs text-base-content/50">{t("doc.checksAll")}</div>
        )}
        {result?.selfCheck.map((issue, index) => (
          <div key={index} className="alert alert-error alert-soft mb-1 p-2 font-mono text-[11px] break-all">
            {issue}
          </div>
        ))}
      </div>
    </div>
  );
}
