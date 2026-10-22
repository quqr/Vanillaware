/** 列表/网格共用的「一行」抽象：本目录文件与全局搜索结果在这里归一。 */

import { useMemo } from "react";
import type { Kind } from "../../lib/ipc";
import { useLibraryStore } from "../../store/libraryStore";

export interface Row {
  path: string;
  name: string;
  /** 格式名（FTEX / MBS / …）。 */
  label: string;
  kind: Kind;
  size: number | null;
  mtimeMs: number | null;
  hasThumb: boolean;
  /** 全局搜索命中时的所在目录。 */
  dir: string | null;
}

export function useRows(): { rows: Row[]; searching: boolean; total: number; truncated: boolean } {
  const files = useLibraryStore((s) => s.files);
  const kinds = useLibraryStore((s) => s.kinds);
  const query = useLibraryStore((s) => s.query);
  const scope = useLibraryStore((s) => s.scope);
  const search = useLibraryStore((s) => s.search);

  const searching = scope === "all" && query.trim().length > 0;

  return useMemo(() => {
    if (searching) {
      const items = search?.items ?? [];
      const filtered = kinds.length === 0 ? items : items.filter((item) => kinds.includes(item.kind));
      const rows: Row[] = filtered.map((item) => ({
        path: item.path,
        name: item.name,
        label: item.label,
        kind: item.kind,
        size: null,
        mtimeMs: null,
        hasThumb: false,
        dir: parentOf(item.path),
      }));
      return {
        rows,
        searching,
        total: search?.total ?? 0,
        truncated: search?.truncated ?? false,
      };
    }

    const visible = kinds.length === 0 ? files : files.filter((file) => kinds.includes(file.kind));
    const rows: Row[] = visible.map((file) => ({
      path: file.path,
      name: file.name,
      label: file.label,
      kind: file.kind,
      size: file.size,
      mtimeMs: file.mtimeMs,
      hasThumb: file.hasThumb,
      dir: null,
    }));
    return { rows, searching, total: rows.length, truncated: false };
  }, [files, kinds, query, scope, search, searching]);
}

function parentOf(path: string): string {
  const cut = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  return cut > 0 ? path.slice(0, cut) : path;
}
