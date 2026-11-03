import { t } from "../i18n";
import { useLibraryStore } from "../store/libraryStore";
import { useAppStore } from "../store/appStore";

export function StatusBar() {
  const root = useLibraryStore((s) => s.root);
  const dir = useLibraryStore((s) => s.dir);
  const columns = useLibraryStore((s) => s.columns);
  const dirs = useLibraryStore((s) => s.dirs);
  const files = useLibraryStore((s) => s.files);
  const warnings = useLibraryStore((s) => s.warnings);
  const index = useLibraryStore((s) => s.index);
  const query = useLibraryStore((s) => s.query);
  const scope = useLibraryStore((s) => s.scope);
  const search = useLibraryStore((s) => s.search);
  const tag = useAppStore((s) => s.tag);

  const searching = scope === "all" && query.trim().length > 0 && search !== null;
  const shown = searching ? search.items.length : dirs.length + files.length;
  const total = searching ? search.total : shown;
  const truncated = searching && search.truncated;

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
      <span>cols {columns.length}</span>
      <span>dirs {dirs.length}</span>
      <span>
        {searching ? t("status.hits") : t("status.assets")}{" "}
        {truncated ? `${shown}/${total}` : shown}
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
