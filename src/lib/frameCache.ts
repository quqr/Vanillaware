export interface FrameBatchItem {

  index: number;
  bitmap: ImageBitmap;
}

export interface FrameMerge {

  next: Map<number, ImageBitmap>;

  dropped: ImageBitmap[];
}

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
