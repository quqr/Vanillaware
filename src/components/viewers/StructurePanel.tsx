import { useEffect, useState } from "react";
import { JsonTree } from "../JsonTree";
import { api, type ConvertResult } from "../../lib/ipc";

/** 动画结构查看器：单文件 V55 → Quad 的字段级树 + 自检 + 导出（ADR-0008 / ADR-0012）。
 *
 * golden 比对由测试套件负责（`mbs_golden_quad`），界面不提供手选 golden 的入口。
 */
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
    <div className="structure">
      <div className="bar">
        <button className="btn tiny" disabled={busy} onClick={() => void exportQuad()}>
          导出 Quad
        </button>
        {busy && <span className="dim mono">转换中…</span>}
        <div className="spacer" />
        {result && (
          <span className={`badge ${result.selfCheck.length > 0 ? "err" : "ok"}`}>
            self-check {result.selfCheck.length}
          </span>
        )}
      </div>

      <div className="json-panel">
        {error && <div className="issue">{error}</div>}
        {!result && !busy && <div className="dim">等待转换结果</div>}
        {result && <JsonTree value={result.quad} />}
      </div>

      <div className="report-panel">
        <div className="section-title">自检（结构不变量）</div>
        {result?.selfCheck.length === 0 && <div className="dim mono">全部成立</div>}
        {result?.selfCheck.map((issue, index) => (
          <div key={index} className="issue">
            {issue}
          </div>
        ))}
      </div>
    </div>
  );
}
