/** 库状态：目录列表、选中/打开、过滤与索引。
 *
 * 列表永远来自后端实时 `list_dir`；索引只服务「全部目录」搜索，不冒充列表。
 * 目录树与当前目录共用同一份缓存：展开时读的那层，正是之后导航用的那层。
 */

import { create } from "zustand";
import {
  api,
  type DirEntry,
  type FileEntry,
  type FileMeta,
  type FormatInfo,
  type IndexStatus,
  type SearchResult,
} from "../lib/ipc";
import { clearThumbUrls } from "../lib/thumbCache";

export type ViewMode = "list" | "grid";
export type SearchScope = "dir" | "all";

interface TreeNode {
  dirs: DirEntry[];
  phase: "loading" | "ready" | "error";
}

interface State {
  root: string | null;
  dir: string | null;
  parent: string | null;
  dirs: DirEntry[];
  files: FileEntry[];
  hiddenFiles: number;
  warnings: string[];
  listingPhase: "idle" | "loading" | "ready" | "error";
  listingError: string | null;

  history: string[];
  historyIndex: number;

  tree: Record<string, TreeNode>;
  expanded: Record<string, boolean>;

  selectedPath: string | null;
  selectedPaths: string[];
  meta: FileMeta | null;
  metaPhase: "idle" | "loading" | "ready" | "error";

  /** 双击/Enter 打开的文件；只有它有查看器内容。 */
  openedPath: string | null;
  openedViewer: string | null;

  viewMode: ViewMode;
  /** 注册表快照（工具栏过滤项）。 */
  availableFormats: FormatInfo[];
  /** 选中的格式扩展名；空数组 = 全部。 */
  formatFilter: string[];
  query: string;
  scope: SearchScope;
  search: SearchResult | null;
  searchPhase: "idle" | "loading" | "ready" | "error";
  index: IndexStatus;

  cacheEpoch: number;
}

interface Actions {
  openRoot(): Promise<void>;
  navigate(path: string): Promise<void>;
  back(): void;
  forward(): void;
  up(): Promise<void>;
  reload(): Promise<void>;
  toggleExpanded(path: string): void;
  collapseAll(keep: string): void;

  select(path: string | null, additive?: boolean): void;
  open(path: string): void;
  close(): void;
  setViewerTab(id: string): void;

  setViewMode(mode: ViewMode): void;
  loadFormats(): Promise<void>;
  toggleFormat(ext: string): void;
  clearFormats(): void;
  setQuery(query: string): void;
  setScope(scope: SearchScope): void;
  refreshIndex(): void;
  refreshCache(): Promise<void>;
}

export type LibraryState = State & Actions;

const EMPTY_INDEX: IndexStatus = { state: "idle", count: 0, root: null, error: null };
const SEARCH_DEBOUNCE = 180;
const INDEX_POLL = 400;

/** 列表请求序号：只有最后一次请求能写回当前列表，避免旧响应覆盖新目录。 */
let listingSeq = 0;
let searchTimer: ReturnType<typeof setTimeout> | null = null;
let indexTimer: ReturnType<typeof setInterval> | null = null;

const initialState: State = {
  root: null,
  dir: null,
  parent: null,
  dirs: [],
  files: [],
  hiddenFiles: 0,
  warnings: [],
  listingPhase: "idle",
  listingError: null,
  history: [],
  historyIndex: -1,
  tree: {},
  expanded: {},
  selectedPath: null,
  selectedPaths: [],
  meta: null,
  metaPhase: "idle",
  openedPath: null,
  openedViewer: null,
  viewMode: "list",
  availableFormats: [],
  formatFilter: [],
  query: "",
  scope: "dir",
  search: null,
  searchPhase: "idle",
  index: EMPTY_INDEX,
  cacheEpoch: 0,
};

/** 取路径的上一级（兼容 `\` 与 `/`）。 */
function parentOf(path: string): string | null {
  const cut = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  if (cut <= 0) return null;
  const parent = path.slice(0, cut);
  // `C:\` 这类根：保留盘符。
  return parent.endsWith(":") ? `${parent}\\` : parent;
}

export const useLibraryStore = create<LibraryState>((set, get) => {
  /** 读一层目录；`asTree` 时只更新树节点，否则同时作为当前列表。 */
  function loadDir(path: string, asTree: boolean): Promise<void> {
    // 只有「当前列表」请求参与序号竞争；树展开请求不能推进序号，
    // 否则祖先链的树加载会把当前目录的列表响应判成过期，永远停在 loading。
    const seq = asTree ? listingSeq : ++listingSeq;

    if (asTree) {
      set((prev) => ({
        tree: { ...prev.tree, [path]: { dirs: prev.tree[path]?.dirs ?? [], phase: "loading" } },
      }));
    } else {
      set({ listingPhase: "loading", listingError: null });
    }

    return (async () => {
      try {
        const listing = await api.listDir(path);
        set((prev) => {
          const next: Partial<State> = {
            tree: { ...prev.tree, [listing.path]: { dirs: listing.dirs, phase: "ready" } },
          };
          // 只有最后一次列表请求、且路径仍是当前目录时才写列表，
          // 否则旧目录的响应会把新目录覆盖掉。
          if (!asTree && seq === listingSeq) {
            next.dir = listing.path;
            next.parent = listing.parent;
            next.dirs = listing.dirs;
            next.files = listing.files;
            next.hiddenFiles = listing.hiddenFiles;
            next.warnings = listing.warnings;
            next.listingPhase = "ready";
            next.listingError = null;
          }
          return next;
        });
      } catch (e) {
        set((prev) => {
          if (asTree) {
            return {
              tree: {
                ...prev.tree,
                [path]: { dirs: prev.tree[path]?.dirs ?? [], phase: "error" },
              },
            };
          }
          if (seq !== listingSeq) return {};
          return { listingPhase: "error", listingError: String(e) };
        });
      }
    })();
  }

  async function loadMeta(path: string) {
    set({ metaPhase: "loading" });
    try {
      const meta = await api.fileMeta(path);
      if (get().selectedPath !== path) return;
      set({ meta, metaPhase: "ready" });
    } catch {
      if (get().selectedPath !== path) return;
      set({ meta: null, metaPhase: "error" });
    }
  }

  function navigateTo(path: string, pushHistory: boolean) {
    set((prev) => ({
      dir: path,
      files: [],
      dirs: [],
      listingPhase: "loading",
      listingError: null,
      history: pushHistory
        ? [...prev.history.slice(0, prev.historyIndex + 1), path]
        : prev.history,
      historyIndex: pushHistory ? prev.historyIndex + 1 : prev.historyIndex,
      selectedPath: null,
      selectedPaths: [],
      meta: null,
      metaPhase: "idle",
      openedPath: null,
      openedViewer: null,
    }));

    void loadDir(path, false);

    // 目录树的祖先链要展开，否则当前目录在树里不可见。
    const root = get().root;
    let cursor: string | null = path;
    while (cursor && root && cursor.toLowerCase() !== root.toLowerCase()) {
      const current: string = cursor;
      if (!get().expanded[current]) {
        set((prev) => ({ expanded: { ...prev.expanded, [current]: true } }));
      }
      const parent = parentOf(current);
      if (!parent || parent === current) break;
      if (!get().tree[parent]) void loadDir(parent, true);
      cursor = parent;
    }
  }

  function stopIndexPolling() {
    if (indexTimer) clearInterval(indexTimer);
    indexTimer = null;
  }

  function startIndexPolling() {
    stopIndexPolling();
    indexTimer = setInterval(() => {
      void api
        .indexStatus()
        .then((status) => {
          set({ index: status });
          if (status.state === "ready" || status.state === "failed" || status.state === "idle") {
            stopIndexPolling();
          }
        })
        .catch(stopIndexPolling);
    }, INDEX_POLL);
  }

  function runSearch() {
    if (searchTimer) {
      clearTimeout(searchTimer);
      searchTimer = null;
    }
    const { query, scope } = get();
    const trimmed = query.trim();

    if (scope !== "all" || trimmed.length === 0) {
      set({ search: null, searchPhase: "idle" });
      return;
    }

    set({ searchPhase: "loading" });
    searchTimer = setTimeout(() => {
      void api
        .searchFiles(trimmed)
        .then((result) => {
          // 期间查询或作用域变了就不覆盖。
          if (get().query.trim() !== trimmed || get().scope !== "all") return;
          set({ search: result, searchPhase: "ready" });
        })
        .catch(() => set({ search: null, searchPhase: "error" }));
    }, SEARCH_DEBOUNCE);
  }

  return {
    ...initialState,

    async openRoot() {
      let picked: string | string[] | null = null;
      try {
        const { open } = await import("@tauri-apps/plugin-dialog");
        picked = await open({ directory: true, multiple: false, title: "选择资源目录" });
      } catch (e) {
        set({ listingPhase: "error", listingError: String(e) });
        return;
      }
      if (typeof picked !== "string") return;

      clearThumbUrls();
      const previous = get();
      set({
        ...initialState,
        root: picked,
        // 保留显示偏好与缓存纪元。
        viewMode: previous.viewMode,
        availableFormats: previous.availableFormats,
        formatFilter: previous.formatFilter,
        cacheEpoch: previous.cacheEpoch,
      });

      set({ history: [picked], historyIndex: 0, expanded: { [picked]: true } });
      // 根目录只走一次列表请求；失败时必须落到 error，不能停在 loading。
      await loadDir(picked, false);

      try {
        const status = await api.startIndex(picked);
        set({ index: status });
        startIndexPolling();
      } catch {
        set({ index: { ...EMPTY_INDEX, state: "failed", error: "索引启动失败" } });
      }
    },

    async navigate(path) {
      const same = get().dir?.toLowerCase() === path.toLowerCase();
      if (same && get().listingPhase !== "error") return;
      navigateTo(path, true);
    },

    back() {
      const { history, historyIndex } = get();
      if (historyIndex <= 0) return;
      const index = historyIndex - 1;
      set({ historyIndex: index });
      navigateTo(history[index], false);
    },

    forward() {
      const { history, historyIndex } = get();
      if (historyIndex >= history.length - 1) return;
      const index = historyIndex + 1;
      set({ historyIndex: index });
      navigateTo(history[index], false);
    },

    async up() {
      const parent = get().parent;
      if (parent) await get().navigate(parent);
    },

    async reload() {
      const { dir } = get();
      if (dir) await loadDir(dir, false);
    },

    toggleExpanded(path) {
      const expanded = { ...get().expanded, [path]: !get().expanded[path] };
      set({ expanded });
      if (expanded[path] && !get().tree[path]) void loadDir(path, true);
    },

    collapseAll(keep) {
      const expanded: Record<string, boolean> = {};
      for (const key of Object.keys(get().expanded)) {
        if (key.toLowerCase() === keep.toLowerCase()) expanded[key] = true;
      }
      set({ expanded });
    },

    select(path, additive = false) {
      if (!path) {
        set({ selectedPath: null, selectedPaths: [], meta: null, metaPhase: "idle" });
        return;
      }

      // 同一文件的 meta 已经取过：只改选中，不再触发一次往返（避免查看器闪一下）。
      // 打开态必须一起写：`close()` / `navigate()` 之后重选同一文件也要能渲染内容。
      if (!additive && get().meta?.entry.path === path && get().metaPhase === "ready") {
        const entry = get().meta?.entry;
        set({
          selectedPath: path,
          selectedPaths: [path],
          openedPath: path,
          openedViewer: entry?.viewers[0] ?? get().openedViewer,
        });
        return;
      }

      const selectedPaths = additive
        ? get().selectedPaths.includes(path)
          ? get().selectedPaths.filter((p) => p !== path)
          : [...get().selectedPaths, path]
        : [path];

      const entry =
        get().files.find((file) => file.path === path) ??
        get().search?.items.find((item) => item.path === path);

      set({
        selectedPath: path,
        selectedPaths,
        meta: null,
        metaPhase: "loading",
        // 单击即打开主查看器：属性与内容一起到，不需要再双击一次。
        openedPath: path,
        openedViewer: entry?.viewers[0] ?? get().openedViewer,
      });
      void loadMeta(path);
    },

    open(path) {
      const entry =
        get().files.find((f) => f.path === path) ??
        get().search?.items.find((f) => f.path === path) ??
        (get().meta?.entry.path === path ? get().meta?.entry : undefined);
      if (!entry) return;

      if (get().selectedPath !== path) get().select(path);
      set({ openedPath: path, openedViewer: entry.viewers[0] ?? null });
    },

    close() {
      set({ openedPath: null, openedViewer: null });
    },

    setViewerTab(id) {
      set({ openedViewer: id });
    },

    setViewMode(viewMode) {
      set({ viewMode });
    },

    async loadFormats() {
      if (get().availableFormats.length > 0) return;
      try {
        set({ availableFormats: await api.listFormats() });
      } catch {
        // 注册表加载失败时保持空列表，过滤退化为「全部」。
      }
    },

    toggleFormat(ext) {
      const formatFilter = get().formatFilter.includes(ext)
        ? get().formatFilter.filter((item) => item !== ext)
        : [...get().formatFilter, ext];
      set({ formatFilter });
    },

    clearFormats() {
      set({ formatFilter: [] });
    },

    setQuery(query) {
      set({ query });
      runSearch();
    },

    setScope(scope) {
      set({ scope });
      runSearch();
    },

    refreshIndex() {
      const { root } = get();
      if (!root) return;
      set({ index: { ...EMPTY_INDEX, state: "building", root } });
      void api.startIndex(root).then((status) => {
        set({ index: status });
        startIndexPolling();
      });
    },

    async refreshCache() {
      await api.clearThumbCache();
      clearThumbUrls();
      set((prev) => ({ cacheEpoch: prev.cacheEpoch + 1 }));
    },
  };
});
