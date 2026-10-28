import { api } from "./ipc";

const MAX = 400;

const SCROLL_IDLE_MS = 300;
const cache = new Map<string, { url: string; used: number }>();
const inflight = new Map<string, Promise<string>>();

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

export function getThumbUrl(path: string, index: number): Promise<string> {
  return fetchUrl(`${path.toLowerCase()}#${index}`, () => api.getThumbPng(path, index));
}

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

export function getFileThumbUrl(path: string): Promise<string> {
  return fetchUrl(`file#${path.toLowerCase()}`, () => api.fileThumb(path));
}

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
