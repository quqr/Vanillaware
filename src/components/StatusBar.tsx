import { t } from "../i18n";
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

  const indexTone =
    index.state === "ready"
      ? { text: "text-success", dot: "status-success" }
      : index.state === "failed"
        ? { text: "text-error", dot: "status-error" }
        : { text: "text-warning", dot: "status-warning" };

  return (
    <div className="flex h-[26px] shrink-0 items-center gap-3 overflow-hidden border-t border-base-300 bg-base-200 px-2.5 font-mono text-[11px] text-base-content/60">
      <span className="truncate" title={dir ?? ""}>
        {dir ?? root ?? t("library.noDirectory")}
      </span>
      <span>dirs {dirs.length}</span>
      <span>
        {searching ? t("status.hits") : t("status.assets")} {truncated ? `${rows.length}/${total}` : rows.length}
      </span>
      <span className={`flex items-center gap-1 ${indexTone.text}`}>
        <span className={`status status-xs ${indexTone.dot}`} />
        {t("library.indexCount", { count: index.count })}
      </span>
      <div className="flex-1" />
      <span>tag {tag}</span>
      {warnings.length > 0 && (
        <span className="text-warning" title={warnings.join("\n")}>
          warnings {warnings.length}
        </span>
      )}
    </div>
  );
}
