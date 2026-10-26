import { useLibraryStore } from "../../store/libraryStore";
import { TAGS, useAppStore, type TagId } from "../../store/appStore";
import { formatBytes } from "../../lib/format";
import type { FileEntry } from "../../lib/ipc";
import { AsmViewer } from "../viewers/AsmViewer";
import { AtlasViewer } from "../viewers/AtlasViewer";
import { BbsLayoutViewer } from "../viewers/BbsLayoutViewer";
import { AnimationViewer } from "../viewers/AnimationViewer";
import { DocViewer } from "../viewers/DocViewer";
import { FmdMeshViewer } from "../viewers/FmdMeshViewer";
import { FmsTableViewer } from "../viewers/FmsTableViewer";
import { HexViewer } from "../viewers/HexViewer";
import { OsbSceneViewer } from "../viewers/OsbSceneViewer";
import { RscGraphViewer } from "../viewers/RscGraphViewer";

/** 查看器 id → 组件。新增格式只需在这里加一条（后端负责声明有哪些 id）。 */
const VIEWERS: Record<string, { label: string; render: (entry: FileEntry, tag: string) => React.ReactNode }> = {
  asm: { label: "反汇编", render: (entry) => <AsmViewer entry={entry} /> },
  atlas: { label: "图集", render: (entry) => <AtlasViewer entry={entry} /> },
  animation: { label: "动画", render: (entry, tag) => <AnimationViewer entry={entry} tag={tag} /> },
  doc: { label: "结构", render: (entry, tag) => <DocViewer entry={entry} tag={tag} /> },
  layout: { label: "布局", render: (entry) => <BbsLayoutViewer entry={entry} /> },
  mesh: { label: "模型", render: (entry) => <FmdMeshViewer entry={entry} /> },
  scene: { label: "场景", render: (entry) => <OsbSceneViewer entry={entry} /> },
  graph: { label: "引用图", render: (entry) => <RscGraphViewer entry={entry} /> },
  text: { label: "文本", render: (entry) => <FmsTableViewer entry={entry} /> },
  hex: { label: "十六进制", render: (entry) => <HexViewer entry={entry} /> },
};

const KIND_LABEL: Record<string, string> = {
  texture: "纹理",
  animation: "动画",
  scene: "场景",
  model: "模型",
  script: "脚本",
  text: "文本",
  list: "列表",
  grid: "网格",
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
    return (
      <aside className="flex flex-1 items-center justify-center p-4 text-sm text-base-content/50">
        未选择文件
      </aside>
    );
  }

  const entry = meta?.entry ?? null;
  // 切换文件后旧 tab 可能不属于新格式（例如 asb → bbs 仍保留 asm）；
  // 统一回退到该格式声明的第一个查看器。
  const activeViewer =
    entry && openedViewer && entry.viewers.includes(openedViewer)
      ? openedViewer
      : (entry?.viewers[0] ?? null);

  return (
    <aside className="flex flex-1 flex-col overflow-hidden">
      <div className="flex flex-col gap-1.5 border-b border-base-300 p-2">
        <div className="truncate font-mono text-xs font-semibold" title={selectedPath}>
          {entry?.name ?? selectedPath.split(/[\\/]/).pop()}
        </div>

        {entry && (
          <div className="flex flex-wrap items-center gap-1.5 text-[10px]">
            <span className="badge badge-sm">{KIND_LABEL[entry.kind] ?? entry.kind}</span>
            <span className="badge badge-sm badge-primary">{entry.label}</span>
            <span className="font-mono text-base-content/50">{formatBytes(entry.size)}</span>
            <span className="font-mono text-base-content/50">{formatDate(entry.mtimeMs)}</span>
          </div>
        )}

        {(entry?.ext === "mbs" || entry?.ext === "mbp") && (
          <select
            className="select select-sm"
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
        <div className="flex flex-wrap items-center gap-1.5 border-b border-base-300 p-1.5 text-[10px]">
          <span className="text-base-content/50">关联</span>
          {meta.related.map((file) => (
            <button
              key={file.path}
              className="btn btn-xs"
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

      {metaPhase === "loading" && (
        <div className="flex flex-1 items-center justify-center p-4 font-mono text-xs text-base-content/50">
          读取中…
        </div>
      )}

      {metaPhase === "error" && (
        <div className="m-2 alert alert-error alert-soft text-xs">
          无法读取该文件的元信息
        </div>
      )}

      {metaPhase === "ready" && entry && openedPath === entry.path && (
        <>
          <div role="tablist" className="tabs tabs-border tabs-xs px-2">
            {entry.viewers.map((id) => (
              <button
                key={id}
                role="tab"
                className={`tab ${id === activeViewer ? "tab-active" : ""}`}
                onClick={() => setViewerTab(id)}
              >
                {VIEWERS[id]?.label ?? id}
              </button>
            ))}
          </div>

          <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
            {(VIEWERS[activeViewer ?? ""] ?? VIEWERS.hex).render(entry, tag)}
            {activeViewer && !VIEWERS[activeViewer] && (
              <div className="p-2 font-mono text-xs text-base-content/60">
                未知查看器「{activeViewer}」，已退化为十六进制
              </div>
            )}
          </div>
        </>
      )}

      {metaPhase === "ready" && entry && openedPath !== entry.path && (
        <div className="flex flex-1 items-center justify-center p-4 text-xs text-base-content/50">
          双击文件打开查看器
        </div>
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
