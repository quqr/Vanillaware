import { t } from "../../i18n";
import { useLibraryStore } from "../../store/libraryStore";
import { PROFILE_PRESETS, THEMES, useAppStore, type ProfileMode } from "../../store/appStore";
import { Breadcrumbs } from "./Breadcrumbs";

const BAR = "flex flex-wrap items-center gap-2 border-b border-base-300 bg-base-200 px-2 py-1.5";

export function Toolbar({
  viewerOpen,
  onToggleViewer,
}: {
  viewerOpen: boolean;
  onToggleViewer: () => void;
}) {
  const root = useLibraryStore((s) => s.root);
  const openRoot = useLibraryStore((s) => s.openRoot);
  const openArchive = useLibraryStore((s) => s.openArchive);
  const back = useLibraryStore((s) => s.back);
  const forward = useLibraryStore((s) => s.forward);
  const up = useLibraryStore((s) => s.up);
  const reload = useLibraryStore((s) => s.reload);
  const canBack = useLibraryStore((s) => s.historyIndex > 0);
  const canForward = useLibraryStore((s) => s.historyIndex < s.history.length - 1);

  const query = useLibraryStore((s) => s.query);
  const setQuery = useLibraryStore((s) => s.setQuery);
  const scope = useLibraryStore((s) => s.scope);
  const setScope = useLibraryStore((s) => s.setScope);
  const availableFormats = useLibraryStore((s) => s.availableFormats);
  const formatFilter = useLibraryStore((s) => s.formatFilter);
  const toggleFormat = useLibraryStore((s) => s.toggleFormat);
  const clearFormats = useLibraryStore((s) => s.clearFormats);

  const hiddenFiles = useLibraryStore((s) => s.hiddenFiles);
  const index = useLibraryStore((s) => s.index);
  const refreshCache = useLibraryStore((s) => s.refreshCache);
  const cacheEpoch = useLibraryStore((s) => s.cacheEpoch);

  const theme = useAppStore((s) => s.theme);
  const setTheme = useAppStore((s) => s.setTheme);
  const profile = useAppStore((s) => s.profile);
  const profileMode = useAppStore((s) => s.profileMode);
  const setProfileMode = useAppStore((s) => s.setProfileMode);
  const locale = useAppStore((s) => s.locale);
  const setLocale = useAppStore((s) => s.setLocale);
  const tag = useAppStore((s) => s.tag);

  const searching = scope === "all" && query.trim().length > 0;
  const unverified = !tag.startsWith("swi_");

  return (
    <div className="flex flex-none flex-col">
      <div className={BAR}>
        <button className="btn btn-sm btn-primary" onClick={() => void openRoot()}>
          {t("library.openDirectory")}
        </button>
        <button className="btn btn-sm" onClick={() => void openArchive()} title={t("library.openArchiveHint")}>
          {t("library.openArchive")}
        </button>

        <button className="btn btn-sm btn-square btn-ghost" onClick={back} disabled={!canBack} title={t("library.back")}>
          ←
        </button>
        <button className="btn btn-sm btn-square btn-ghost" onClick={forward} disabled={!canForward} title={t("library.forward")}>
          →
        </button>
        <button className="btn btn-sm btn-square btn-ghost" onClick={() => void up()} disabled={!root} title={t("library.up")}>
          ↑
        </button>

        <Breadcrumbs />

        <div className="flex-1" />

        {unverified && (
          <span className="badge badge-sm badge-warning" title={t("library.unverifiedHint")}>
            {t("library.unverified")}
          </span>
        )}

        <details className="dropdown dropdown-end">
          <summary className="btn btn-sm btn-ghost gap-1.5" title={t("library.profileHint")}>
            {t("library.profile")}{" "}
            {profileMode === "auto"
              ? profile?.label ?? t("library.auto")
              : PROFILE_PRESETS[profileMode].label}
          </summary>
          <ul className="dropdown-content menu menu-sm z-50 mt-2 w-44 rounded-box bg-base-100 p-1 shadow">
            {(["auto", "unicorn_overlord", "thirteen_sentinels"] as ProfileMode[]).map((mode) => (
              <li key={mode}>
                <button
                  className={profileMode === mode ? "menu-active" : ""}
                  onClick={(event) => {
                    const details = event.currentTarget.closest("details");
                    if (details instanceof HTMLDetailsElement) details.open = false;
                    setProfileMode(mode);
                  }}
                >
                  {mode === "auto" ? t("library.autoDetect") : PROFILE_PRESETS[mode].label}
                </button>
              </li>
            ))}
          </ul>
        </details>

        <details className="dropdown dropdown-end">
          <summary className="btn btn-sm btn-ghost gap-1.5" title={t("library.themeHint")}>
            <span className="status status-primary status-xs" />
            {theme}
          </summary>
          <ul className="dropdown-content menu menu-sm z-50 mt-2 w-40 rounded-box bg-base-100 p-1 shadow">
            {THEMES.map((name) => (
              <li key={name}>
                <label
                  className="justify-between"
                  onClick={(event) => {
                    const details = event.currentTarget.closest("details");
                    if (details instanceof HTMLDetailsElement) details.open = false;
                  }}
                >
                  {name}
                  <input
                    type="radio"
                    name="theme"
                    className="theme-controller radio radio-xs"
                    value={name}
                    checked={name === theme}
                    onChange={() => setTheme(name)}
                  />
                </label>
              </li>
            ))}
          </ul>
        </details>

        <button
          className="btn btn-sm btn-ghost"
          onClick={() => setLocale(locale === "en" ? "zh" : "en")}
          title={t("library.locale")}
        >
          {t("locale.name")}
        </button>

        <button
          className={`btn btn-sm btn-ghost ${viewerOpen ? "btn-active" : ""}`}
          onClick={onToggleViewer}
          title={t("library.viewerToggle")}
        >
          {t("library.viewer")}
        </button>

        {index.state === "building" && (
          <span className="badge badge-sm badge-warning" title={t("library.indexIncompleteHint")}>
            {t("library.indexCount", { count: index.count })}
          </span>
        )}
        {index.state === "ready" && <span className="badge badge-sm badge-success">{t("library.indexCount", { count: index.count })}</span>}
        {index.state === "failed" && (
          <span className="badge badge-sm badge-error" title={index.error ?? ""}>
            {t("library.indexFailed")}
          </span>
        )}

        <button
          className="btn btn-sm btn-ghost"
          key={cacheEpoch}
          onClick={() => void refreshCache()}
          title={t("library.refreshCacheHint")}
        >
          {t("library.refreshCache")}
        </button>
      </div>

      <div className={BAR}>
        <input
          className="input input-sm min-w-20 flex-1"
          placeholder={scope === "all" ? t("library.searchAll") : t("library.searchDir")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          disabled={!root}
        />

        <select
          className="select select-sm w-28"
          value={scope}
          onChange={(e) => setScope(e.target.value as "dir" | "all")}
          title={t("library.searchScope")}
        >
          <option value="dir">{t("library.scopeDir")}</option>
          <option value="all">{t("library.scopeAll")}</option>
        </select>

        <details className="dropdown">
          <summary className="btn btn-xs">
            {t("library.format")}{" "}
            {formatFilter.length === 0
              ? t("library.all")
              : `${formatFilter.length}/${availableFormats.length}`}
          </summary>
          <ul className="dropdown-content menu menu-xs z-50 mt-2 max-h-80 w-64 overflow-auto rounded-box bg-base-100 p-1 shadow">
            <li>
              <button onClick={clearFormats}>{t("library.allFormats")}</button>
            </li>
            {availableFormats.map((format) => (
              <li key={format.ext}>
                <label className="justify-between">
                  <span>
                    {format.label}
                    <span className="ml-1 font-mono text-base-content/50">.{format.ext}</span>
                  </span>
                  <input
                    type="checkbox"
                    className="checkbox checkbox-xs"
                    checked={formatFilter.includes(format.ext)}
                    onChange={() => toggleFormat(format.ext)}
                  />
                </label>
              </li>
            ))}
          </ul>
        </details>

        <div className="flex-1" />

        {!searching && hiddenFiles > 0 && (
          <span className="font-mono text-xs text-base-content/60" title={t("library.notCataloguedHint")}>
            {t("library.notCatalogued", { count: hiddenFiles })}
          </span>
        )}

        {root && (
          <button className="btn btn-sm btn-ghost" onClick={() => void reload()} title={t("library.reloadHint")}>
            {t("library.reload")}
          </button>
        )}
      </div>
    </div>
  );
}
