import type { DirListing, FileEntry, Kind } from "../../lib/ipc";

export interface Row {
  path: string;
  name: string;
  isDir: boolean;
  kind: Kind | null;
  ext: string;
  label: string;
  icon: string;
  size: number;
  mtimeMs: number;
  hasThumb: boolean;
  viewers: string[];
}

export interface Column {
  dir: string;
  entries: Row[];
  phase: "loading" | "ready" | "error";
  error: string | null;
  selected: string | null;
  hiddenFiles: number;
  warnings: string[];
}

export function dirRow(entry: { path: string; name: string; kind?: Kind | null }): Row {
  return {
    path: entry.path,
    name: entry.name,
    isDir: true,
    kind: entry.kind ?? null,
    ext: "",
    label: entry.kind === "archive" ? "CPK" : "",
    icon: entry.kind === "archive" ? "archive" : "folder",
    size: 0,
    mtimeMs: 0,
    hasThumb: false,
    viewers: entry.kind === "archive" ? ["overview", "doc", "hex"] : [],
  };
}

export function fileRow(entry: FileEntry): Row {
  return {
    path: entry.path,
    name: entry.name,
    isDir: false,
    kind: entry.kind,
    ext: entry.ext,
    label: entry.label,
    icon: entry.icon,
    size: entry.size,
    mtimeMs: entry.mtimeMs,
    hasThumb: entry.hasThumb,
    viewers: entry.viewers,
  };
}

export function toRows(listing: DirListing): Row[] {
  const rows = listing.dirs.map(dirRow);
  for (const file of listing.files) rows.push(fileRow(file));
  return rows;
}

export function emptyColumn(dir: string): Column {
  return {
    dir,
    entries: [],
    phase: "loading",
    error: null,
    selected: null,
    hiddenFiles: 0,
    warnings: [],
  };
}

export function columnFrom(listing: DirListing): Column {
  return {
    dir: listing.path,
    entries: toRows(listing),
    phase: "ready",
    error: null,
    selected: null,
    hiddenFiles: listing.hiddenFiles,
    warnings: listing.warnings,
  };
}

export function parentOf(path: string): string | null {
  const cut = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  if (cut <= 0) return null;
  const parent = path.slice(0, cut);
  return parent.endsWith(":") ? `${parent}\\` : parent;
}

export function chainFor(root: string, target: string): string[] {
  const chain: string[] = [];
  let cursor: string | null = target;
  let guard = 0;

  while (cursor && cursor.toLowerCase() !== root.toLowerCase() && guard < 64) {
    chain.unshift(cursor);
    cursor = parentOf(cursor);
    guard += 1;
  }
  chain.unshift(root);
  return chain;
}

export function isArchivePath(path: string): boolean {
  return /\.cpk$/i.test(path);
}

export function withSelection(columns: Column[], index: number, path: string | null): Column[] {
  return columns.map((column, at) => (at === index ? { ...column, selected: path } : column));
}

export function truncate(columns: Column[], keep: number): Column[] {
  return columns.slice(0, keep);
}

export function step(
  entries: Row[],
  current: string | null,
  delta: number,
): string | null {
  if (entries.length === 0) return null;
  const at = entries.findIndex((row) => row.path === current);
  if (at < 0) return entries[delta > 0 ? 0 : entries.length - 1].path;
  const next = Math.min(Math.max(at + delta, 0), entries.length - 1);
  return entries[next].path;
}

export function firstByLetter(entries: Row[], letter: string): string | null {
  const needle = letter.toLowerCase();
  const hit = entries.find((row) => row.name.toLowerCase().startsWith(needle));
  return hit?.path ?? null;
}
