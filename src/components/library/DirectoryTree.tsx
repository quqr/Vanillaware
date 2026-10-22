import { useLibraryStore } from "../../store/libraryStore";

/** 目录树：真实目录，懒加载，空目录照常显示。 */
export function DirectoryTree() {
  const root = useLibraryStore((s) => s.root);
  const tree = useLibraryStore((s) => s.tree);
  const collapseAll = useLibraryStore((s) => s.collapseAll);

  if (!root) {
    return <div className="p-2 text-xs text-base-content/50">未打开目录</div>;
  }

  const rootName = root.split(/[\\/]/).filter(Boolean).pop() ?? root;

  return (
    <nav className="flex min-h-0 flex-1 flex-col">
      <ul className="menu menu-xs w-full flex-1 overflow-auto p-0 pb-2">
        <li className="menu-title text-[10px] uppercase tracking-wider">目录</li>
        <TreeNode path={root} name={rootName} depth={0} />
        {tree[root]?.phase === "error" && (
          <li className="px-2 py-1 text-xs text-base-content/50">读取失败</li>
        )}
        <li className="h-2.5" aria-hidden="true" />
        <li>
          <button onClick={() => collapseAll(root)} title="只保留根目录展开">
            折叠全部
          </button>
        </li>
      </ul>
    </nav>
  );
}

function TreeNode({ path, name, depth }: { path: string; name: string; depth: number }) {
  const expanded = useLibraryStore((s) => Boolean(s.expanded[path]));
  const node = useLibraryStore((s) => s.tree[path]);
  const active = useLibraryStore((s) => s.dir?.toLowerCase() === path.toLowerCase());
  const toggleExpanded = useLibraryStore((s) => s.toggleExpanded);
  const navigate = useLibraryStore((s) => s.navigate);

  const hasChildren = (node?.dirs.length ?? 0) > 0;

  return (
    <>
      <li>
        <button
          className={active ? "menu-active" : ""}
          style={{ paddingLeft: 6 + depth * 12 }}
          title={path}
          onClick={() => {
            // 行点击 = 导航；未加载过的节点顺手展开。折叠只由箭头负责，避免“想进去却被收起”。
            if (!node) toggleExpanded(path);
            void navigate(path);
          }}
        >
          <span
            className={`w-2.5 shrink-0 text-base-content/50 transition-transform ${
              expanded ? "rotate-90" : ""
            } ${!hasChildren && node?.phase === "ready" ? "invisible" : ""}`}
            onClick={(event) => {
              event.stopPropagation();
              toggleExpanded(path);
            }}
          >
            {hasChildren || node?.phase !== "ready" ? "▸" : ""}
          </span>
          <span className="truncate">{name}</span>
          {node?.phase === "loading" && <span className="font-mono text-base-content/50">…</span>}
        </button>
      </li>

      {expanded &&
        node?.dirs.map((child) => (
          <TreeNode key={child.path} path={child.path} name={child.name} depth={depth + 1} />
        ))}
    </>
  );
}
