import { t } from "../../i18n";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, type FileEntry, type UsmVideo } from "../../lib/ipc";
import { decodeGop, describeError, UnsupportedVideoError, videoDecodable } from "../../lib/videoDecode";
import { createFrameBitmapConverter, isNearBlack } from "../../lib/frameBitmap";
import { gopIndexAt, gopRange, nextGopToLoad, type GopRange } from "../../lib/videoWindow";
import { mergeFrameBatch, type FrameBatchItem } from "../../lib/frameCache";

const TIMELINE_HEIGHT = 56;

const MAX_BITMAP_WIDTH = 1024;

const PREFETCH_AHEAD = 12;

function withDeadline<T>(promise: Promise<T>, ms: number, label: string): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(() => reject(new Error(t("video.timeout", { label, ms }))), ms);
    promise.then(
      (value) => {
        window.clearTimeout(timer);
        resolve(value);
      },
      (error) => {
        window.clearTimeout(timer);
        reject(error);
      },
    );
  });
}

export function VideoViewer({ entry }: { entry: FileEntry }) {
  const [info, setInfo] = useState<UsmVideo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string>(t("video.indexing"));

  const [frames, setFrames] = useState<Map<number, ImageBitmap>>(() => new Map());
  const [loaded, setLoaded] = useState<GopRange | null>(null);
  const [gopStats, setGopStats] = useState<{ gop: number; chunks: number; decoded: number } | null>(
    null,
  );
  const [current, setCurrent] = useState(0);
  const [playing, setPlaying] = useState(false);

  const [hint, setHint] = useState("");

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const timelineRef = useRef<HTMLCanvasElement>(null);

  const framesRef = useRef<Map<number, ImageBitmap>>(new Map());
  const loadingRef = useRef(false);
  const generationRef = useRef(0);

  const framesTotalRef = useRef(0);

  const playingRef = useRef(false);

  const currentRef = useRef(0);

  useEffect(() => {
    let alive = true;
    setInfo(null);
    setError(null);
    setStatus(t("video.indexing"));
    framesRef.current.forEach((bitmap) => bitmap.close());
    framesRef.current = new Map();
    setFrames(framesRef.current);
    setLoaded(null);
    setGopStats(null);
    setHint("");
    setCurrent(0);
    setPlaying(false);

    api
      .usmVideoInfo(entry.path)
      .then((next) => {
        if (!alive) return;
        framesTotalRef.current = next.frames;
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
    (frame: number) => gopIndexAt(info?.keyframes ?? [], frame),
    [info],
  );

  const loadGop = useCallback(
    async (gop: number) => {
      if (!info || loadingRef.current) return;
      loadingRef.current = true;
      const generation = (generationRef.current += 1);
      const mine = () => generation === generationRef.current;
      const report = (text: string) => {
        if (!mine()) return;
        if (playingRef.current) return;
        setStatus(text);
      };

      const range = gopRange(info.keyframes, info.frames, gop);
      const startFrame = range.start;
      const span = range.end - range.start;
      const batch: FrameBatchItem[] = [];
      let handedOver = false;

      report(t("video.fetch"));
      try {
        const bytes = await withDeadline(api.usmVideoGop(entry.path, gop), 15_000, t("video.fetch"));
        if (!mine()) return;
        const targetWidth = Math.min(MAX_BITMAP_WIDTH, info.width) || info.width || 640;
        const targetHeight = Math.max(
          2,
          Math.round((targetWidth * info.height) / Math.max(1, info.width)),
        );
        const toBitmap = createFrameBitmapConverter(targetWidth, targetHeight);
        const result = await decodeGop(
          new Uint8Array(bytes),
          info,
          startFrame,
          (frame) => {
            const index = Math.round((frame.timestamp * info.fps) / 1_000_000);
            batch.push({ index, bitmap: toBitmap(frame) });
          },
          report,
        );
        if (!mine()) return;

        setGopStats({ gop, chunks: result.chunks, decoded: result.decoded });
        const lost =
          result.decoded > batch.length
            ? t("video.bitmapMismatch", { decoded: result.decoded, bitmaps: batch.length })
            : "";
        const keepFrom = Math.max(0, startFrame - span);
        const merged = mergeFrameBatch(framesRef.current, batch, keepFrom);
        framesRef.current = merged.next;
        setFrames(merged.next);
        for (const bitmap of merged.dropped) bitmap.close();
        handedOver = true;
        setLoaded(range);
        setStatus(result.warning ?? lost);
        if (gop === 0) {
          const first = batch.find((item) => item.index === startFrame) ?? batch[0];
          setHint(first && isNearBlack(first.bitmap) ? t("video.firstFrameBlack") : "");
        }
      } catch (e) {
        report(e instanceof UnsupportedVideoError ? e.message : describeError(e));
      } finally {
        if (!handedOver) batch.forEach((item) => item.bitmap.close());
        loadingRef.current = false;
      }
    },
    [entry.path, info],
  );

  useEffect(() => {
    if (!info || loaded) return;
    void loadGop(gopOf(current));
  }, [info, loaded, current, gopOf, loadGop]);

  useEffect(() => {
    if (!info || loadingRef.current) return;
    const target = nextGopToLoad({
      keyframes: info.keyframes,
      total: info.frames,
      current,
      loaded,
      has: (frame) => frames.has(frame),
      prefetchAhead: PREFETCH_AHEAD,
    });
    if (target !== null) void loadGop(target);
  }, [current, frames, info, loaded, loadGop]);

  useEffect(
    () => () => {
      framesRef.current.forEach((bitmap) => bitmap.close());
      framesRef.current = new Map();
    },
    [],
  );

  useEffect(() => {
    playingRef.current = playing;
  }, [playing]);

  useEffect(() => {
    currentRef.current = current;
  }, [current]);

  useEffect(() => {
    if (!playing || !info) return;
    const from = currentRef.current;
    const startedAt = performance.now();
    const fps = Math.max(1, info.fps);
    const interval = window.setInterval(() => {
      const elapsed = (performance.now() - startedAt) / 1000;
      const next = from + Math.floor(elapsed * fps);
      if (next >= framesTotalRef.current) {
        setCurrent(Math.max(0, framesTotalRef.current - 1));
        setPlaying(false);
        return;
      }
      setCurrent(next);
    }, Math.max(8, 500 / fps));
    return () => window.clearInterval(interval);
  }, [playing, info]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const bitmap = frames.get(current);
    if (!bitmap) return;
    const context = canvas.getContext("2d");
    if (!context) return;
    if (canvas.width !== bitmap.width || canvas.height !== bitmap.height) {
      canvas.width = bitmap.width;
      canvas.height = bitmap.height;
    }
    context.drawImage(bitmap, 0, 0);
  }, [frames, current]);

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
            <span className="badge badge-xs badge-ghost">{t("video.frames", { count: info.frames })}</span>
            <span className="badge badge-xs badge-ghost">{t("video.keyframes", { count: info.keyframes.length })}</span>
            <span className="badge badge-xs badge-ghost">{t("video.audio", { audio: info.audio })}</span>
            <span className="badge badge-xs badge-ghost">
              {(info.streamBytes / (1024 * 1024)).toFixed(1)} MB
            </span>
          </>
        ) : (
          <span className="text-base-content/50">{status || t("common.loading")}</span>
        )}
        {status && info && <span className="badge badge-xs badge-warning">{status}</span>}
        {!videoDecodable() && (
          <span className="badge badge-xs badge-warning">{t("video.noWebCodecs")}</span>
        )}
      </div>

      <div className="relative flex min-h-0 flex-1 items-center justify-center overflow-hidden bg-base-300/40 p-2">
        <canvas ref={canvasRef} className="max-h-full max-w-full object-contain shadow" />
        {frames.size === 0 && (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 p-6 text-center">
            <span
              className={`max-w-full text-xs ${status ? "text-warning" : "text-base-content/50"}`}
            >
              {status || (info ? t("video.noPicture") : "")}
            </span>
            {status && info && (
              <button
                className="btn btn-xs"
                onClick={() => {
                  loadingRef.current = false;
                  void loadGop(gopOf(current));
                }}
              >
                {t("common.retry")}
              </button>
            )}
          </div>
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
            title={t("video.space")}
          >
            {playing ? t("common.pause") : t("common.play")}
          </button>
          <button className="btn btn-sm" disabled={!info} onClick={() => seek(current - 1)}>
            {t("video.prevFrame")}
          </button>
          <button className="btn btn-sm" disabled={!info} onClick={() => seek(current + 1)}>
            {t("video.nextFrame")}
          </button>
          <button className="btn btn-sm" disabled={!info} onClick={() => seek(0)}>
            {t("video.toStart")}
          </button>
          <span className="font-mono text-[11px] text-base-content/60">
            {current + 1} / {info?.frames ?? 0} · {formatTime(current / Math.max(1, info?.fps ?? 30))}
          </span>
          {loaded && (
            <span className="ml-auto font-mono text-[10px] text-base-content/40">
              GOP {loaded.gop} · {loaded.start}..{loaded.end - 1}
              {gopStats && gopStats.gop === loaded.gop
                ? t("video.gopStats", { chunks: gopStats.chunks, decoded: gopStats.decoded })
                : ""}
              {t("video.cache", { count: frames.size })}
            </span>
          )}
          {hint && <span className="text-[10px] text-base-content/40">{hint}</span>}
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
