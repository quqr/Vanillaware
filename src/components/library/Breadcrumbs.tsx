import { useLibraryStore } from "../../store/libraryStore";

/** 面包屑：从根到当前目录，每一段可点。 */
export function Breadcrumbs() {
  const root = useLibraryStore((s) => s.root);
  const dir = useLibraryStore((s) => s.dir);
  const navigate = useLibraryStore((s) => s.navigate);

  if (!root || !dir) return <div className="crumbs dim">未打开目录</div>;

  const segments = buildSegments(root, dir);

  return (
    <div className="crumbs" title={dir}>
      {segments.map((segment, index) => (
        <span key={segment.path} className="crumb-item">
          {index > 0 && <span className="crumb-sep">›</span>}
          <button
            className="crumb"
            data-active={segment.path.toLowerCase() === dir.toLowerCase()}
            onClick={() => void navigate(segment.path)}
          >
            {segment.name}
          </button>
        </span>
      ))}
    </div>
  );
}

function buildSegments(root: string, dir: string): { name: string; path: string }[] {
  const rootName = root.split(/[\\/]/).filter(Boolean).pop() ?? root;
  const segments = [{ name: rootName, path: root }];

  if (dir.toLowerCase().startsWith(root.toLowerCase())) {
    const rest = dir.slice(root.length).replace(/^[\\/]+/, "");
    if (!rest) return segments;

    let cursor = root;
    for (const part of rest.split(/[\\/]/)) {
      cursor = `${cursor}\\${part}`;
      segments.push({ name: part, path: cursor });
    }
    return segments;
  }

  // 落到了根之外（例如「上一级」跳出去）：退化为按绝对路径分段。
  const parts = dir.split(/[\\/]/).filter(Boolean);
  let cursor = "";
  return parts.map((part, index) => {
    cursor = index === 0 ? `${part}\\` : `${cursor}${cursor.endsWith("\\") ? "" : "\\"}${part}`;
    return { name: part, path: cursor };
  });
}
