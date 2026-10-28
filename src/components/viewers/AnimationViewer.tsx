import { useState } from "react";
import type { FileEntry } from "../../lib/ipc";
import { ScenePlayer } from "./ScenePlayer";
import { StructurePanel } from "./StructurePanel";

type Mode = "play" | "structure";

export function AnimationViewer({ entry, tag }: { entry: FileEntry; tag: string }) {
  const [mode, setMode] = useState<Mode>("play");

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div role="tablist" className="tabs tabs-border tabs-xs flex-none px-2">
        <button
          role="tab"
          className={`tab ${mode === "play" ? "tab-active" : ""}`}
          onClick={() => setMode("play")}
        >
          播放
        </button>
        <button
          role="tab"
          className={`tab ${mode === "structure" ? "tab-active" : ""}`}
          onClick={() => setMode("structure")}
        >
          结构
        </button>
      </div>

      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        {mode === "play" ? <ScenePlayer path={entry.path} tag={tag} /> : <StructurePanel path={entry.path} tag={tag} />}
      </div>
    </div>
  );
}
