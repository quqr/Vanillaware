import { useLibraryStore } from "../../store/libraryStore";
import { PROFILE_PRESETS, THEMES, useAppStore, type ProfileMode } from "../../store/appStore";
import { Breadcrumbs } from "./Breadcrumbs";

const BAR = "flex flex-wrap items-center gap-2 border-b border-base-300 bg-base-200 px-2 py-1.5";

/** 工具栏：打开目录 / 导航 / 搜索 / 类别过滤 / 视图切换 / 主题 / 索引状态。 */
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
  const availableFormats = useLibraryStore((s) => s.availableFormats);
  const formatFilter = useLibraryStore((s) => s.formatFilter);
  const toggleFormat = useLibraryStore((s) => s.toggleFormat);
  const clearFormats = useLibraryStore((s) => s.clearFormats);
  const viewMode = useLibraryStore((s) => s.viewMode);
  const setViewMode = useLibraryStore((s) => s.setViewMode);

  const hiddenFiles = useLibraryStore((s) => s.hiddenFiles);
  const index = useLibraryStore((s) => s.index);
  const refreshCache = useLibraryStore((s) => s.refreshCache);
  const cacheEpoch = useLibraryStore((s) => s.cacheEpoch);

  const theme = useAppStore((s) => s.theme);
  const setTheme = useAppStore((s) => s.setTheme);
  const profile = useAppStore((s) => s.profile);
  const profileMode = useAppStore((s) => s.profileMode);
  const setProfileMode = useAppStore((s) => s.setProfileMode);

  const searching = scope === "all" && query.trim().length > 0;

  return (
    <div className="flex flex-none flex-col">
      <div className={BAR}>
        <button className="btn btn-sm btn-primary" onClick={() => void openRoot()}>
          打开目录
        </button>

        <button className="btn btn-sm btn-square btn-ghost" onClick={back} disabled={!canBack} title="后退">
          ←
        </button>
        <button className="btn btn-sm btn-square btn-ghost" onClick={forward} disabled={!canForward} title="前进">
          →
        </button>
        <button className="btn btn-sm btn-square btn-ghost" onClick={() => void up()} disabled={!root} title="上一级">
          ↑
        </button>

        <Breadcrumbs />

        <div className="flex-1" />

        <details className="dropdown dropdown-end">
          <summary className="btn btn-sm btn-ghost gap-1.5" title="游戏档案">
            档案{" "}
            {profileMode === "auto"
              ? profile?.label ?? "自动"
              : PROFILE_PRESETS[profileMode].label}
          </summary>
          <ul className="dropdown-content menu menu-sm z-50 mt-2 w-44 rounded-box bg-base-100 p-1 shadow">
            {(["auto", "unicorn_overlord", "thirteen_sentinels"] as ProfileMode[]).map((mode) => (
              <li key={mode}>
                <button
                  className={profileMode === mode ? "menu-active" : ""}
                  onClick={(event) => {
                    const details = event.currentTarget.closest("details");
                    if (details instanceof HTMLDetailsElement) details.open = false;
                    setProfileMode(mode);
                  }}
                >
                  {mode === "auto" ? "自动嗅探" : PROFILE_PRESETS[mode].label}
                </button>
              </li>
            ))}
          </ul>
        </details>

        <details className="dropdown dropdown-end">
          <summary className="btn btn-sm btn-ghost gap-1.5" title="切换主题">
            <span className="status status-primary status-xs" />
            {theme}
          </summary>
          <ul className="dropdown-content menu menu-sm z-50 mt-2 w-40 rounded-box bg-base-100 p-1 shadow">
            {THEMES.map((name) => (
              <li key={name}>
                <label
                  className="justify-between"
                  onClick={(event) => {
                    const details = event.currentTarget.closest("details");
                    if (details instanceof HTMLDetailsElement) details.open = false;
                  }}
                >
                  {name}
                  <input
                    type="radio"
                    name="theme"
                    className="theme-controller radio radio-xs"
                    value={name}
                    checked={name === theme}
                    onChange={() => setTheme(name)}
                  />
                </label>
              </li>
            ))}
          </ul>
        </details>

        <button
          className={`btn btn-sm btn-square btn-ghost ${treeOpen ? "btn-active" : ""}`}
          onClick={onToggleTree}
          title="显示/隐藏目录树"
        >
          树
        </button>
        <button
          className={`btn btn-sm btn-ghost ${viewerOpen ? "btn-active" : ""}`}
          onClick={onToggleViewer}
          title="显示/隐藏查看器"
        >
          查看器
        </button>

        {index.state === "building" && (
          <span className="badge badge-sm badge-warning" title="全局搜索在索引完成前不完整">
            索引 {index.count}
          </span>
        )}
        {index.state === "ready" && <span className="badge badge-sm badge-success">索引 {index.count}</span>}
        {index.state === "failed" && (
          <span className="badge badge-sm badge-error" title={index.error ?? ""}>
            索引失败
          </span>
        )}

        <button
          className="btn btn-sm btn-ghost"
          key={cacheEpoch}
          onClick={() => void refreshCache()}
          title="清空磁盘与内存缓存并重新解码全部缩略图"
        >
          刷新缓存
        </button>
      </div>

      <div className={BAR}>
        <input
          className="input input-sm min-w-20 flex-1"
          placeholder={scope === "all" ? "在全库中搜索文件名…" : "在当前目录中过滤…"}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          disabled={!root}
        />

        <select
          className="select select-sm w-28"
          value={scope}
          onChange={(e) => setScope(e.target.value as "dir" | "all")}
          title="搜索作用域"
        >
          <option value="dir">本目录</option>
          <option value="all">全部目录</option>
        </select>

        <details className="dropdown">
          <summary className="btn btn-xs">
            格式{" "}
            {formatFilter.length === 0
              ? "全部"
              : `${formatFilter.length}/${availableFormats.length}`}
          </summary>
          <ul className="dropdown-content menu menu-xs z-50 mt-2 max-h-80 w-64 overflow-auto rounded-box bg-base-100 p-1 shadow">
            <li>
              <button onClick={clearFormats}>全部格式</button>
            </li>
            {availableFormats.map((format) => (
              <li key={format.ext}>
                <label className="justify-between">
                  <span>
                    {format.label}
                    <span className="ml-1 font-mono text-base-content/50">.{format.ext}</span>
                  </span>
                  <input
                    type="checkbox"
                    className="checkbox checkbox-xs"
                    checked={formatFilter.includes(format.ext)}
                    onChange={() => toggleFormat(format.ext)}
                  />
                </label>
              </li>
            ))}
          </ul>
        </details>

        <div className="flex-1" />

        {!searching && hiddenFiles > 0 && (
          <span className="font-mono text-xs text-base-content/60" title="不在格式注册表内，未收录">
            未收录 {hiddenFiles}
          </span>
        )}

        <div className="join">
          <button
            className={`btn btn-xs join-item ${viewMode === "list" ? "btn-active" : ""}`}
            onClick={() => setViewMode("list")}
          >
            列表
          </button>
          <button
            className={`btn btn-xs join-item ${viewMode === "grid" ? "btn-active" : ""}`}
            onClick={() => setViewMode("grid")}
          >
            网格
          </button>
        </div>

        {root && (
          <button className="btn btn-sm btn-ghost" onClick={() => void reload()} title="重新读取当前目录">
            刷新
          </button>
        )}
      </div>
    </div>
  );
}
