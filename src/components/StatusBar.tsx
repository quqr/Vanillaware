import { useLibraryStore } from "../store/libraryStore";
import { useAppStore } from "../store/appStore";
import { useRows } from "./library/rows";

export function StatusBar() {
  const root = useLibraryStore((s) => s.root);
  const dir = useLibraryStore((s) => s.dir);
  const dirs = useLibraryStore((s) => s.dirs);
  const warnings = useLibraryStore((s) => s.warnings);
  const index = useLibraryStore((s) => s.index);
  const tag = useAppStore((s) => s.tag);
  const { rows, searching, total, truncated } = useRows();

  return (
    <div className="statusbar">
      <span title={dir ?? ""}>{dir ?? root ?? "未打开目录"}</span>
      <span>dirs {dirs.length}</span>
      <span>
        {searching ? "命中" : "资源"} {truncated ? `${rows.length}/${total}` : rows.length}
      </span>
      <span className={index.state === "ready" ? "ok" : index.state === "failed" ? "err" : "warn"}>
        索引 {index.count}
      </span>
      <div className="spacer" />
      <span>tag {tag}</span>
      {warnings.length > 0 && (
        <span className="warn" title={warnings.join("\n")}>
          warnings {warnings.length}
        </span>
      )}
    </div>
  );
}
