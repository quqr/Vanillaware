import { invoke } from "@tauri-apps/api/core";

export type Kind =
  | "texture"
  | "animation"
  | "scene"
  | "model"
  | "script"
  | "text"
  | "list"
  | "grid"
  | "audio"
  | "video"
  | "archive";

export type GameId = "unicorn_overlord" | "thirteen_sentinels" | "unknown";

export interface GameProfile {
  id: GameId;
  label: string;

  mbsTag: string | null;
  confidence: number;
  evidence: string[];
}

export interface FormatInfo {
  ext: string;
  kind: Kind;
  kindLabel: string;
  label: string;
  icon: string;
  viewers: string[];
  hasThumb: boolean;
}

export interface DirEntry {
  path: string;
  name: string;
  kind?: Kind | null;
}

export interface ArchiveSummary {
  path: string;
  magic: string;
  version: number;
  revision: number;
  align: number;
  fileSize: number;
  entries: number;
  directories: number;
  declaredFiles: number;
  totalExtract: number;
  compressed: number;
  outOfRange: number;
  tocOffset: number;
  contentOffset: number;
  headerEncrypted: boolean;
  tocEncrypted: boolean;
}

export interface ArchiveCacheInfo {
  usedBytes: number;
  budgetBytes: number;
  root: string | null;
}

export interface FileEntry {
  path: string;
  name: string;

  ext: string;
  size: number;
  mtimeMs: number;
  kind: Kind;

  label: string;

  icon: string;

  viewers: string[];
  hasThumb: boolean;
}

export interface DirListing {
  path: string;
  parent: string | null;
  dirs: DirEntry[];
  files: FileEntry[];

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

export interface AsmFunction {
  index: number;
  name: string;
  ret: string;
  params: number;
  bytecode: number;
}

export interface AsmLine {
  index: number;
  op: string;
  args: string[];
  note: string | null;
  target: number | null;
}

export interface Asm {
  functions: AsmFunction[];
  selected: number;
  lines: AsmLine[];
}

export interface Pseudo {
  text: string;
}

export interface BbsObject {
  mbs: number;
  ftx: number;
  kind: number;
  group: number;
  pos: [number, number, number];
  rot: number;
  scale: [number, number];
  color: number;
  id: number;
}

export interface BbsZone {
  id: number;
  quad: [[number, number], [number, number], [number, number], [number, number]];
  dist: number;
}

export interface BbsRoute {
  slot: number;
  local: number;
  pos: [number, number, number];
  kind: number;
  next: number;
}

export interface BbsLayout {
  bbox: [number, number, number, number];
  counts: number[];
  mbsNames: string[];
  ftxNames: string[];
  objects: BbsObject[];
  zones: BbsZone[];
  routes: BbsRoute[];
}

export interface VizReference {
  path: string;
  ext: string;
  role: string;
  status: string;
}

export interface VizSubRange {
  start: number;
  count: number;
  material: number;

  page: number;

  rgba: [number, number, number, number];

  texRef: number;
}

export interface VizMesh {
  positions: number[];
  indices: number[];

  uvs: number[];
  bounds: [number, number, number, number, number, number];
  subRanges: VizSubRange[];
}

export interface VizNode {
  id: number;
  parent: number | null;
  name: string;
  kind: string;
  transform: number[];
  mesh: VizMesh | null;

  meshRef: number | null;
  reference: VizReference | null;
}

export interface VizScene {
  bounds: [number, number, number, number, number, number];
  nodes: VizNode[];
  references: VizReference[];
  truncated: boolean;
}

export interface GraphNode {
  id: number;
  label: string;
  kind: string;
  detail: string | null;
  path: string | null;
  offset: number | null;

  status: string | null;
}

export interface GraphEdge {
  from: number;
  to: number;
  label: string;
}

export interface FileGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
  truncated: boolean;
}

export interface FmsText {
  encoding: number;
  count: number;
  strings: string[];
}

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

export interface AudioTrack {
  index: number;
  name: string;
  cueId: number;
  /** HCA / Opus / ADX … */
  codec: string;

  playable: boolean;

  stream: boolean;
  bytes: number;
}

export interface AudioCodecCount {
  codec: string;
  count: number;
}

export interface AudioBundle {
  container: string;
  source: string;
  tracks: AudioTrack[];
  codecs: AudioCodecCount[];
  external: string[];
  warnings: string[];
}

export interface AudioClip {

  codec: string;
  sampleRate: number;
  channels: number;
  frames: number;
  durationMs: number;

  stride: number;
  skip: number;
  bytes: number;
}

export interface UsmVideo {
  width: number;
  height: number;
  /** avc1.640028 */
  codec: string;
  profile: number;
  level: number;
  fps: number;
  frames: number;
  durationMs: number;

  sps: string;
  pps: string;
  /** adx / hca / none */
  audio: string;
  chunks: number;
  streamBytes: number;
  keyframes: number[];
  frameSizes: number[];
}

export interface ContentSection {
  title: string;
  columns: string[];
  rows: string[][];

  total: number;
}

export interface ContentSeries {
  label: string;
  points: number[];
  note: string | null;
}

export interface Content {
  title: string;
  note: string | null;
  sections: ContentSection[];
  series: ContentSeries | null;
  text: string[] | null;
  /** `data:image/png;base64,...` */
  image: string | null;

  imageNote: string | null;
}

export interface FieldMap {
  name: string;
  group: string;
  files: string[];
  layers: number;
  tiles: number;
  window: [number, number, number, number];
}

export interface DialoguePool {
  path: string | null;
  strings: number;
}

export interface DialogueStats {
  entries: number;
  commands: number;
  textCommands: number;
  uniqueIds: number;
  missingText: number;
}

export interface DialogueLine {
  at: number;
  opcode: number;
  textId: number | null;
  speaker: string | null;
  voice: number | null;
  text: string | null;
}

export interface DialogueEntry {
  index: number;
  name: string;
  group: number;
  link: number;
  kind: number;
  offset: number;
  commands: number;
  lines: DialogueLine[];
}

export interface Dialogue {
  entries: DialogueEntry[];
  stats: DialogueStats;
  pool: DialoguePool;
  warnings: string[];
}

export const api = {
  listDir: (path: string) => invoke<DirListing>("list_dir", { path }),
  archiveSummary: (path: string) => invoke<ArchiveSummary>("archive_summary", { path }),
  archiveExport: (path: string, outPath: string) =>
    invoke<number>("archive_export", { path, outPath }),
  archiveCacheInfo: () => invoke<ArchiveCacheInfo>("archive_cache_info"),
  archiveCacheClear: () => invoke<ArchiveCacheInfo>("archive_cache_clear"),
  listFormats: () => invoke<FormatInfo[]>("list_formats"),
  detectProfile: (root: string) => invoke<GameProfile>("detect_profile", { root }),
  openDoc: (path: string, tag?: string) => invoke<DocOpen>("open_doc", { path, tag }),
  docChildren: (handle: number, node: number, offset?: number, limit?: number) =>
    invoke<DocPage>("doc_children", { handle, node, offset, limit }),
  docAsm: (path: string, fn?: number) => invoke<Asm>("doc_asm", { path, function: fn }),
  docPseudo: (path: string, fn?: number) =>
    invoke<Pseudo>("doc_pseudo", { path, function: fn }),
  docLayout: (path: string) => invoke<BbsLayout>("doc_layout", { path }),
  docText: (path: string) => invoke<FmsText>("doc_text", { path }),
  docMesh: (path: string) => invoke<VizScene>("doc_mesh", { path }),
  docScene: (path: string) => invoke<VizScene>("doc_scene", { path }),
  docGraph: (path: string) => invoke<FileGraph>("doc_graph", { path }),
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

  audioBundle: (path: string) => invoke<AudioBundle>("audio_bundle", { path }),
  audioClipInfo: (path: string, index: number) =>
    invoke<AudioClip>("audio_clip_info", { path, index }),
  audioClipData: (path: string, index: number) =>
    invoke<ArrayBuffer>("audio_clip_data", { path, index }),

  contentOpen: (path: string) => invoke<Content>("content_open", { path }),
  fieldMaps: (path: string) => invoke<FieldMap[]>("field_maps", { path }),
  fieldRender: (
    path: string,
    layer: string | null,
    x0: number | null,
    width: number,
    mode?: string,
  ) => invoke<Content>("field_render", { path, layer, x0, width, mode: mode ?? null }),

  dialogueOpen: (path: string) => invoke<Dialogue>("dialogue_open", { path }),

  usmVideoInfo: (path: string) => invoke<UsmVideo>("usm_video_info", { path }),
  usmVideoGop: (path: string, gop: number) =>
    invoke<ArrayBuffer>("usm_video_gop", { path, gop }),

  getCacheInfo: () => invoke<CacheInfo>("get_cache_info"),
  clearThumbCache: () => invoke<CacheInfo>("clear_thumb_cache"),
};
