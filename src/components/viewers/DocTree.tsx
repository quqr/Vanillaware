import type { DocMark, DocNode, DocPage } from "../../lib/ipc";

const MARK_BADGE: Record<DocMark, string> = {
  note: "badge-info",
  warn: "badge-warning",
  error: "badge-error",
};

const MARK_TEXT: Record<DocMark, string> = {
  note: "text-info",
  warn: "text-warning",
  error: "text-error",
};

export function DocTree({
  nodes,
  depth,
  pages,
  expanded,
  loading,
  onToggle,
  onMore,
}: {
  nodes: DocNode[];
  depth: number;
  pages: Map<number, DocPage>;
  expanded: Set<number>;
  loading: Set<number>;
  onToggle: (node: DocNode) => void;
  onMore: (node: DocNode) => void;
}) {
  if (depth > 63) {
    return <div className="py-0.5 text-base-content/40">… 深度上限</div>;
  }

  return (
    <>
      {nodes.map((node) => {
        const page = pages.get(node.id);
        const open = expanded.has(node.id);
        const container = node.childCount > 0;
        const mark = node.mark;

        return (
          <div key={node.id}>
            <div
              className="flex items-start gap-1.5 py-0.5 pr-1 hover:bg-base-200"
              style={{ paddingLeft: 4 + depth * 12 }}
            >
              <button
                className="w-3 shrink-0 text-left text-base-content/40 disabled:opacity-0"
                onClick={() => onToggle(node)}
                disabled={!container}
                aria-label={open ? "折叠" : "展开"}
              >
                {container ? (open ? "▾" : "▸") : " "}
              </button>

              <span className="w-[4.8rem] shrink-0 text-right text-base-content/40">
                {node.offset === null ? "" : node.offset.toString(16).padStart(6, "0")}
              </span>

              <span className={`min-w-0 flex-1 break-all ${mark ? MARK_TEXT[mark] : ""}`}>
                {node.key && <span className="text-info">{node.key}</span>}
                {node.key && node.key !== node.name && <span className="text-base-content/40"> · </span>}
                {node.value === null ? (
                  <span className="text-base-content/80">{node.name}</span>
                ) : (
                  <span className={node.kind === "text" ? "text-success" : "text-base-content/80"}>
                    {node.value}
                  </span>
                )}
                {node.len !== null && (
                  <span className="ml-1 text-base-content/40">[{node.len} B]</span>
                )}
              </span>

              {mark && <span className={`badge badge-xs ${MARK_BADGE[mark]}`}>{mark}</span>}
            </div>

            {open && page && (
              <>
                <DocTree
                  nodes={page.items}
                  depth={depth + 1}
                  pages={pages}
                  expanded={expanded}
                  loading={loading}
                  onToggle={onToggle}
                  onMore={onMore}
                />
                {page.items.length < page.total && (
                  <button
                    className="btn btn-xs btn-ghost my-0.5"
                    style={{ marginLeft: 4 + (depth + 1) * 12 }}
                    disabled={loading.has(node.id)}
                    onClick={() => onMore(node)}
                  >
                    加载更多（{page.total - page.items.length}）
                  </button>
                )}
              </>
            )}

            {open && !page && loading.has(node.id) && (
              <div className="py-0.5 text-base-content/40" style={{ paddingLeft: 4 + (depth + 1) * 12 }}>
                读取中…
              </div>
            )}
          </div>
        );
      })}
    </>
  );
}
