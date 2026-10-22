/** 后端 DTO 的 TypeScript 镜像与 invoke 封装。 */

import { invoke } from "@tauri-apps/api/core";

// ─── 库：目录与文件 ─────────────────────────────────

/** 资源类别。只列原生游戏二进制资源的类别。 */
export type Kind = "texture" | "animation" | "binary";

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

// ─── 索引 ───────────────────────────────────────────

export interface IndexedFile {
  path: string;
  name: string;
  kind: Kind;
  label: string;
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
