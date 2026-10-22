import { useLibraryStore } from "../../store/libraryStore";
import { TAGS, useAppStore, type TagId } from "../../store/appStore";
import { formatBytes } from "../../lib/format";
import type { FileEntry } from "../../lib/ipc";
import { AtlasViewer } from "../viewers/AtlasViewer";
import { AnimationViewer } from "../viewers/AnimationViewer";
import { HexViewer } from "../viewers/HexViewer";

/** 查看器 id → 组件。新增格式只需在这里加一条（后端负责声明有哪些 id）。 */
const VIEWERS: Record<string, { label: string; render: (entry: FileEntry, tag: string) => React.ReactNode }> = {
  atlas: { label: "图集", render: (entry) => <AtlasViewer entry={entry} /> },
  animation: { label: "动画", render: (entry, tag) => <AnimationViewer entry={entry} tag={tag} /> },
  hex: { label: "十六进制", render: (entry) => <HexViewer entry={entry} /> },
};

const KIND_LABEL: Record<string, string> = {
  texture: "纹理",
  animation: "动画",
  binary: "二进制",
};

/** 右侧查看器：属性 + 查看器 tab + 内容。 */
export function ViewerPanel() {
  const selectedPath = useLibraryStore((s) => s.selectedPath);
  const meta = useLibraryStore((s) => s.meta);
  const metaPhase = useLibraryStore((s) => s.metaPhase);
  const openedPath = useLibraryStore((s) => s.openedPath);
  const openedViewer = useLibraryStore((s) => s.openedViewer);
  const select = useLibraryStore((s) => s.select);
  const open = useLibraryStore((s) => s.open);
  const setViewerTab = useLibraryStore((s) => s.setViewerTab);

  const tag = useAppStore((s) => s.tag);
  const setTag = useAppStore((s) => s.setTag);

  if (!selectedPath) {
    return <aside className="viewer-panel grid-empty">未选择文件</aside>;
  }

  const entry = meta?.entry ?? null;

  return (
    <aside className="viewer-panel">
      <div className="viewer-head">
        <div className="viewer-title" title={selectedPath}>
          {entry?.name ?? selectedPath.split(/[\\/]/).pop()}
        </div>

        {entry && (
          <div className="viewer-meta">
            <span className="badge">{KIND_LABEL[entry.kind] ?? entry.kind}</span>
            <span className="badge ok">{entry.label}</span>
            <span className="mono dim">{formatBytes(entry.size)}</span>
            <span className="mono dim">{formatDate(entry.mtimeMs)}</span>
          </div>
        )}

        {entry?.kind === "animation" && (
          <select
            className="select"
            value={tag}
            onChange={(e) => setTag(e.target.value as TagId)}
            title="解析用的平台标签"
          >
            {TAGS.map((id) => (
              <option key={id} value={id}>
                {id}
              </option>
            ))}
          </select>
        )}
      </div>

      {meta && meta.related.length > 0 && (
        <div className="viewer-related">
          <span className="dim">关联</span>
          {meta.related.map((file) => (
            <button
              key={file.path}
              className="chip"
              title={file.path}
              onClick={() => {
                select(file.path);
                open(file.path);
              }}
            >
              {file.ext.toUpperCase()} · {file.label}
            </button>
          ))}
        </div>
      )}

      {metaPhase === "loading" && <div className="pending-hint">读取中…</div>}

      {metaPhase === "ready" && entry && openedPath === entry.path && (
        <>
          <div className="viewer-tabs">
            {entry.viewers.map((id) => (
              <button
                key={id}
                className="tab"
                data-active={id === openedViewer}
                onClick={() => setViewerTab(id)}
              >
                {VIEWERS[id]?.label ?? id}
              </button>
            ))}
          </div>

          <div className="viewer-content">
            {(VIEWERS[openedViewer ?? ""] ?? VIEWERS.hex).render(entry, tag)}
            {!VIEWERS[openedViewer ?? ""] && (
              <div className="dim mono" style={{ padding: 8 }}>
                未知查看器「{openedViewer}」，已退化为十六进制
              </div>
            )}
          </div>
        </>
      )}
    </aside>
  );
}

function formatDate(ms: number): string {
  if (!ms) return "—";
  const date = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}
