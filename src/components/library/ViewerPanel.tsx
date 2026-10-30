import { t, type MsgKey } from "../../i18n";
import { useLibraryStore } from "../../store/libraryStore";
import { TAGS, useAppStore, type TagId } from "../../store/appStore";
import { formatBytes } from "../../lib/format";
import type { FileEntry } from "../../lib/ipc";
import { AsmViewer } from "../viewers/AsmViewer";
import { AtlasViewer } from "../viewers/AtlasViewer";
import { AudioViewer } from "../viewers/AudioViewer";
import { BbsLayoutViewer } from "../viewers/BbsLayoutViewer";
import { AnimationViewer } from "../viewers/AnimationViewer";
import { ContentViewer } from "../viewers/ContentViewer";
import { DialogueViewer } from "../viewers/DialogueViewer";
import { DocViewer } from "../viewers/DocViewer";
import { FmdMeshViewer } from "../viewers/FmdMeshViewer";
import { FmsTableViewer } from "../viewers/FmsTableViewer";
import { HexViewer } from "../viewers/HexViewer";
import { OsbSceneViewer } from "../viewers/OsbSceneViewer";
import { RefGraphViewer } from "../viewers/RefGraphViewer";
import { VideoViewer } from "../viewers/VideoViewer";

const VIEWERS: Record<
  string,
  { labelKey: string; render: (entry: FileEntry, tag: string) => React.ReactNode }
> = {
  asm: { labelKey: "viewer.asm", render: (entry) => <AsmViewer entry={entry} /> },
  atlas: { labelKey: "viewer.atlas", render: (entry) => <AtlasViewer entry={entry} /> },
  animation: { labelKey: "viewer.animation", render: (entry, tag) => <AnimationViewer entry={entry} tag={tag} /> },
  audio: { labelKey: "viewer.audio", render: (entry) => <AudioViewer entry={entry} /> },
  content: { labelKey: "viewer.content", render: (entry) => <ContentViewer entry={entry} /> },
  dialogue: { labelKey: "viewer.dialogue", render: (entry) => <DialogueViewer entry={entry} /> },
  doc: { labelKey: "viewer.doc", render: (entry, tag) => <DocViewer entry={entry} tag={tag} /> },
  layout: { labelKey: "viewer.layout", render: (entry) => <BbsLayoutViewer entry={entry} /> },
  mesh: { labelKey: "viewer.mesh", render: (entry) => <FmdMeshViewer entry={entry} /> },
  scene: { labelKey: "viewer.scene", render: (entry) => <OsbSceneViewer entry={entry} /> },
  graph: { labelKey: "viewer.graph", render: (entry) => <RefGraphViewer entry={entry} /> },
  text: { labelKey: "viewer.text", render: (entry) => <FmsTableViewer entry={entry} /> },
  hex: { labelKey: "viewer.hex", render: (entry) => <HexViewer entry={entry} /> },
  video: { labelKey: "viewer.video", render: (entry) => <VideoViewer entry={entry} /> },
};

const KIND_LABEL: Record<string, MsgKey> = {
  texture: "kind.texture",
  animation: "kind.animation",
  scene: "kind.scene",
  model: "kind.model",
  script: "kind.script",
  text: "kind.text",
  list: "kind.list",
  grid: "kind.grid",
  audio: "kind.audio",
  video: "kind.video",
  binary: "kind.binary",
};

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
        {t("viewer.noSelection")}
      </aside>
    );
  }

  const entry = meta?.entry ?? null;
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
            <span className="badge badge-sm">{entry.kind ? t(KIND_LABEL[entry.kind] ?? "kind.binary") : entry.kind}</span>
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
            title={t("viewer.tagHint")}
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
          <span className="text-base-content/50">{t("viewer.related")}</span>
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
          {t("viewer.readingMeta")}
        </div>
      )}

      {metaPhase === "error" && (
        <div className="m-2 alert alert-error alert-soft text-xs">
          {t("viewer.metaFailed")}
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
                {VIEWERS[id] ? t(VIEWERS[id].labelKey as MsgKey) : id}
              </button>
            ))}
          </div>

          <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
            {(VIEWERS[activeViewer ?? ""] ?? VIEWERS.hex).render(entry, tag)}
            {activeViewer && !VIEWERS[activeViewer] && (
              <div className="p-2 font-mono text-xs text-base-content/60">
                {t("viewer.unknownViewer", { id: activeViewer })}
              </div>
            )}
          </div>
        </>
      )}

      {metaPhase === "ready" && entry && openedPath !== entry.path && (
        <div className="flex flex-1 items-center justify-center p-4 text-xs text-base-content/50">
          {t("viewer.doubleClickHint")}
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
