export interface GopRange {
  gop: number;

  start: number;

  end: number;
}

export function gopIndexAt(keyframes: number[], frame: number): number {
  if (keyframes.length === 0) return 0;
  let low = 0;
  let high = keyframes.length - 1;
  while (low < high) {
    const middle = Math.ceil((low + high) / 2);
    if (keyframes[middle] <= frame) {
      low = middle;
    } else {
      high = middle - 1;
    }
  }
  return low;
}

export function gopRange(keyframes: number[], total: number, gop: number): GopRange {
  const start = keyframes[gop] ?? 0;
  const end = keyframes[gop + 1] ?? total;
  return { gop, start, end: Math.max(start + 1, end) };
}

export interface WindowQuery {
  keyframes: number[];
  total: number;
  current: number;
  loaded: GopRange | null;

  has: (frame: number) => boolean;

  prefetchAhead: number;
}

export function nextGopToLoad(query: WindowQuery): number | null {
  const { keyframes, total, loaded, has, prefetchAhead } = query;
  if (keyframes.length === 0 || total <= 0) return null;
  const clamp = (frame: number) => Math.max(0, Math.min(total - 1, frame));

  if (!loaded) return gopIndexAt(keyframes, clamp(query.current));

  if (!has(clamp(query.current))) {
    const target = gopIndexAt(keyframes, clamp(query.current));
    return target === loaded.gop ? null : target;
  }

  const ahead = clamp(query.current + prefetchAhead);
  const aheadGop = gopIndexAt(keyframes, ahead);
  if (aheadGop !== loaded.gop && !has(ahead)) return aheadGop;
  return null;
}
