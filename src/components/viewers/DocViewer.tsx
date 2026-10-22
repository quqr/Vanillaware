import { useCallback, useEffect, useRef, useState } from "react";
import { api, type DocNode, type DocOpen, type DocPage, type FileEntry } from "../../lib/ipc";
import { DocTree } from "./DocTree";

const PAGE = 100;

/** 统一结构查看器：后端把格式投影成 arena，这里只做分页展开。 */
export function DocViewer({ entry, tag }: { entry: FileEntry; tag: string }) {
  const [open, setOpen] = useState<DocOpen | null>(null);
  const [pages, setPages] = useState<Map<number, DocPage>>(new Map());
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [loading, setLoading] = useState<Set<number>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [epoch, setEpoch] = useState(0);

  const handleRef = useRef<number | null>(null);

  const docTag = entry.ext === "mbs" || entry.ext === "mbp" ? tag : undefined;

  useEffect(() => {
    let alive = true;
    handleRef.current = null;
    setOpen(null);
    setPages(new Map());
    setExpanded(new Set());
    setLoading(new Set());
    setError(null);
    setBusy(true);

    api
      .openDoc(entry.path, docTag)
      .then((next) => {
        if (!alive) return;
        handleRef.current = next.doc.handle;
        setOpen(next);
        setPages(new Map([[next.doc.root, next.page]]));
        setExpanded(new Set([next.doc.root]));
      })
      .catch((e) => {
        if (alive) setError(String(e));
      })
      .finally(() => {
        if (alive) setBusy(false);
      });

    return () => {
      alive = false;
    };
  }, [entry.path, docTag, epoch]);

  const loadPage = useCallback(
    async (node: DocNode, offset: number) => {
      const current = handleRef.current;
      if (current === null) return;

      setLoading((prev) => new Set(prev).add(node.id));
      try {
        const page = await api.docChildren(current, node.id, offset, PAGE);
        if (handleRef.current !== current) return;

        setPages((prev) => {
          const previous = prev.get(node.id);
          const items = offset === 0 ? page.items : [...(previous?.items ?? []), ...page.items];
          return new Map(prev).set(node.id, { ...page, offset: 0, items });
        });
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading((prev) => {
          const next = new Set(prev);
          next.delete(node.id);
          return next;
        });
      }
    },
    [],
  );

  function toggle(node: DocNode) {
    if (node.childCount === 0) return;

    if (expanded.has(node.id)) {
      setExpanded((prev) => {
        const next = new Set(prev);
        next.delete(node.id);
        return next;
      });
      return;
    }

    setExpanded((prev) => new Set(prev).add(node.id));
    if (!pages.has(node.id)) void loadPage(node, 0);
  }

  function more(node: DocNode) {
    const page = pages.get(node.id);
    void loadPage(node, page?.items.length ?? 0);
  }

  if (busy && !open) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">
        构建文档树…
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-2 p-4">
        <div className="alert alert-error alert-soft text-sm break-all">{error}</div>
        <button className="btn btn-sm" onClick={() => setEpoch((value) => value + 1)}>
          重新打开
        </button>
      </div>
    );
  }

  if (!open) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">
        无内容
      </div>
    );
  }

  const { doc } = open;
  const rootPage = pages.get(doc.root);
  const passed = doc.checks.filter((check) => check.ok).length;

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex flex-none flex-col gap-1 border-b border-base-300 bg-base-200 p-2">
        <div className="truncate font-mono text-xs font-semibold" title={doc.path}>
          {doc.path}
        </div>
        <div className="flex flex-wrap items-center gap-1 text-[10px]">
          <span className="badge badge-xs badge-primary">{doc.label}</span>
          <span className="badge badge-xs">.{doc.ext}</span>
          <span className="badge badge-xs">nodes {doc.totalNodes}</span>
          <span className="badge badge-xs">{doc.buildMs} ms</span>
          <span className={`badge badge-xs ${doc.complete ? "badge-success" : "badge-warning"}`}>
            {doc.complete ? "闭合" : "截断"}
          </span>
          {doc.checks.length > 0 && (
            <span
              className={`badge badge-xs ${passed === doc.checks.length ? "badge-success" : "badge-error"}`}
              title={doc.checks
                .map((check) => `${check.ok ? "✓" : "✗"} ${check.label}${check.detail ? `：${check.detail}` : ""}`)
                .join("\n")}
            >
              check {passed}/{doc.checks.length}
            </span>
          )}
          {doc.warnings.length > 0 && (
            <span className="badge badge-xs badge-warning" title={doc.warnings.join("\n")}>
              warn {doc.warnings.length}
            </span>
          )}
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-2 font-mono text-[11px]">
        {rootPage ? (
          <DocTree
            nodes={rootPage.items}
            depth={0}
            pages={pages}
            expanded={expanded}
            loading={loading}
            onToggle={toggle}
            onMore={more}
          />
        ) : (
          <div className="text-base-content/40">根节点无子项</div>
        )}
      </div>
    </div>
  );
}
