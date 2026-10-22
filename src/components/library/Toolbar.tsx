import { useLibraryStore } from "../../store/libraryStore";
import type { Kind } from "../../lib/ipc";
import { Breadcrumbs } from "./Breadcrumbs";

const KINDS: { id: Kind; label: string }[] = [
  { id: "texture", label: "纹理" },
  { id: "animation", label: "动画" },
  { id: "binary", label: "二进制" },
];

/** 工具栏：打开目录 / 导航 / 搜索 / 类别过滤 / 视图切换 / 索引状态。 */
export function Toolbar({
  treeOpen,
  viewerOpen,
  onToggleTree,
  onToggleViewer,
}: {
  treeOpen: boolean;
  viewerOpen: boolean;
  onToggleTree: () => void;
  onToggleViewer: () => void;
}) {
  const root = useLibraryStore((s) => s.root);
  const phase = useLibraryStore((s) => s.listingPhase);
  const openRoot = useLibraryStore((s) => s.openRoot);
  const back = useLibraryStore((s) => s.back);
  const forward = useLibraryStore((s) => s.forward);
  const up = useLibraryStore((s) => s.up);
  const reload = useLibraryStore((s) => s.reload);
  const canBack = useLibraryStore((s) => s.historyIndex > 0);
  const canForward = useLibraryStore((s) => s.historyIndex < s.history.length - 1);

  const query = useLibraryStore((s) => s.query);
  const setQuery = useLibraryStore((s) => s.setQuery);
  const scope = useLibraryStore((s) => s.scope);
  const setScope = useLibraryStore((s) => s.setScope);
  const kinds = useLibraryStore((s) => s.kinds);
  const toggleKind = useLibraryStore((s) => s.toggleKind);
  const viewMode = useLibraryStore((s) => s.viewMode);
  const setViewMode = useLibraryStore((s) => s.setViewMode);

  const hiddenFiles = useLibraryStore((s) => s.hiddenFiles);
  const index = useLibraryStore((s) => s.index);
  const refreshCache = useLibraryStore((s) => s.refreshCache);
  const cacheEpoch = useLibraryStore((s) => s.cacheEpoch);

  const searching = scope === "all" && query.trim().length > 0;

  return (
    <div className="library-toolbar">
      <div className="bar">
        <button className="btn primary" onClick={() => void openRoot()} disabled={phase === "loading"}>
          打开目录
        </button>

        <button className="btn tiny" onClick={back} disabled={!canBack} title="后退">
          ←
        </button>
        <button className="btn tiny" onClick={forward} disabled={!canForward} title="前进">
          →
        </button>
        <button className="btn tiny" onClick={() => void up()} disabled={!root} title="上一级">
          ↑
        </button>

        <Breadcrumbs />

        <div className="spacer" />

        <button className="btn tiny" data-active={treeOpen} onClick={onToggleTree} title="显示/隐藏目录树">
          树
        </button>
        <button
          className="btn tiny"
          data-active={viewerOpen}
          onClick={onToggleViewer}
          title="显示/隐藏查看器"
        >
          查看器
        </button>

        {index.state === "building" && (
          <span className="badge warn" title="全局搜索在索引完成前不完整">
            索引 {index.count}
          </span>
        )}
        {index.state === "ready" && <span className="badge ok">索引 {index.count}</span>}
        {index.state === "failed" && (
          <span className="badge err" title={index.error ?? ""}>
            索引失败
          </span>
        )}

        <button
          className="btn"
          key={cacheEpoch}
          onClick={() => void refreshCache()}
          title="清空磁盘与内存缓存并重新解码全部缩略图"
        >
          刷新缓存
        </button>
      </div>

      <div className="bar">
        <input
          className="input grow"
          placeholder={scope === "all" ? "在全库中搜索文件名…" : "在当前目录中过滤…"}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          disabled={!root}
        />

        <select
          className="select"
          value={scope}
          onChange={(e) => setScope(e.target.value as "dir" | "all")}
          title="搜索作用域"
        >
          <option value="dir">本目录</option>
          <option value="all">全部目录</option>
        </select>

        <div className="kind-chips">
          {KINDS.map((kind) => (
            <button
              key={kind.id}
              className="chip"
              data-active={kinds.includes(kind.id)}
              onClick={() => toggleKind(kind.id)}
            >
              {kind.label}
            </button>
          ))}
        </div>

        <div className="spacer" />

        {!searching && hiddenFiles > 0 && (
          <span className="dim mono" title="不在格式注册表内，未收录">
            未收录 {hiddenFiles}
          </span>
        )}

        <div className="segmented">
          <button className="seg" data-active={viewMode === "list"} onClick={() => setViewMode("list")}>
            列表
          </button>
          <button className="seg" data-active={viewMode === "grid"} onClick={() => setViewMode("grid")}>
            网格
          </button>
        </div>

        {root && (
          <button className="btn tiny" onClick={() => void reload()} title="重新读取当前目录">
            刷新
          </button>
        )}
      </div>
    </div>
  );
}
