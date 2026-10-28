/**
 * 位图缓存：把一段 GOP 解出的帧并进按**绝对帧号**索引的表。
 *
 * 单独抽出来是为了能在 Node 里跑断言 —— 这里出过一次「解出 30 帧、缓存 0 帧」
 * 的事故：合并原本写在 `setFrames` 的 updater 里，而 updater 是**延后执行**的，
 * 调用方在 `finally` 里已经把源数组清空了，updater 拿到的是空数组。
 * 所以这个函数必须**纯**：不关闭任何位图，把该关的放在 `dropped` 里交给调用方，
 * 且调用方传进来的 `batch` 必须是**不可变的快照**。
 */

export interface FrameBatchItem {
  /** 绝对帧号。 */
  index: number;
  bitmap: ImageBitmap;
}

export interface FrameMerge {
  /** 合并后的新表（不修改入参）。 */
  next: Map<number, ImageBitmap>;
  /** 调用方负责 `close()` 的位图：被淘汰的旧段、以及被同帧新位图替换的。 */
  dropped: ImageBitmap[];
}

/**
 * @param keepFrom 小于这个帧号的旧帧会被淘汰（只留「上一段 + 这一段」，
 *                 内存不随播放时长增长）。
 */
export function mergeFrameBatch(
  previous: ReadonlyMap<number, ImageBitmap>,
  batch: readonly FrameBatchItem[],
  keepFrom: number,
): FrameMerge {
  const next = new Map<number, ImageBitmap>();
  const dropped: ImageBitmap[] = [];

  for (const [index, bitmap] of previous) {
    if (index < keepFrom) dropped.push(bitmap);
    else next.set(index, bitmap);
  }
  for (const item of batch) {
    const old = next.get(item.index);
    if (old && old !== item.bitmap) dropped.push(old);
    next.set(item.index, item.bitmap);
  }

  return { next, dropped };
}
