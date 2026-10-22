/** 缩略图 Blob URL 的前端 LRU + in-flight 去重。 */

import { api } from "./ipc";

const MAX = 400;
/** 列表滚动停止多久后才开始解码缩略图。 */
const SCROLL_IDLE_MS = 300;
const cache = new Map<string, { url: string; used: number }>();
const inflight = new Map<string, Promise<string>>();

/** 串行闸门：列表缩略图一次只解码一张，避免与预览争 CPU。 */
let serialTail: Promise<unknown> = Promise.resolve();
let lastScrollAt = 0;

function runSerial<T>(task: () => Promise<T>): Promise<T> {
  const next = serialTail.then(task, task);
  serialTail = next.catch(() => undefined);
  return next;
}

async function waitForScrollIdle(): Promise<void> {
  while (performance.now() - lastScrollAt < SCROLL_IDLE_MS) {
    await new Promise((resolve) => setTimeout(resolve, 60));
  }
}

/** 列表滚动时调用：推迟缩略图解码，先把滚动本身做顺。 */
export function noteListScroll(): void {
  lastScrollAt = performance.now();
}

async function fetchUrl(key: string, load: () => Promise<ArrayBuffer>): Promise<string> {
  const hit = cache.get(key);
  if (hit) {
    hit.used = performance.now();
    return hit.url;
  }

  const pending = inflight.get(key);
  if (pending) return pending;

  const task = (async () => {
    const raw = await load();
    const bytes =
      raw instanceof ArrayBuffer
        ? raw
        : (new Uint8Array(raw as unknown as Uint8Array).buffer as ArrayBuffer);
    const url = URL.createObjectURL(new Blob([bytes], { type: "image/png" }));
    cache.set(key, { url, used: performance.now() });
    evict();
    return url;
  })();

  inflight.set(
    key,
    task.finally(() => inflight.delete(key)),
  );
  return task;
}

/** 纹理文件第 `index` 帧的缩略图。 */
export function getThumbUrl(path: string, index: number): Promise<string> {
  return fetchUrl(`${path.toLowerCase()}#${index}`, () => api.getThumbPng(path, index));
}

/** 查看器用的原尺寸帧图。单独一张缓存，避免和缩略图抢 LRU 名额。 */
const frames = new Map<string, string>();

export async function getFrameUrl(path: string, index: number): Promise<string> {
  const key = `${path.toLowerCase()}#${index}`;
  const hit = frames.get(key);
  if (hit) return hit;

  const raw = await api.getFramePng(path, index);
  const bytes =
    raw instanceof ArrayBuffer
      ? raw
      : (new Uint8Array(raw as unknown as Uint8Array).buffer as ArrayBuffer);
  const url = URL.createObjectURL(new Blob([bytes], { type: "image/png" }));
  frames.set(key, url);
  while (frames.size > 24) {
    const [oldest, oldestUrl] = frames.entries().next().value as [string, string];
    URL.revokeObjectURL(oldestUrl);
    frames.delete(oldest);
  }
  return url;
}

/** 网格用的缩略图：立即请求（看图时人在等图）。 */
export function getFileThumbUrl(path: string): Promise<string> {
  return fetchUrl(`file#${path.toLowerCase()}`, () => api.fileThumb(path));
}

/** 列表用的缩略图：滚动停止后才串行解码，行内图标不值得卡住滚动。 */
export function getFileThumbUrlDeferred(path: string): Promise<string> {
  return fetchUrl(`file#${path.toLowerCase()}`, () =>
    runSerial(async () => {
      await waitForScrollIdle();
      return api.fileThumb(path);
    }),
  );
}

function evict() {
  while (cache.size > MAX) {
    let oldestKey = "";
    let oldest = Infinity;
    for (const [key, value] of cache) {
      if (value.used < oldest) {
        oldest = value.used;
        oldestKey = key;
      }
    }
    const entry = cache.get(oldestKey);
    if (entry) URL.revokeObjectURL(entry.url);
    cache.delete(oldestKey);
  }
}

export function clearThumbUrls() {
  for (const { url } of cache.values()) URL.revokeObjectURL(url);
  cache.clear();
  for (const url of frames.values()) URL.revokeObjectURL(url);
  frames.clear();
}
