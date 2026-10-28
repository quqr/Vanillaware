import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, type FileEntry, type UsmVideo } from "../../lib/ipc";
import { decodeGop, UnsupportedVideoError, videoDecodable } from "../../lib/videoDecode";

const TIMELINE_HEIGHT = 56;
/** 解码后转成位图时的最大宽度：够看清，又不至于把显存吃光。 */
const MAX_BITMAP_WIDTH = 1280;

/** USM 影片：画面 + 码率时间轴 + 逐帧控制。 */
export function VideoViewer({ entry }: { entry: FileEntry }) {
  const [info, setInfo] = useState<UsmVideo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string>("建立索引…");

  const [bitmaps, setBitmaps] = useState<ImageBitmap[]>([]);
  const [gopIndex, setGopIndex] = useState(-1);
  const [gopStart, setGopStart] = useState(0);
  const [current, setCurrent] = useState(0);
  const [playing, setPlaying] = useState(false);

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const timelineRef = useRef<HTMLCanvasElement>(null);
  const loadingRef = useRef(false);

  // ─── 索引 ───────────────────────────────────────

  useEffect(() => {
    let alive = true;
    setInfo(null);
    setError(null);
    setStatus("建立索引…");
    setBitmaps((previous) => {
      previous.forEach((bitmap) => bitmap.close());
      return [];
    });
    setGopIndex(-1);
    setGopStart(0);
    setCurrent(0);
    setPlaying(false);

    api
      .usmVideoInfo(entry.path)
      .then((next) => {
        if (!alive) return;
        setInfo(next);
        setStatus("");
      })
      .catch((e) => {
        if (alive) {
          setError(String(e));
          setStatus("");
        }
      });
    return () => {
      alive = false;
    };
  }, [entry.path]);

  const gopOf = useCallback(
    (frame: number) => {
      if (!info || !info.keyframes.length) return 0;
      let low = 0;
      let high = info.keyframes.length - 1;
      while (low < high) {
        const middle = Math.ceil((low + high) / 2);
        if (info.keyframes[middle] <= frame) {
          low = middle;
        } else {
          high = middle - 1;
        }
      }
      return low;
    },
    [info],
  );

  const loadGop = useCallback(
    async (gop: number) => {
      if (!info || loadingRef.current) return;
      loadingRef.current = true;
      setStatus("解码 GOP…");
      try {
        const bytes = await api.usmVideoGop(entry.path, gop);
        const startFrame = info.keyframes[gop] ?? 0;
        const frames = await decodeGop(new Uint8Array(bytes), info, startFrame);
        const targetWidth = Math.min(MAX_BITMAP_WIDTH, info.width);
        const targetHeight = Math.max(
          2,
          Math.round((targetWidth * info.height) / Math.max(1, info.width)),
        );
        const next: ImageBitmap[] = [];
        for (const frame of frames) {
          try {
            next.push(
              await createImageBitmap(frame, {
                resizeWidth: targetWidth,
                resizeHeight: targetHeight,
                resizeQuality: "low",
              }),
            );
          } finally {
            frame.close();
          }
        }
        setBitmaps((previous) => {
          previous.forEach((bitmap) => bitmap.close());
          return next;
        });
        setGopStart(startFrame);
        setGopIndex(gop);
        setStatus("");
      } catch (e) {
        setStatus(e instanceof UnsupportedVideoError ? e.message : String(e));
      } finally {
        loadingRef.current = false;
      }
    },
    [entry.path, info],
  );

  // 首屏先解第 0 个 GOP。
  useEffect(() => {
    if (!info || gopIndex >= 0) return;
    void loadGop(gopOf(current));
  }, [info, gopIndex, current, gopOf, loadGop]);

  // 当前帧跨出已解码范围时补载。
  useEffect(() => {
    if (!info || gopIndex < 0 || loadingRef.current) return;
    if (current >= gopStart && current < gopStart + bitmaps.length) return;
    const target = gopOf(current);
    if (target !== gopIndex) {
      void loadGop(target);
    }
  }, [current, info, gopIndex, gopStart, bitmaps.length, gopOf, loadGop]);

  useEffect(
    () => () => {
      setBitmaps((previous) => {
        previous.forEach((bitmap) => bitmap.close());
        return [];
      });
    },
    [],
  );

  // ─── 播放 ───────────────────────────────────────

  useEffect(() => {
    if (!playing || !info) return;
    const interval = window.setInterval(() => {
      setCurrent((previous) => {
        const next = previous + 1;
        if (next >= info.frames) {
          setPlaying(false);
          return previous;
        }
        return next;
      });
    }, 1000 / Math.max(1, info.fps));
    return () => window.clearInterval(interval);
  }, [playing, info]);

  // ─── 画面 ───────────────────────────────────────

  useEffect(() => {
    const canvas = canvasRef.current;
    const bitmap = bitmaps[current - gopStart];
    if (!canvas) return;
    const context = canvas.getContext("2d");
    if (!context) return;
    if (!bitmap) {
      context.clearRect(0, 0, canvas.width, canvas.height);
      return;
    }
    if (canvas.width !== bitmap.width || canvas.height !== bitmap.height) {
      canvas.width = bitmap.width;
      canvas.height = bitmap.height;
    }
    context.drawImage(bitmap, 0, 0);
  }, [bitmaps, current, gopStart]);

  // ─── 时间轴 ─────────────────────────────────────

  useEffect(() => {
    const canvas = timelineRef.current;
    if (!canvas || !info) return;
    const paint = () => {
      const context = canvas.getContext("2d");
      if (!context) return;
      const rect = canvas.getBoundingClientRect();
      const width = Math.max(1, Math.floor(rect.width));
      const height = TIMELINE_HEIGHT;
      const dpr = window.devicePixelRatio || 1;
      canvas.width = Math.floor(width * dpr);
      canvas.height = Math.floor(height * dpr);
      context.setTransform(dpr, 0, 0, dpr, 0, 0);
      context.clearRect(0, 0, width, height);

      const styles = getComputedStyle(canvas);
      const accent = styles.getPropertyValue("--color-primary").trim() || "#3b82f6";
      const faint = styles.getPropertyValue("--color-base-content").trim() || "#888";

      const sizes = info.frameSizes;
      let peak = 1;
      for (const size of sizes) if (size > peak) peak = size;
      const columns = Math.min(sizes.length, width);
      const step = sizes.length / columns;

      for (let column = 0; column < columns; column += 1) {
        const from = Math.floor(column * step);
        const to = Math.max(from + 1, Math.floor((column + 1) * step));
        let max = 0;
        let key = false;
        for (let index = from; index < to && index < sizes.length; index += 1) {
          if (sizes[index] > max) max = sizes[index];
          if (info.keyframes.includes(index)) key = true;
        }
        const barHeight = Math.max(1, (max / peak) * (height - 8));
        context.fillStyle = key ? accent : faint;
        context.globalAlpha = key ? 0.95 : 0.35;
        context.fillRect(column, height - barHeight, 1, barHeight);
      }
      context.globalAlpha = 1;

      // 关键帧刻度
      context.fillStyle = accent;
      for (const key of info.keyframes) {
        const x = (key / Math.max(1, sizes.length)) * width;
        context.fillRect(x, 0, 1, 4);
      }
    };
    paint();
    const observer = new ResizeObserver(paint);
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [info]);

  const seek = useCallback(
    (frame: number) => {
      if (!info) return;
      const clamped = Math.max(0, Math.min(info.frames - 1, Math.round(frame)));
      setPlaying(false);
      setCurrent(clamped);
    },
    [info],
  );

  const onTimelineClick = (event: React.MouseEvent<HTMLDivElement>) => {
    if (!info) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (event.clientX - rect.left) / Math.max(1, rect.width)));
    seek(ratio * (info.frames - 1));
  };

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target && /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName)) return;
      if (event.code === "Space") {
        event.preventDefault();
        setPlaying((value) => !value);
      } else if (event.code === "ArrowRight") {
        event.preventDefault();
        seek(current + 1);
      } else if (event.code === "ArrowLeft") {
        event.preventDefault();
        seek(current - 1);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [current, seek]);

  const progress = useMemo(() => {
    if (!info || !info.frames) return 0;
    return current / info.frames;
  }, [current, info]);

  if (error) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 text-sm text-error">{error}</div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex flex-none flex-wrap items-center gap-1.5 border-b border-base-300 bg-base-200 p-1.5 text-[10px]">
        {info ? (
          <>
            <span className="badge badge-xs badge-primary">{info.codec}</span>
            <span className="badge badge-xs">
              {info.width}×{info.height}
            </span>
            <span className="badge badge-xs">{info.fps.toFixed(2)} fps</span>
            <span className="badge badge-xs">{formatTime(info.durationMs / 1000)}</span>
            <span className="badge badge-xs badge-ghost">{info.frames} 帧</span>
            <span className="badge badge-xs badge-ghost">{info.keyframes.length} 关键帧</span>
            <span className="badge badge-xs badge-ghost">音频 {info.audio}</span>
            <span className="badge badge-xs badge-ghost">
              {(info.streamBytes / (1024 * 1024)).toFixed(1)} MB
            </span>
          </>
        ) : (
          <span className="text-base-content/50">{status || "读取中…"}</span>
        )}
        {status && info && <span className="badge badge-xs badge-warning">{status}</span>}
        {!videoDecodable() && (
          <span className="badge badge-xs badge-warning">WebView 无 WebCodecs，无法出画面</span>
        )}
      </div>

      <div className="flex min-h-0 flex-1 items-center justify-center overflow-hidden bg-base-300/40 p-2">
        <canvas ref={canvasRef} className="max-h-full max-w-full object-contain shadow" />
        {!bitmaps.length && !status && info && (
          <span className="absolute text-xs text-base-content/50">没有解码出画面</span>
        )}
      </div>

      <div className="flex flex-none flex-col gap-1 border-t border-base-300 p-2">
        <div
          className="relative w-full cursor-pointer overflow-hidden rounded border border-base-300 bg-base-100"
          style={{ height: TIMELINE_HEIGHT }}
          onClick={onTimelineClick}
        >
          <canvas ref={timelineRef} className="block h-full w-full" />
          <div
            className="pointer-events-none absolute inset-y-0 w-px bg-error"
            style={{ left: `${progress * 100}%` }}
          />
        </div>

        <div className="flex items-center gap-2">
          <button
            className="btn btn-sm btn-primary"
            disabled={!info}
            onClick={() => setPlaying((value) => !value)}
            title="空格"
          >
            {playing ? "暂停" : "播放"}
          </button>
          <button className="btn btn-sm" disabled={!info} onClick={() => seek(current - 1)}>
            上一帧
          </button>
          <button className="btn btn-sm" disabled={!info} onClick={() => seek(current + 1)}>
            下一帧
          </button>
          <button className="btn btn-sm" disabled={!info} onClick={() => seek(0)}>
            回到开头
          </button>
          <span className="font-mono text-[11px] text-base-content/60">
            {current + 1} / {info?.frames ?? 0} · {formatTime(current / Math.max(1, info?.fps ?? 30))}
          </span>
          {gopIndex >= 0 && (
            <span className="ml-auto font-mono text-[10px] text-base-content/40">
              GOP {gopIndex} / 已缓存 {bitmaps.length} 帧
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00.0";
  const minutes = Math.floor(seconds / 60);
  const rest = seconds - minutes * 60;
  return `${minutes}:${rest.toFixed(1).padStart(4, "0")}`;
}
