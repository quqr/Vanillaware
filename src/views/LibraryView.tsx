import { useState } from "react";
import { useLibraryStore } from "../store/libraryStore";
import { Toolbar } from "../components/library/Toolbar";
import { DirectoryTree } from "../components/library/DirectoryTree";
import { FileList } from "../components/library/FileList";
import { FileGrid } from "../components/library/FileGrid";
import { ViewerPanel } from "../components/library/ViewerPanel";

const TREE_MIN = 160;
const TREE_MAX = 420;
const VIEWER_MIN = 320;
const VIEWER_MAX = 900;

/** 库视图：目录树 | 文件列表/网格 | 查看器，三栏可拖拽（ADR-0012）。 */
export function LibraryView() {
  const viewMode = useLibraryStore((s) => s.viewMode);

  const [treeOpen, setTreeOpen] = useState(true);
  const [viewerOpen, setViewerOpen] = useState(true);
  const [treeWidth, setTreeWidth] = useState(230);
  const [viewerWidth, setViewerWidth] = useState(460);

  return (
    <div className="library">
      <Toolbar
        treeOpen={treeOpen}
        viewerOpen={viewerOpen}
        onToggleTree={() => setTreeOpen((v) => !v)}
        onToggleViewer={() => setViewerOpen((v) => !v)}
      />

      <div className="library-body">
        {treeOpen ? (
          <>
            <aside className="pane pane-tree" style={{ width: treeWidth }}>
              <DirectoryTree />
            </aside>
            <Divider onDelta={(dx) => setTreeWidth((w) => clamp(w + dx, TREE_MIN, TREE_MAX))} />
          </>
        ) : (
          <CollapsedStrip label="目录" onClick={() => setTreeOpen(true)} />
        )}

        <section className="pane pane-files">
          {viewMode === "list" ? <FileList /> : <FileGrid />}
        </section>

        {viewerOpen ? (
          <>
            <Divider onDelta={(dx) => setViewerWidth((w) => clamp(w - dx, VIEWER_MIN, VIEWER_MAX))} />
            <aside className="pane pane-viewer" style={{ width: viewerWidth }}>
              <ViewerPanel />
            </aside>
          </>
        ) : (
          <CollapsedStrip label="查看器" onClick={() => setViewerOpen(true)} />
        )}
      </div>
    </div>
  );
}

function Divider({ onDelta }: { onDelta: (dx: number) => void }) {
  return (
    <div
      className="divider"
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
    <button className="pane-collapsed" onClick={onClick} title={`展开${label}`}>
      {label}
    </button>
  );
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
