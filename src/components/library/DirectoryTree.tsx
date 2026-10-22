import { useLibraryStore } from "../../store/libraryStore";

/** 目录树：真实目录，懒加载，空目录照常显示。 */
export function DirectoryTree() {
  const root = useLibraryStore((s) => s.root);
  const tree = useLibraryStore((s) => s.tree);
  const collapseAll = useLibraryStore((s) => s.collapseAll);

  if (!root) {
    return (
      <nav className="tree">
        <div className="tree-label">目录</div>
        <div className="dim" style={{ padding: "8px 10px" }}>
          未打开目录
        </div>
      </nav>
    );
  }

  const rootName = root.split(/[\\/]/).filter(Boolean).pop() ?? root;

  return (
    <nav className="tree">
      <div className="tree-label">目录</div>
      <TreeNode path={root} name={rootName} depth={0} />
      {tree[root]?.phase === "error" && (
        <div className="dim" style={{ padding: "4px 10px" }}>
          读取失败
        </div>
      )}
      <div className="tree-spacer" />
      <button className="tree-item" onClick={() => collapseAll(root)} title="只保留根目录展开">
        <span>折叠全部</span>
      </button>
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
      <div
        className="tree-node"
        data-active={active}
        style={{ paddingLeft: 6 + depth * 12 }}
        title={path}
        onClick={() => {
          // 行点击 = 导航；未加载过的节点顺手展开。折叠只由箭头负责，避免“想进去却被收起”。
          if (!node) toggleExpanded(path);
          void navigate(path);
        }}
      >
        <span
          className="chev"
          data-expanded={expanded}
          data-leaf={!hasChildren && node?.phase === "ready"}
          onClick={(event) => {
            event.stopPropagation();
            toggleExpanded(path);
          }}
        >
          {hasChildren || node?.phase !== "ready" ? "▸" : ""}
        </span>
        <span className="name">{name}</span>
        {node?.phase === "loading" && <span className="dim mono">…</span>}
      </div>

      {expanded &&
        node?.dirs.map((child) => (
          <TreeNode key={child.path} path={child.path} name={child.name} depth={depth + 1} />
        ))}
    </>
  );
}
