/**
 * 影片的「窗口」逻辑：当前该解哪一段 GOP。
 *
 * 抽成纯函数是为了能测——播放时那种「突然闪一下」正是这段判断的锅：
 * 以前是**播到当前段最后才去取下一段**，取流 + 解码要 100~200 ms，
 * 这几帧里没有画面，画布就被清空了。
 */

export interface GopRange {
  gop: number;
  /** 含。 */
  start: number;
  /** 不含。 */
  end: number;
}

/** 帧号 → 所属 GOP 下标（`keyframes` 升序）。 */
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
  /** 某一帧是否已经在缓存里。 */
  has: (frame: number) => boolean;
  /** 提前多少帧开始取下一段。 */
  prefetchAhead: number;
}

/**
 * 返回要加载的 GOP 下标；`null` 表示缓存够用、不用动。
 *
 * 三条规则：
 * 1. 还没加载过 → 取当前帧所在段；
 * 2. 当前帧不在缓存里（拖动跳段）→ 取它所在段；
 * 3. 当前帧要到段尾了（`prefetchAhead` 以内）→ 提前取下一段，
 *    但下一段已经在缓存里就不重复取。
 */
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
