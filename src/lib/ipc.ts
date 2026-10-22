/** 后端 DTO 的 TypeScript 镜像与 invoke 封装。 */

import { invoke } from "@tauri-apps/api/core";

// ─── 库：目录与文件 ─────────────────────────────────

/** 资源类别（粗分类，只用于图标与分组）。 */
export type Kind =
  | "texture"
  | "animation"
  | "scene"
  | "model"
  | "script"
  | "text"
  | "list"
  | "grid"
  | "binary";

/** 已知游戏档案。 */
export type GameId = "unicorn_overlord" | "thirteen_sentinels" | "unknown";

/** 游戏档案嗅探结果。 */
export interface GameProfile {
  id: GameId;
  label: string;
  /** 默认 mbs 平台标签；null 表示不改动当前选择。 */
  mbsTag: string | null;
  confidence: number;
  evidence: string[];
}

/** 注册表里的一条格式声明。 */
export interface FormatInfo {
  ext: string;
  kind: Kind;
  kindLabel: string;
  label: string;
  icon: string;
  viewers: string[];
  hasThumb: boolean;
}

/** 一个子目录。 */
export interface DirEntry {
  path: string;
  name: string;
}

/** 一个已收录的文件条目。 */
export interface FileEntry {
  path: string;
  name: string;
  /** 小写、不含点。 */
  ext: string;
  size: number;
  mtimeMs: number;
  kind: Kind;
  /** 给人看的格式名，如 `FTEX`。 */
  label: string;
  /** 图标语义名。 */
  icon: string;
  /** 可用查看器 id，按展示顺序。 */
  viewers: string[];
  hasThumb: boolean;
}

export interface DirListing {
  path: string;
  parent: string | null;
  dirs: DirEntry[];
  files: FileEntry[];
  /** 因不在格式注册表里而被过滤掉的文件数。 */
  hiddenFiles: number;
  warnings: string[];
}

export interface RelatedFile {
  path: string;
  name: string;
  ext: string;
  kind: Kind;
  label: string;
}

export interface FileMeta {
  entry: FileEntry;
  related: RelatedFile[];
}

// ─── 文档树 ─────────────────────────────────────────

export type DocNodeKind = "root" | "struct" | "array" | "field" | "bytes" | "text" | "opaque";
export type DocMark = "note" | "warn" | "error";

export interface DocNode {
  id: number;
  parent: number | null;
  key: string | null;
  name: string;
  kind: DocNodeKind;
  value: string | null;
  offset: number | null;
  len: number | null;
  mark: DocMark | null;
  childCount: number;
}

export interface DocCheck {
  id: string;
  label: string;
  ok: boolean;
  detail: string | null;
}

export interface Doc {
  handle: number;
  path: string;
  ext: string;
  label: string;
  size: number;
  totalNodes: number;
  buildMs: number;
  complete: boolean;
  truncated: boolean;
  checks: DocCheck[];
  warnings: string[];
  root: number;
}

export interface DocPage {
  handle: number;
  node: number;
  offset: number;
  total: number;
  items: DocNode[];
}

export interface DocOpen {
  doc: Doc;
  page: DocPage;
}

// ─── 索引 ───────────────────────────────────────────

export interface IndexedFile {
  path: string;
  name: string;
  ext: string;
  kind: Kind;
  label: string;
  icon: string;
  viewers: string[];
}

export interface SearchResult {
  items: IndexedFile[];
  truncated: boolean;
  total: number;
}

export type IndexState = "idle" | "building" | "ready" | "failed";

export interface IndexStatus {
  state: IndexState;
  count: number;
  root: string | null;
  error: string | null;
}

// ─── 纹理帧 ─────────────────────────────────────────

export interface FrameMeta {
  index: number;
  width: number;
  height: number;
  format: number;
  formatName: string;
  channels: number;
}

export interface FrameList {
  path: string;
  count: number;
  frames: FrameMeta[];
}

export interface CacheInfo {
  root: string;
  entries: number;
  bytes: number;
  hits: number;
  misses: number;
}

// ─── 预览场景 ───────────────────────────────────────

/** 图集一帧（预览用，不含像素）。 */
export interface AtlasFrame {
  texId: number;
  width: number;
  height: number;
  format: number;
  formatName: string;
  channels: number;
}

export interface SceneLayer {
  flags: string;
  dst: number[];
  src: number[] | null;
  texId: number | null;
  blendId: number;
  fog: string[] | null;
  colorize: string | null;
  attribute: string[] | null;
}

export interface SceneKeyframe {
  name: string;
  flags: string;
  layers: (SceneLayer | null)[];
}

export interface SceneHitbox {
  name: string;
  layers: ({ flags: string; quad: number[] } | null)[];
}

export interface SceneAttach {
  kind: string;
  id: number;
}

export interface SceneFrame {
  time: number;
  flags: string;
  attach: SceneAttach | null;
  color: string | null;
  matrix: number[];
}

export interface SceneAnimation {
  id: number;
  name: string;
  loopId: number;
  frames: SceneFrame[];
}

export interface SceneSkeleton {
  name: string;
  bones: SceneAttach[];
}

export interface Scene {
  mbsPath: string;
  tag: string;
  atlas: AtlasFrame[];
  keyframe: (SceneKeyframe | null)[];
  hitbox: (SceneHitbox | null)[];
  animations: SceneAnimation[];
  skeletons: SceneSkeleton[];
  selfCheck: string[];
}

export interface ConvertResult {
  mbsPath: string;
  tag: string;
  quad: unknown;
  selfCheck: string[];
}

export const api = {
  listDir: (path: string) => invoke<DirListing>("list_dir", { path }),
  listFormats: () => invoke<FormatInfo[]>("list_formats"),
  detectProfile: (root: string) => invoke<GameProfile>("detect_profile", { root }),
  openDoc: (path: string, tag?: string) => invoke<DocOpen>("open_doc", { path, tag }),
  docChildren: (handle: number, node: number, offset?: number, limit?: number) =>
    invoke<DocPage>("doc_children", { handle, node, offset, limit }),
  fileMeta: (path: string) => invoke<FileMeta>("file_meta", { path }),
  fileThumb: (path: string) => invoke<ArrayBuffer>("file_thumb", { path }),
  readFileHead: (path: string, maxBytes: number) =>
    invoke<ArrayBuffer>("read_file_head", { path, maxBytes }),

  startIndex: (root: string) => invoke<IndexStatus>("start_index", { root }),
  indexStatus: () => invoke<IndexStatus>("index_status"),
  searchFiles: (query: string, limit?: number) =>
    invoke<SearchResult>("search_files", { query, limit }),

  listFrames: (path: string) => invoke<FrameList>("list_frames", { path }),
  getThumbPng: (path: string, index: number) =>
    invoke<ArrayBuffer>("get_thumb_png", { path, index }),
  getFramePng: (path: string, index: number) =>
    invoke<ArrayBuffer>("get_frame_png", { path, index }),

  previewScene: (path: string, tag: string) => invoke<Scene>("preview_scene", { path, tag }),
  getAtlasPng: (path: string, texId: number) =>
    invoke<ArrayBuffer>("get_atlas_png", { path, texId }),
  convertMbs: (path: string, tag: string) => invoke<ConvertResult>("convert_mbs", { path, tag }),
  exportQuad: (path: string, tag: string, outPath: string) =>
    invoke<number>("export_quad", { path, tag, outPath }),

  getCacheInfo: () => invoke<CacheInfo>("get_cache_info"),
  clearThumbCache: () => invoke<CacheInfo>("clear_thumb_cache"),
};
