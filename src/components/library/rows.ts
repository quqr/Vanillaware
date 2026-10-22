/** 列表/网格共用的「一行」抽象：本目录文件与全局搜索结果在这里归一。 */

import { useMemo } from "react";
import { useLibraryStore } from "../../store/libraryStore";

export interface Row {
  path: string;
  name: string;
  /** 格式名（FTEX / MBS / …）。 */
  label: string;
  /** 小写、不含点的扩展名；格式过滤以它为准。 */
  ext: string;
  /** 图标语义名（后端注册表给出）。 */
  icon: string;
  size: number | null;
  mtimeMs: number | null;
  hasThumb: boolean;
  /** 全局搜索命中时的所在目录。 */
  dir: string | null;
}

export function useRows(): { rows: Row[]; searching: boolean; total: number; truncated: boolean } {
  const files = useLibraryStore((s) => s.files);
  const formatFilter = useLibraryStore((s) => s.formatFilter);
  const query = useLibraryStore((s) => s.query);
  const scope = useLibraryStore((s) => s.scope);
  const search = useLibraryStore((s) => s.search);

  const searching = scope === "all" && query.trim().length > 0;

  return useMemo(() => {
    const visible = (ext: string) =>
      formatFilter.length === 0 || formatFilter.includes(ext);

    if (searching) {
      const items = (search?.items ?? []).filter((item) => visible(item.ext));
      const rows: Row[] = items.map((item) => ({
        path: item.path,
        name: item.name,
        label: item.label,
        ext: item.ext,
        icon: item.icon,
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

    const needle = query.trim().toLowerCase();
    const rows: Row[] = files
      .filter((file) => visible(file.ext))
      .filter((file) => needle.length === 0 || file.name.toLowerCase().includes(needle))
      .map((file) => ({
        path: file.path,
        name: file.name,
        label: file.label,
        ext: file.ext,
        icon: file.icon,
        size: file.size,
        mtimeMs: file.mtimeMs,
        hasThumb: file.hasThumb,
        dir: null,
      }));
    return { rows, searching, total: rows.length, truncated: false };
  }, [files, formatFilter, query, scope, search, searching]);
}

function parentOf(path: string): string {
  const cut = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  return cut > 0 ? path.slice(0, cut) : path;
}
