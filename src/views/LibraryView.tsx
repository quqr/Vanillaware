import { useEffect, useState } from "react";

import { useLibraryStore } from "../store/libraryStore";
import { PROFILE_PRESETS, useAppStore } from "../store/appStore";
import { api } from "../lib/ipc";
import { Toolbar } from "../components/library/Toolbar";
import { ViewerPanel } from "../components/library/ViewerPanel";
import { ColumnView } from "../components/columns/ColumnView";

const VIEWER_MIN = 320;
const VIEWER_MAX = 900;

export function LibraryView() {
  const root = useLibraryStore((s) => s.root);
  const rootIsArchive = useLibraryStore((s) => s.rootIsArchive);
  const loadFormats = useLibraryStore((s) => s.loadFormats);
  const profileMode = useAppStore((s) => s.profileMode);
  const setProfile = useAppStore((s) => s.setProfile);

  useEffect(() => {
    void loadFormats();
  }, [loadFormats]);

  useEffect(() => {
    if (!root || rootIsArchive) return;
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
  }, [root, rootIsArchive, profileMode, setProfile]);

  const [viewerOpen, setViewerOpen] = useState(true);
  const [viewerWidth, setViewerWidth] = useState(440);

  const clamp = (value: number) => Math.min(Math.max(value, VIEWER_MIN), VIEWER_MAX);

  return (
    <div className="flex h-full flex-col">
      <Toolbar viewerOpen={viewerOpen} onToggleViewer={() => setViewerOpen((open) => !open)} />

      <div className="flex min-h-0 flex-1 flex-row">
        <ColumnView />

        {viewerOpen && <Divider onDelta={(dx) => setViewerWidth((width) => clamp(width - dx))} />}

        {viewerOpen && (
          <aside
            className="flex min-h-0 flex-none flex-col overflow-hidden border-l border-base-300 bg-base-200"
            style={{ width: viewerWidth }}
          >
            <ViewerPanel />
          </aside>
        )}
      </div>
    </div>
  );
}

function Divider({ onDelta }: { onDelta: (dx: number) => void }) {
  return (
    <div
      className="w-1 flex-none cursor-col-resize bg-transparent transition-colors hover:bg-primary/40"
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
