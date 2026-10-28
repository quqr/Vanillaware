import { t } from "../i18n";
import { useEffect, useState } from "react";
import { useLibraryStore } from "../store/libraryStore";
import { PROFILE_PRESETS, useAppStore } from "../store/appStore";
import { api } from "../lib/ipc";
import { Toolbar } from "../components/library/Toolbar";
import { DirectoryTree } from "../components/library/DirectoryTree";
import { FileList } from "../components/library/FileList";
import { FileGrid } from "../components/library/FileGrid";
import { ViewerPanel } from "../components/library/ViewerPanel";

const TREE_MIN = 160;
const TREE_MAX = 420;
const VIEWER_MIN = 320;
const VIEWER_MAX = 900;

export function LibraryView() {
  const viewMode = useLibraryStore((s) => s.viewMode);
  const root = useLibraryStore((s) => s.root);
  const loadFormats = useLibraryStore((s) => s.loadFormats);
  const profileMode = useAppStore((s) => s.profileMode);
  const setProfile = useAppStore((s) => s.setProfile);

  useEffect(() => {
    void loadFormats();
  }, [loadFormats]);

  useEffect(() => {
    if (!root) return;
    let alive = true;

    if (profileMode === "auto") {
      void api
        .detectProfile(root)
        .then((profile) => {
          if (alive) setProfile(profile);
        })
        .catch(() => undefined);
    } else {
      setProfile(PROFILE_PRESETS[profileMode]);
    }

    return () => {
      alive = false;
    };
  }, [root, profileMode, setProfile]);

  const [treeOpen, setTreeOpen] = useState(true);
  const [viewerOpen, setViewerOpen] = useState(true);
  const [treeWidth, setTreeWidth] = useState(230);
  const [viewerWidth, setViewerWidth] = useState(460);

  return (
    <div className="flex h-full flex-col">
      <Toolbar
        treeOpen={treeOpen}
        viewerOpen={viewerOpen}
        onToggleTree={() => setTreeOpen((v) => !v)}
        onToggleViewer={() => setViewerOpen((v) => !v)}
      />

      <div className="flex min-h-0 flex-1">
        {treeOpen ? (
          <>
            <aside
              className="flex shrink-0 flex-col overflow-hidden border-r border-base-300 bg-base-200"
              style={{ width: treeWidth }}
            >
              <DirectoryTree />
            </aside>
            <Divider onDelta={(dx) => setTreeWidth((w) => clamp(w + dx, TREE_MIN, TREE_MAX))} />
          </>
        ) : (
          <CollapsedStrip label={t("library.directory")} onClick={() => setTreeOpen(true)} />
        )}

        <section className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-base-100">
          {viewMode === "list" ? <FileList /> : <FileGrid />}
        </section>

        {viewerOpen ? (
          <>
            <Divider onDelta={(dx) => setViewerWidth((w) => clamp(w - dx, VIEWER_MIN, VIEWER_MAX))} />
            <aside
              className="flex shrink-0 flex-col overflow-hidden border-l border-base-300 bg-base-200"
              style={{ width: viewerWidth }}
            >
              <ViewerPanel />
            </aside>
          </>
        ) : (
          <CollapsedStrip label={t("library.viewer")} onClick={() => setViewerOpen(true)} />
        )}
      </div>
    </div>
  );
}

function Divider({ onDelta }: { onDelta: (dx: number) => void }) {
  return (
    <div
      className="w-1 shrink-0 cursor-col-resize bg-transparent transition-colors hover:bg-primary/40"
      onPointerDown={(event) => {
        event.currentTarget.setPointerCapture(event.pointerId);
      }}
      onPointerMove={(event) => {
        if (event.currentTarget.hasPointerCapture(event.pointerId)) onDelta(event.movementX);
      }}
      onPointerUp={(event) => {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }}
    />
  );
}

function CollapsedStrip({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <button
      className="btn btn-xs h-full w-7 shrink-0 rounded-none px-0 tracking-widest [writing-mode:vertical-rl]"
      onClick={onClick}
      title={t("library.expand", { label })}
    >
      {label}
    </button>
  );
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
