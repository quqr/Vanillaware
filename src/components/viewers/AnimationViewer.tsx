import { useState } from "react";
import type { FileEntry } from "../../lib/ipc";
import { ScenePlayer } from "./ScenePlayer";
import { StructurePanel } from "./StructurePanel";

type Mode = "play" | "structure";

/** 动画查看器：同一文件的两种看法，不额外占导航位。 */
export function AnimationViewer({ entry, tag }: { entry: FileEntry; tag: string }) {
  const [mode, setMode] = useState<Mode>("play");

  return (
    <div className="animation-viewer">
      <div className="viewer-subtabs">
        <button className="tab" data-active={mode === "play"} onClick={() => setMode("play")}>
          播放
        </button>
        <button className="tab" data-active={mode === "structure"} onClick={() => setMode("structure")}>
          结构
        </button>
      </div>

      <div className="viewer-body">
        {mode === "play" ? <ScenePlayer path={entry.path} tag={tag} /> : <StructurePanel path={entry.path} tag={tag} />}
      </div>
    </div>
  );
}
