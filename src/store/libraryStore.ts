import { t } from "../i18n";
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

  openedPath: string | null;
  openedViewer: string | null;

  viewMode: ViewMode;

  availableFormats: FormatInfo[];

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

function parentOf(path: string): string | null {
  const cut = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  if (cut <= 0) return null;
  const parent = path.slice(0, cut);
  return parent.endsWith(":") ? `${parent}\\` : parent;
}

export const useLibraryStore = create<LibraryState>((set, get) => {

  function loadDir(path: string, asTree: boolean): Promise<void> {
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
        picked = await open({ directory: true, multiple: false, title: t("library.pickDirectoryTitle") });
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
        viewMode: previous.viewMode,
        availableFormats: previous.availableFormats,
        formatFilter: previous.formatFilter,
        cacheEpoch: previous.cacheEpoch,
      });

      set({ history: [picked], historyIndex: 0, expanded: { [picked]: true } });
      await loadDir(picked, false);

      try {
        const status = await api.startIndex(picked);
        set({ index: status });
        startIndexPolling();
      } catch {
        set({ index: { ...EMPTY_INDEX, state: "failed", error: t("library.indexStartFailed") } });
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
