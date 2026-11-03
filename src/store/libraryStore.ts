import { create } from "zustand";

import { t } from "../i18n";
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
import {
  chainFor,
  columnFrom,
  emptyColumn,
  isArchivePath,
  parentOf,
  step,
  withSelection,
  type Column,
  type Row,
} from "../components/columns/model";

export type SearchScope = "dir" | "all";

interface State {
  root: string | null;
  rootIsArchive: boolean;

  columns: Column[];
  activeColumn: number;

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

  previewPath: string | null;
  previewIsDir: boolean;
  selectedPath: string | null;
  selectedPaths: string[];
  meta: FileMeta | null;
  metaPhase: "idle" | "loading" | "ready" | "error";

  openedPath: string | null;
  openedViewer: string | null;

  columnWidths: number[];

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
  openArchive(): Promise<void>;
  navigate(path: string): Promise<void>;
  back(): void;
  forward(): void;
  up(): Promise<void>;
  reload(): Promise<void>;

  selectRow(index: number, path: string): void;
  selectSibling(delta: number): void;
  enterRow(index: number): void;
  setActiveColumn(index: number): void;

  select(path: string | null): void;
  open(path: string): void;
  unpin(): void;
  close(): void;
  setViewerTab(id: string): void;

  setColumnWidth(index: number, width: number): void;

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
const COLUMN_WIDTH = 240;
const WIDTH_KEY = "vw-column-widths";

let seq = 0;
let searchTimer: ReturnType<typeof setTimeout> | null = null;
let indexTimer: ReturnType<typeof setInterval> | null = null;

function loadWidths(): number[] {
  try {
    const raw = localStorage.getItem(WIDTH_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((value) => typeof value === "number") : [];
  } catch {
    return [];
  }
}

function saveWidths(widths: number[]) {
  try {
    localStorage.setItem(WIDTH_KEY, JSON.stringify(widths.slice(0, 16)));
  } catch {
    /* ignore */
  }
}

const initialState: State = {
  root: null,
  rootIsArchive: false,
  columns: [],
  activeColumn: 0,
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
  previewPath: null,
  previewIsDir: false,
  selectedPath: null,
  selectedPaths: [],
  meta: null,
  metaPhase: "idle",
  openedPath: null,
  openedViewer: null,
  columnWidths: loadWidths(),
  availableFormats: [],
  formatFilter: [],
  query: "",
  scope: "all",
  search: null,
  searchPhase: "idle",
  index: EMPTY_INDEX,
  cacheEpoch: 0,
};

function mirror(columns: Column[]): Partial<State> {
  const last = columns[columns.length - 1];
  if (!last) return {};
  const dirs: DirEntry[] = last.entries
    .filter((row) => row.isDir)
    .map((row) => ({ path: row.path, name: row.name, kind: row.kind }));
  const files: FileEntry[] = last.entries
    .filter((row) => !row.isDir)
    .map((row) => ({
      path: row.path,
      name: row.name,
      ext: row.ext,
      size: row.size,
      mtimeMs: row.mtimeMs,
      kind: row.kind ?? "text",
      label: row.label,
      icon: row.icon,
      viewers: row.viewers,
      hasThumb: row.hasThumb,
    }));

  return {
    dir: last.dir,
    parent: parentOf(last.dir),
    dirs,
    files,
    hiddenFiles: last.hiddenFiles,
    warnings: last.warnings,
    listingPhase: last.phase,
    listingError: last.error,
    activeColumn: columns.length - 1,
  };
}

export const useLibraryStore = create<LibraryState>((set, get) => {
  async function loadColumn(dir: string): Promise<Column> {
    try {
      const listing = await api.listDir(dir);
      return columnFrom(listing);
    } catch (error) {
      return { ...emptyColumn(dir), phase: "error", error: String(error) };
    }
  }

  function selectRowAt(index: number, path: string) {
    const columns = get().columns;
    const column = columns[index];
    if (!column) return;
    const row = column.entries.find((entry) => entry.path === path);
    if (!row) return;

    if (row.isDir) {
      const next = withSelection(columns.slice(0, index + 1), index, path);
      set({
        columns: next,
        previewPath: path,
        previewIsDir: true,
        selectedPath: path,
        selectedPaths: [path],
        meta: null,
        metaPhase: "idle",
        openedPath: null,
        openedViewer: null,
        ...mirror(next),
      });
      void pushColumn(path, index + 1);
      return;
    }

    const next = withSelection(columns, index, path);
    set({
      columns: next,
      previewPath: path,
      previewIsDir: false,
      selectedPath: path,
      selectedPaths: [path],
      meta: null,
      metaPhase: "loading",
      activeColumn: index,
      openedPath: null,
      openedViewer: null,
    });
    void api
      .fileMeta(path)
      .then((meta) => {
        if (get().previewPath !== path) return;
        set({ meta, metaPhase: "ready" });
      })
      .catch(() => {
        if (get().previewPath !== path) return;
        set({ meta: null, metaPhase: "error" });
      });
  }

  async function pushColumn(dir: string, index: number) {
    const token = seq;
    set((prev) => {
      const columns = [...prev.columns.slice(0, index), emptyColumn(dir)];
      return { columns, ...mirror(columns) };
    });
    const column = await loadColumn(dir);
    if (token !== seq) return;
    set((prev) => {
      if (prev.columns.length !== index + 1) return {};
      const columns = [...prev.columns.slice(0, index), column];
      return { columns, ...mirror(columns) };
    });
  }

  async function setChain(chain: string[], selectAt: string | null) {
    const token = ++seq;
    const columns: Column[] = chain.map((dir) => emptyColumn(dir));
    set({
      columns: [...columns],
      previewPath: null,
      previewIsDir: false,
      meta: null,
      metaPhase: "idle",
      openedPath: null,
      openedViewer: null,
      ...mirror(columns),
    });

    for (let index = 0; index < chain.length; index += 1) {
      const column = await loadColumn(chain[index]);
      if (token !== seq) return;
      columns[index] = column;
      const next = chain[index + 1] ?? selectAt;
      if (next) column.selected = next;
      set({ columns: [...columns], ...mirror(columns) });
    }

    if (selectAt) selectRowAt(columns.length - 1, selectAt);
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
          if (status.state !== "building") stopIndexPolling();
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

  async function start(root: string, isArchive: boolean) {
    clearThumbUrls();
    const previous = get();
    set({
      ...initialState,
      root,
      rootIsArchive: isArchive,
      availableFormats: previous.availableFormats,
      formatFilter: previous.formatFilter,
      columnWidths: previous.columnWidths,
      cacheEpoch: previous.cacheEpoch,
      history: [root],
      historyIndex: 0,
    });

    await setChain([root], null);

    try {
      const status = await api.startIndex(root);
      set({ index: status });
      startIndexPolling();
    } catch {
      set({ index: { ...EMPTY_INDEX, state: "failed", error: t("library.indexStartFailed") } });
    }
  }

  return {
    ...initialState,

    async openRoot() {
      let picked: string | string[] | null = null;
      try {
        const { open } = await import("@tauri-apps/plugin-dialog");
        picked = await open({
          directory: true,
          multiple: false,
          title: t("library.pickDirectoryTitle"),
        });
      } catch (error) {
        set({ listingPhase: "error", listingError: String(error) });
        return;
      }
      if (typeof picked !== "string") return;
      await start(picked, false);
    },

    async openArchive() {
      let picked: string | string[] | null = null;
      try {
        const { open } = await import("@tauri-apps/plugin-dialog");
        picked = await open({
          directory: false,
          multiple: false,
          title: t("library.pickArchiveTitle"),
          filters: [{ name: "CPK", extensions: ["cpk", "CPK"] }],
        });
      } catch (error) {
        set({ listingPhase: "error", listingError: String(error) });
        return;
      }
      if (typeof picked !== "string") return;
      await start(picked, true);
    },

    async navigate(path) {
      const root = get().root;
      if (!root) return;
      const chain = chainFor(root, path);
      set((prev) => ({
        history: [...prev.history.slice(0, prev.historyIndex + 1), path],
        historyIndex: prev.historyIndex + 1,
      }));
      await setChain(chain, null);
    },

    back() {
      const { history, historyIndex, root } = get();
      if (!root || historyIndex <= 0) return;
      const index = historyIndex - 1;
      set({ historyIndex: index });
      void setChain(chainFor(root, history[index]), null);
    },

    forward() {
      const { history, historyIndex, root } = get();
      if (!root || historyIndex >= history.length - 1) return;
      const index = historyIndex + 1;
      set({ historyIndex: index });
      void setChain(chainFor(root, history[index]), null);
    },

    async up() {
      const parent = get().parent;
      if (parent) await get().navigate(parent);
    },

    async reload() {
      const { columns } = get();
      if (columns.length === 0) return;
      await setChain(
        columns.map((column) => column.dir),
        columns[columns.length - 1].selected,
      );
    },

    selectRow(index, path) {
      selectRowAt(index, path);
    },

    selectSibling(delta) {
      const { columns, activeColumn } = get();
      const column = columns[activeColumn];
      if (!column) return;
      const next = step(column.entries, column.selected, delta);
      if (next) selectRowAt(activeColumn, next);
    },

    enterRow(index) {
      const { columns } = get();
      const column = columns[index];
      if (!column?.selected) return;
      const row = column.entries.find((entry) => entry.path === column.selected);
      if (!row) return;
      if (row.isDir) {
        selectRowAt(index, row.path);
        return;
      }
      set({ openedPath: row.path, openedViewer: row.viewers[0] ?? null });
    },

    setActiveColumn(index) {
      const { columns } = get();
      if (index < 0 || index >= columns.length) return;
      set({ activeColumn: index });
    },

    select(path) {
      if (!path) {
        set({
          previewPath: null,
          previewIsDir: false,
          selectedPath: null,
          selectedPaths: [],
          meta: null,
          metaPhase: "idle",
        });
        return;
      }
      const { columns } = get();
      const index = columns.findIndex((column) =>
        column.entries.some((entry) => entry.path === path),
      );
      if (index < 0) {
        set({ previewPath: path, selectedPath: path, selectedPaths: [path], metaPhase: "loading" });
        return;
      }
      selectRowAt(index, path);
    },

    open(path) {
      const row: Row | undefined = get()
        .columns.flatMap((column) => column.entries)
        .find((entry) => entry.path === path);
      set({ openedPath: path, openedViewer: row?.viewers[0] ?? null });
      if (get().previewPath !== path) get().select(path);
    },

    unpin() {
      set({ openedPath: null, openedViewer: null });
    },

    close() {
      set({ openedPath: null, openedViewer: null });
    },

    setViewerTab(id) {
      set({ openedViewer: id });
    },

    setColumnWidth(index, width) {
      const widths = [...get().columnWidths];
      widths[index] = width;
      set({ columnWidths: widths });
      saveWidths(widths);
    },

    async loadFormats() {
      if (get().availableFormats.length > 0) return;
      try {
        set({ availableFormats: await api.listFormats() });
      } catch {
        /* ignore */
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

export const COLUMN_WIDTH_DEFAULT = COLUMN_WIDTH;
export { isArchivePath };
