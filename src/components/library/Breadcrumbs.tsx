import { useLibraryStore } from "../../store/libraryStore";

/** 面包屑：从根到当前目录，每一段可点。 */
export function Breadcrumbs() {
  const root = useLibraryStore((s) => s.root);
  const dir = useLibraryStore((s) => s.dir);
  const navigate = useLibraryStore((s) => s.navigate);

  if (!root || !dir) return <div className="px-2 text-xs text-base-content/50">未打开目录</div>;

  const segments = buildSegments(root, dir);

  return (
    <div className="breadcrumbs min-w-0 overflow-hidden text-xs" title={dir}>
      <ul>
        {segments.map((segment) => {
          const active = segment.path.toLowerCase() === dir.toLowerCase();
          return (
            <li key={segment.path}>
              <button
                className={`link link-hover max-w-48 truncate ${active ? "font-semibold text-base-content" : "text-base-content/60"}`}
                aria-current={active ? "page" : undefined}
                onClick={() => void navigate(segment.path)}
              >
                {segment.name}
              </button>
            </li>
          );
        })}
      </ul>
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
      cursor = joinPath(cursor, part);
      segments.push({ name: part, path: cursor });
    }
    return segments;
  }

  // 落到了根之外（例如「上一级」跳出去）：退化为按绝对路径分段。
  const parts = dir.split(/[\\/]/).filter(Boolean);
  let cursor = "";
  return parts.map((part, index) => {
    cursor = index === 0 ? `${part}\\` : joinPath(cursor, part);
    return { name: part, path: cursor };
  });
}

function joinPath(base: string, part: string): string {
  if (base.endsWith("\\") || base.endsWith("/")) return `${base}${part}`;
  return `${base}\\${part}`;
}
