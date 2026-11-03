import { useEffect, useState } from "react";

import { t } from "../../i18n";
import { formatBytes, formatCount } from "../../lib/format";
import { api, type ArchiveSummary } from "../../lib/ipc";

export function ArchiveOverview({ path }: { path: string }) {
  const [summary, setSummary] = useState<ArchiveSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    setSummary(null);
    setError(null);
    setBusy(true);
    api
      .archiveSummary(path)
      .then((value) => {
        if (live) setSummary(value);
      })
      .catch((reason) => {
        if (live) setError(String(reason));
      })
      .finally(() => {
        if (live) setBusy(false);
      });
    return () => {
      live = false;
    };
  }, [path]);

  if (busy) {
    return (
      <div className="p-4 font-mono text-xs text-base-content/50">{t("archive.reading")}</div>
    );
  }

  if (error || !summary) {
    return (
      <div className="m-2 alert alert-error alert-soft text-xs break-all">
        {error ?? t("archive.failed")}
      </div>
    );
  }

  const ratio = summary.entries > 0 ? Math.round((summary.compressed * 100) / summary.entries) : 0;
  const saving =
    summary.totalExtract > 0
      ? Math.max(0, Math.round((1 - summary.fileSize / summary.totalExtract) * 100))
      : 0;

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-auto">
      <table className="table table-xs">
        <tbody>
          <Row label={t("archive.entries")} value={formatCount(summary.entries)} />
          <Row label={t("archive.directories")} value={formatCount(summary.directories)} />
          <Row label={t("archive.archiveSize")} value={formatBytes(summary.fileSize)} />
          <Row label={t("archive.extractSize")} value={formatBytes(summary.totalExtract)} />
          <Row label={t("archive.saving")} value={`${saving}%`} />
          <Row label={t("archive.compressed")} value={`${formatCount(summary.compressed)} (${ratio}%)`} />
          <Row label={t("archive.version")} value={`${summary.version}.${summary.revision}`} />
          <Row label={t("archive.align")} value={String(summary.align)} />
          <Row label={t("archive.tocOffset")} value={String(summary.tocOffset)} />
          <Row label={t("archive.contentOffset")} value={String(summary.contentOffset)} />
          <Row
            label={t("archive.encrypted")}
            value={
              summary.headerEncrypted || summary.tocEncrypted
                ? `${summary.headerEncrypted ? "header" : ""}${summary.headerEncrypted && summary.tocEncrypted ? " + " : ""}${summary.tocEncrypted ? "toc" : ""}`
                : t("archive.plain")
            }
          />
        </tbody>
      </table>

      {summary.outOfRange > 0 && (
        <div className="m-2 alert alert-warning alert-soft text-xs">
          {t("archive.outOfRange", { count: summary.outOfRange })}
        </div>
      )}

      <div className="mt-auto flex items-center gap-2 border-t border-base-300 p-2">
        <button
          className="btn btn-xs"
          onClick={() => {
            void api.archiveCacheClear().then(() => {
              setBusy(true);
              void api.archiveSummary(path).then((value) => {
                setSummary(value);
                setBusy(false);
              });
            });
          }}
        >
          {t("archive.clearCache")}
        </button>
        <span className="font-mono text-[10px] text-base-content/40">{summary.magic}</span>
      </div>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <tr>
      <td className="w-1/2 text-base-content/60">{label}</td>
      <td className="font-mono">{value}</td>
    </tr>
  );
}
